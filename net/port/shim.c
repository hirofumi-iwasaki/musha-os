// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#include "lwip/init.h"
#include "lwip/netif.h"
#include "lwip/etharp.h"
#include "lwip/udp.h"
#include "lwip/dhcp.h"
#include "lwip/timeouts.h"
#include "lwip/stats.h"
#include "netif/ethernet.h"
#include <string.h>
#define FRAMES 8
#define FRAME_BYTES 1514
static struct netif nic;
static struct udp_pcb *echo;
static u8_t queue[FRAMES][FRAME_BYTES];
static u16_t lengths[FRAMES];
static u32_t head,count,now,udp_received,tx_drops,rx_drops;
static u8_t initialized, core_initialized;
u32_t sys_now(void) { return now; }
static err_t output(struct netif *netif, struct pbuf *p) {
    (void)netif;
    if(p->tot_len<14 || p->tot_len>FRAME_BYTES || count==FRAMES) { tx_drops++; return ERR_MEM; }
    u32_t tail=(head+count)%FRAMES;
    if(pbuf_copy_partial(p,queue[tail],p->tot_len,0)!=p->tot_len) { tx_drops++; return ERR_IF; }
    lengths[tail]=p->tot_len;count++;return ERR_OK;
}
static err_t setup(struct netif *netif) {
    netif->name[0]='m';netif->name[1]='u';netif->mtu=1500;
    netif->hwaddr_len=6;netif->flags=NETIF_FLAG_BROADCAST|NETIF_FLAG_ETHARP;
    netif->output=etharp_output;netif->linkoutput=output;return ERR_OK;
}
static void receive(void *arg,struct udp_pcb *pcb,struct pbuf *p,const ip_addr_t *address,u16_t port) {
    (void)arg;
    if(!p) return;
    if(p->tot_len<=1472 && udp_sendto(pcb,p,address,port)==ERR_OK) udp_received++;
    else rx_drops++;
    pbuf_free(p);
}
/* The core/timeouts are boot-owned and initialized exactly once. Endpoint and
 * frame-queue lifetimes are scoped to a stopped NIC session, never DMA buffers. */
void musha_lwip_stop(void) {
    if (initialized) {
        dhcp_stop(&nic);
        dhcp_cleanup(&nic);
    }
    if (echo) {
        udp_remove(echo);
        echo = NULL;
    }
    if (initialized) {
        netif_set_link_down(&nic);
        netif_set_down(&nic); /* Releases ARP entries and queued pbuf references. */
        netif_remove(&nic);
    }
    initialized = 0;
    head = count = 0;
    memset(lengths, 0, sizeof(lengths));
    memset(queue, 0, sizeof(queue));
}
int musha_lwip_init(const u8_t *mac) {
    if (initialized || !mac) return -1;
    if (!core_initialized) {
        lwip_init();
        core_initialized = 1;
    }
    memset(&nic, 0, sizeof(nic));
    ip4_addr_t ip, mask, gw;
    ip4_addr_set_zero(&ip);
    ip4_addr_set_zero(&mask);
    ip4_addr_set_zero(&gw);
    if (!netif_add(&nic,&ip,&mask,&gw,0,setup,ethernet_input)) return -1;
    initialized = 1;
    memcpy(nic.hwaddr,mac,6);
    netif_set_default(&nic);
    netif_set_link_up(&nic);
    netif_set_up(&nic);
    echo = udp_new();
    if (!echo || udp_bind(echo,IP_ADDR_ANY,12345)!=ERR_OK) {
        musha_lwip_stop();
        return -1;
    }
    udp_received = tx_drops = rx_drops = 0;
    udp_recv(echo,receive,0);
    if (dhcp_start(&nic) != ERR_OK) {
        musha_lwip_stop();
        return -1;
    }
    return 0;
}
void musha_lwip_poll(u32_t milliseconds) { now=milliseconds;sys_check_timeouts(); }
int musha_lwip_input(const u8_t *bytes,u16_t length) {
    if(!initialized || length<14 || length>FRAME_BYTES) { rx_drops++;return -1; }
    struct pbuf *p=pbuf_alloc(PBUF_RAW,length,PBUF_POOL);
    if(!p) { rx_drops++;return -1; }
    if(pbuf_take(p,bytes,length)!=ERR_OK || nic.input(p,&nic)!=ERR_OK) { pbuf_free(p);rx_drops++;return -1; }
    return 0;
}
int musha_lwip_tx(u8_t *out,u16_t capacity) {
    if(!count) return 0;
    u16_t length=lengths[head];if(capacity<length) return -1;
    memcpy(out,queue[head],length);head=(head+1)%FRAMES;count--;return length;
}
void musha_lwip_counters(u32_t *out) {
    out[0]=lwip_stats.etharp.recv;out[1]=lwip_stats.icmp.recv;out[2]=udp_received;
    out[3]=tx_drops;out[4]=rx_drops;
}

/* Only a supplied DHCP lease qualifies as an address; no static fallback. */
int musha_lwip_address(u8_t *out) {
    if (!out || !initialized || !dhcp_supplied_address(&nic) ||
        ip4_addr_isany_val(*netif_ip4_addr(&nic))) return 0;
    out[0]=ip4_addr1(netif_ip4_addr(&nic)); out[1]=ip4_addr2(netif_ip4_addr(&nic));
    out[2]=ip4_addr3(netif_ip4_addr(&nic)); out[3]=ip4_addr4(netif_ip4_addr(&nic));
    return 1;
}
u32_t musha_lwip_dhcp_state(void) {
    struct dhcp *d=initialized?netif_dhcp_data(&nic):NULL;
    return d?d->state:0;
}

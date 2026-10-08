// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#include "lwip/init.h"
#include "lwip/netif.h"
#include "lwip/etharp.h"
#include "lwip/udp.h"
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
static u8_t initialized;
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
int musha_lwip_init(const u8_t *mac) {
    if(initialized) return -1;
    lwip_init();
    ip4_addr_t ip,mask,gw;IP4_ADDR(&ip,10,0,2,15);IP4_ADDR(&mask,255,255,255,0);IP4_ADDR(&gw,10,0,2,2);
    if(!netif_add(&nic,&ip,&mask,&gw,0,setup,ethernet_input)) return -1;
    memcpy(nic.hwaddr,mac,6);netif_set_default(&nic);netif_set_link_up(&nic);netif_set_up(&nic);
    echo=udp_new();if(!echo || udp_bind(echo,IP_ADDR_ANY,12345)!=ERR_OK) return -1;
    udp_recv(echo,receive,0);initialized=1;return 0;
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

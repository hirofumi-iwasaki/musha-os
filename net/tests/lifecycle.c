/* SPDX-License-Identifier: Apache-2.0 */
/* Native test of original shim + pinned lwIP, not a simulated network stack. */
#include "lwip/netif.h"
#include "lwip/udp.h"
#include "lwip/pbuf.h"
#include "lwip/stats.h"
#include "lwip/memp.h"
#include "lwip/mem.h"
void musha_lwip_stop(void);
int musha_lwip_init(const u8_t *);
void musha_lwip_poll(u32_t);
int musha_lwip_tx(u8_t *, u16_t);
u32_t sys_now(void);
_Noreturn void musha_lwip_assert(void) { __builtin_trap(); }
#define CHECK(test) do { if (!(test)) return __LINE__; } while (0)
int main(void) {
    const u8_t mac[6] = {2, 0, 0, 0, 0, 1};
    u8_t frame[1514];
    u32_t used[MEMP_MAX];
    u32_t heap = 0;
    musha_lwip_stop(); /* Partial/empty teardown is idempotent. */
    CHECK(musha_lwip_init((void *)0) == -1);
    for (u32_t session = 0; session < 1000; ++session) {
        u32_t before = sys_now();
        CHECK(musha_lwip_init(mac) == 0);
        CHECK(sys_now() == before); /* Endpoint restart never rewinds time. */
        CHECK(musha_lwip_init(mac) == -1); /* Cannot replace a live endpoint. */
        CHECK(netif_list && netif_list == netif_default && !netif_list->next);
        CHECK(udp_pcbs && !udp_pcbs->next);
        struct pbuf *payload = pbuf_alloc(PBUF_TRANSPORT, 17, PBUF_POOL);
        CHECK(payload);
        ip_addr_t destination;
        IP_ADDR4(&destination,10,0,2,2);
        CHECK(udp_sendto(udp_pcbs,payload,&destination,43210) == ERR_OK);
        pbuf_free(payload); /* ARP retains its own queued reference. */
        musha_lwip_poll(1000 + session * 10);
        musha_lwip_stop();
        musha_lwip_stop();
        CHECK(!netif_list && !netif_default && !udp_pcbs);
        CHECK(musha_lwip_tx(frame,sizeof(frame)) == 0); /* No old TX queue. */
        if (!session) {
            for (unsigned i = 0; i < MEMP_MAX; ++i) used[i] = lwip_stats.memp[i]->used;
            heap = lwip_stats.mem.used;
        } else {
            for (unsigned i = 0; i < MEMP_MAX; ++i) CHECK(used[i] == lwip_stats.memp[i]->used);
            CHECK(heap == lwip_stats.mem.used); /* Includes queued ARP pbufs. */
        }
    }
    return 0;
}

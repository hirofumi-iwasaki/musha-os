// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#ifndef MUSHA_LWIPOPTS_H
#define MUSHA_LWIPOPTS_H
#define NO_SYS 1
#define SYS_LIGHTWEIGHT_PROT 0
#define MEM_ALIGNMENT 8
#define MEM_SIZE 32768
#define MEM_LIBC_MALLOC 0
#define MEMP_MEM_MALLOC 0
#define PBUF_POOL_SIZE 16
#define PBUF_POOL_BUFSIZE 1536
#define MEMP_NUM_UDP_PCB 2
#define MEMP_NUM_SYS_TIMEOUT 4
#define LWIP_IPV4 1
#define LWIP_IPV6 0
#define LWIP_ARP 1
#define LWIP_ICMP 1
#define LWIP_UDP 1
#define LWIP_TCP 0
#define LWIP_RAW 0
#define LWIP_DHCP 0
#define LWIP_AUTOIP 0
#define LWIP_DNS 0
#define LWIP_IGMP 0
#define IP_REASSEMBLY 0
#define IP_FRAG 0
#define LWIP_NETCONN 0
#define LWIP_SOCKET 0
#define LWIP_NETIF_LOOPBACK 0
#define LWIP_HAVE_LOOPIF 0
#define LWIP_STATS 1
#define LINK_STATS 1
#define ETHARP_STATS 1
#define ICMP_STATS 1
#define UDP_STATS 1
#define IP_STATS 1
#define MIB2_STATS 0
#define LWIP_NETIF_HOSTNAME 0
#define LWIP_CHECKSUM_CTRL_PER_NETIF 0
#define CHECKSUM_GEN_IP 1
#define CHECKSUM_GEN_UDP 1
#define CHECKSUM_GEN_ICMP 1
#define CHECKSUM_CHECK_IP 1
#define CHECKSUM_CHECK_UDP 1
#define CHECKSUM_CHECK_ICMP 1
#endif

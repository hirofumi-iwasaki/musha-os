/* SPDX-License-Identifier: Apache-2.0 */
#include <musha/api.h>
#include "syscall.h"
static void put(const char *s) { uint64_t n=0,w=0; while(s[n]) n++; musha_log(s,n,&w); }
static void number(const char *label,uint64_t value) {
    char b[]="0x0000000000000000\n";put(label);
    for(unsigned i=0;i<16;i++)b[17-i]="0123456789ABCDEF"[(value>>(i*4))&15];
    put(b);
}
#ifndef IO_CASE
#define IO_CASE 0
#endif
int musha_main(const musha_start_info_v1 *start) {
    uint64_t features=0,t0=0,t1=0,reason=99;
    if (!start || start->abi_minor!=2 || musha_query(&features) || features!=3) return 1;
#if IO_CASE == 1
    musha_raw_result denied;
    for(uint64_t call=MUSHA_SYS_TIME;call<=MUSHA_SYS_INPUT;call++) {
        denied=musha_raw(call,0,0,0,0,0,0);
        if(denied.status!=MUSHA_ERR_DENIED || denied.value)return 20;
    }
    put("R2 PERMISSIONS OK\n");return 0;
#elif IO_CASE == 5
    if(musha_time(&t0) || musha_wait(0,0,&reason)!=MUSHA_ERR_DENIED || musha_input((musha_input_event *)(uintptr_t)start->arena_base)!=MUSHA_ERR_DENIED)return 24;
    put("R2 PERMISSIONS OK\n");return 0;
#elif IO_CASE == 6
    if(musha_wait(0,0,&reason) || reason || musha_time(&t0)!=MUSHA_ERR_DENIED || musha_input((musha_input_event *)(uintptr_t)start->arena_base)!=MUSHA_ERR_DENIED || musha_wait(0,MUSHA_WAIT_INPUT,&reason)!=MUSHA_ERR_DENIED)return 25;
    put("R2 PERMISSIONS OK\n");return 0;
#endif
    if (musha_time(&t0) || musha_wait(0,0,&reason) || reason || musha_wait(20,0,&reason) || reason || musha_time(&t1) || t1<t0+20) return 2;
    put("R2 C TIME WAIT OK\n");
    musha_raw_result bad=musha_raw(MUSHA_SYS_INPUT,0,48,0,0,0,0);
    if (bad.status!=MUSHA_ERR_INVALID || bad.value) return 3;
    bad=musha_raw(MUSHA_SYS_INPUT,(uint64_t)start,48,0,0,0,0);
    if (bad.status!=MUSHA_ERR_INVALID || bad.value) return 4;
    bad=musha_raw(MUSHA_SYS_INPUT,start->arena_base,47,0,0,0,0);
    if (bad.status!=MUSHA_ERR_LIMIT || bad.value) return 5;
    bad=musha_raw(MUSHA_SYS_INPUT,UINT64_MAX-4,48,0,0,0,0);
    if (bad.status!=MUSHA_ERR_INVALID || bad.value) return 6;
    bad=musha_raw(MUSHA_SYS_INPUT,start->arena_base+start->arena_bytes-24,48,0,0,0,0);
    if(bad.status!=MUSHA_ERR_INVALID || bad.value)return 10;
    if (musha_wait(60001,0,&reason)!=MUSHA_ERR_LIMIT) return 7;
    put("R2 INVALID BUFFERS OK\n");
    musha_input_event e;
#if IO_CASE == 2
    put("R2 FAULT READY\n");__asm__ volatile("ud2");return 21;
#elif IO_CASE == 3
    put("R2 OVERFLOW READY\n");
    if(musha_wait(2500,0,&reason))return 22;
    int overflow=0;while(musha_input(&e)==0) {
        if(e.kind==MUSHA_INPUT_OVERFLOW && e.buttons>0)overflow=1;
    }
    if(!overflow)return 23;
    put("R2 OVERFLOW OK\n");return 0;
#endif
    while(musha_input(&e)==0) {}
    put("R2 INPUT READY\n");
    int q_down=0,mouse_down=0;
    (void)mouse_down;
    for (;;) {
        musha_status wake=musha_wait(5000,MUSHA_WAIT_INPUT,&reason);
#if IO_CASE == 4
        if(wake==MUSHA_ERR_DISCONNECTED){put("R2 DISCONNECTED OK\n");return 0;}
#endif
        if(wake || reason!=MUSHA_WAKE_INPUT)return 8;
        musha_status status;
        while ((status=musha_input(&e))==0) {
            if (e.kind==MUSHA_INPUT_KEY) {
                put(e.flags ? "R2 KEY DOWN\n" : "R2 KEY UP\n");
                if(e.code==0x14) { if(e.flags)q_down=1; else if(q_down){put("R2 Q EXIT\n");return 0;} }
            } else if (e.kind==MUSHA_INPUT_MOUSE) {
                put("R2 MOUSE\n");number("DX ",(uint64_t)(int64_t)e.value0);number("DY ",(uint64_t)(int64_t)e.value1);
                number("WHEEL ",(uint64_t)(int64_t)e.value2);number("BUTTONS ",e.buttons);
#if IO_CASE == 7
                if(e.buttons&1)mouse_down=1;
                else if(mouse_down){put("R2 MOUSE EXIT\n");return 0;}
#endif
            }
        }
        if(status!=MUSHA_ERR_AGAIN)return 9;
    }
}

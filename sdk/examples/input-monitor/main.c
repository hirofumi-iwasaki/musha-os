/* Copyright 2026 Hirofumi Iwasaki; SPDX-License-Identifier: Apache-2.0 */
/* Generic live r2 observer: no fixed fixture sequence or device requirement. */
#include <musha/api.h>
static int put(const char *s, uint64_t n) {
    uint64_t written=0;
    return musha_log(s,n,&written) || written!=n;
}
static void text(char *line, unsigned *n, const char *s) {
    while (*s && *n<239) line[(*n)++]=*s++;
}
static void hex(char *line, unsigned *n, uint64_t v, unsigned digits) {
    for (unsigned i=digits;i>0 && *n<239;i--) line[(*n)++]="0123456789ABCDEF"[(v>>((i-1)*4))&15];
}
static int show(const musha_input_event *e) {
    char b[240];unsigned n=0;
    text(b,&n,"LIVE kind=");hex(b,&n,e->kind,4);
    text(b,&n," dev=");hex(b,&n,e->device,8);
    text(b,&n," code=");hex(b,&n,e->code,4);
    text(b,&n," flags=");hex(b,&n,e->flags,8);
    text(b,&n," value=");hex(b,&n,(uint32_t)e->value0,8);
    text(b,&n,",");hex(b,&n,(uint32_t)e->value1,8);
    text(b,&n,",");hex(b,&n,(uint32_t)e->value2,8);
    text(b,&n," buttons=");hex(b,&n,e->buttons,8);
    text(b,&n," mods=");hex(b,&n,e->modifiers,4);
    text(b,&n," ms=");hex(b,&n,e->timestamp_ms,16);
    b[n++]='\n';return put(b,n);
}
int musha_main(const musha_start_info_v1 *start) {
    uint64_t began=0,now=0,reason=0;
    if (!start || start->abi_minor!=2 || musha_time(&began)) return 1;
    static const char ready[]="LIVE INPUT READY; Q/Esc exits; five-minute limit. Values are hexadecimal.\n";
    if(put(ready,sizeof(ready)-1))return 2;
    for(;;) {
        for(unsigned count=0;count<64;count++) {
            musha_input_event e;
            musha_status status=musha_input(&e);
            if(status==MUSHA_ERR_AGAIN)break;
            if(status==MUSHA_ERR_DISCONNECTED)return 0;
            if(status || show(&e))return 3;
            if(e.kind==MUSHA_INPUT_KEY && e.flags==1 && (e.code==0x14 || e.code==0x29)) {
                static const char bye[]="LIVE INPUT EXIT\n";
                return put(bye,sizeof(bye)-1)?4:0;
            }
        }
        if(musha_time(&now) || now<began)return 5;
        if(now-began>=300000)return 0;
        musha_status status=musha_wait(100,MUSHA_WAIT_INPUT,&reason);
        if(status==MUSHA_ERR_DISCONNECTED)return 0;
        if(status)return 6;
    }
}

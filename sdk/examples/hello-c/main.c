/* SPDX-License-Identifier: Apache-2.0 */
#include <musha/api.h>
static volatile unsigned char zero_bss[16];
int musha_main(const musha_start_info_v1 *info){uint64_t features=0,written=0;if(!info||info->struct_size!=MUSHA_START_SIZE||zero_bss[0])return 1;if(musha_query(&features))return 2;return musha_log("Hello Musha-OS!\n",16,&written)!=0;}

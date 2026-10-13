/* SPDX-License-Identifier: Apache-2.0 */
.section .text.start,"ax",@progbits
.global _start
.type _start,@function
_start:
 /* Entry is a jump with RSP % 16 == 0, RDI=start info, no return address. */
 cld
 xor %ebp,%ebp
 call musha_main
 mov %eax,%edi
 call musha_exit
 ud2
.section .note.GNU-stack,"",@progbits

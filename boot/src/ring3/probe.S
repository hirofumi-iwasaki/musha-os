// SPDX-License-Identifier: Apache-2.0
// Relocatable, bounded fixture copied into its own RX user page. Only local
// branches/RIP-relative data and explicit candidate VAs; no kernel relocations.
.text
.p2align 4
.globl musha_probe_start
.globl musha_probe_end
musha_probe_start:
    mov ax, cs
    and eax, 3
    cmp eax, 3
    jne 90f
    test rsp, 15
    jnz 90f
    sub rsp, 32
    mov [rsp + 16], rdi
    mov [rsp + 24], rsi
    mov ebx, 0x123
    mov esi, 0x234
    mov edi, 0x345
    mov ebp, 0x456
    mov r8d, 0x567
    mov r9d, 0x678
    mov r10d, 0x789
    mov r12d, 0x89a
    mov r13d, 0x9ab
    mov r14d, 0xabc
    mov r15d, 0xbcd
    mov eax, 0x12345678
    movq xmm0, rax
    movq xmm15, rax
    fld1
    std
    mov eax, {query}
    syscall
    test rax, rax
    jnz 90f
    cmp rdx, 1
    jne 90f
    cmp ebx, 0x123
    jne 90f
    cmp esi, 0x234
    jne 90f
    cmp edi, 0x345
    jne 90f
    cmp ebp, 0x456
    jne 90f
    cmp r8d, 0x567
    jne 90f
    cmp r9d, 0x678
    jne 90f
    cmp r10d, 0x789
    jne 90f
    cmp r12d, 0x89a
    jne 90f
    cmp r13d, 0x9ab
    jne 90f
    cmp r14d, 0xabc
    jne 90f
    cmp r15d, 0xbcd
    jne 90f
    pushfq
    pop rax
    test eax, 0x47700 // TF, IF, DF, IOPL, NT, AC must be clear.
    jnz 90f
    movq rax, xmm0
    cmp rax, 0x12345678
    jne 90f
    movq rax, xmm15
    cmp rax, 0x12345678
    jne 90f
    fstp qword ptr [rsp]
    mov rax, 0x3ff0000000000000
    cmp [rsp], rax
    jne 90f
    stmxcsr [rsp]
    cmp dword ptr [rsp], 0x1f80
    jne 90f
    // Unknown number must return sign-extended status and zero secondary value.
    mov rax, -1
    mov edx, 123
    syscall
    cmp rax, -1
    jne 90f
    test rdx, rdx
    jnz 90f
    // A wrapping/noncanonical pointer must be rejected without dereferencing it.
    mov eax, {log}
    mov rdi, -1
    mov esi, 2
    syscall
    cmp rax, -3
    jne 90f
    test rdx, rdx
    jnz 90f
    mov eax, {log}
    mov esi, 4097
    syscall
    cmp rax, -5
    jne 90f
    test rdx, rdx
    jnz 90f
    mov eax, {log}
    lea rdi, [rip + 80f]
    mov esi, 17
    syscall
    test rax, rax
    jnz 90f
    cmp rdx, 17
    jne 90f
    mov rax, [rsp + 16]
    cmp eax, 1
    je 81f
    cmp eax, 2
    je 82f
    cmp eax, 3
    je 83f
    cmp eax, 4
    je 84f
    cmp eax, 5
    je 85f
    cmp eax, 6
    je 86f
    xor edi, edi
    mov eax, {exit}
    syscall
    ud2
80: .ascii "RING3 USER HELLO\n"
81: mov byte ptr [{code}], 0
    jmp 90f
82: mov byte ptr [rsp], 0xc3
    jmp rsp
83: mov rax, [{stack} - 8]
    jmp 90f
84: mov rax, [rsp + 24]
    mov rax, [rax]
    jmp 90f
85: mov rsp, 0x800000000000
    mov eax, {query}
    syscall
    ud2
86: cli
    jmp 90f
90: cld
    mov edi, 1
    mov eax, {exit}
    syscall
    ud2
musha_probe_end:

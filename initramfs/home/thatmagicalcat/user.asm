; user.asm
bits 64

%define SYS_WRITE 1
%define SYS_EXIT 0
%define STDOUT 1

section .rodata
    msg db "Hello from the ELF userspace process!", 0xD, 0xA ; 0xD - \r, 0xA - \n
    msg_len equ $ - msg

section .text
    global _start

_start:
    mov rax, SYS_WRITE
    mov rdi, STDOUT
    mov rsi, msg
    mov rdx, msg_len
    syscall

    mov rax, SYS_EXIT
    syscall

.halt:
    jmp .halt

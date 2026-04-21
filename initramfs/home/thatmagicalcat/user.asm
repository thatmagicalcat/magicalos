bits 64

%define SYS_EXIT 0
%define SYS_READ 1
%define SYS_WRITE 2
%define SYS_MMAP 3
%define SYS_ARCH_PRCTL 4

%define MAP_ANONYMOUS 0x20
%define MAP_PRIVATE 0x02

%define PROT_READ 0x1
%define PROT_WRITE 0x2

section .rodata
    msg db "Hello, World!", 0xD, 0xA
    msg_len equ $ - msg

section .text
global _start

_start:
    mov rax, SYS_WRITE
    mov rdi, 1 ; STDOUT
    lea rsi, [rel msg]
    mov rdx, msg_len
    syscall

    mov rax, 0 ; SYS_EXIT
    syscall


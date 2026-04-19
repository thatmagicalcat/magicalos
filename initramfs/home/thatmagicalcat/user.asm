bits 64

%define MAP_ANONYMOUS 0x20
%define MAP_PRIVATE 0x02

%define PROT_READ 0x1
%define PROT_WRITE 0x2

section .text
global _start

_start:
    mov rax, 2                           ; SYS_MMAP
    mov rdi, 0                           ; addr
    mov rsi, 0x1000                      ; length
    mov rdx, PROT_READ                   ; prot
    mov r10, MAP_PRIVATE | MAP_ANONYMOUS ; flags
    syscall

    mov qword [rax], 42 ; write

    mov rax, 0 ; SYS_EXIT
    syscall


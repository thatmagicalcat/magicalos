ALIGN_   equ 1 << 0
MEMINFO  equ 1 << 1
FLAGS    equ ALIGN_ | MEMINFO
MAGIC    equ 0x1BADB002
CHECKSUM equ -(MAGIC + FLAGS)

section .multiboot
align 4
dd MAGIC
dd FLAGS
dd CHECKSUM

section .bss
align 16
stack_bottom:
resb 16384 ; 16 KiB
stack_top:

section .text
global _start
extern kernel_main

_start:
    ; setup a stack
    mov esp, stack_top

    ; initialize crucial processor state here

    ; enter the high-level kernel
    call kernel_main
    cli

.hang:
    hlt
    jmp .hang

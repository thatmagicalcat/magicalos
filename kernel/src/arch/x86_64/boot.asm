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

section .rodata
gdt64:
    dq 0
.code: equ $ - gdt64
    dq (1 << 44) | (1 << 47) | (1 << 41) | (1 << 43) | (1 << 53)
.data: equ $ - gdt64
    dq (1 << 44) | (1 << 47) | (1 << 41)
.pointer:
    dw .pointer - gdt64 - 1
    dq gdt64

; I'll be using 2 MiB pages so I only need 3 tables
section .bss
align 4096
p4_tbl:
    resb 4096
p3_tbl:
    resb 4096
p2_tbl:
    resb 4096

align 16
stack_bottom:
resb 16384 ; 16 KiB
stack_top:

section .text
bits 32
global _start
extern kernel_main

_start:
    ; setup a stack
    mov esp, stack_top
    mov edi, ebx ; save the multiboot info pointer in edi

    ; setup paging
    ; bit range
    ; 0 - present (must be 1)
    ; 1 - R/W (0 - R, 1 - RW)
    ; 2 - U/S (0 - Kernel space, 1 - user space)
    ; 3 - pwt (page level write through, usually 0)
    ; 4 - pcd (page level cache disable, usually 0)
    ; 7 - page size (0 - 4 KiB, 1 - 2 MiB)
    ; the rest of the bits are the physical address of the next level page table

    ;; P4 table
    mov eax, p3_tbl
    or eax, 0b11 ; present and R/W
    ; set the first entry of the P4 table to point to the P3 table
    mov dword [p4_tbl + 0], eax

    ;; P3 table
    mov eax, p2_tbl
    or eax, 0b11 ; present and R/W
    ; set the first entry of the P3 table to point to the P2 table
    mov dword [p3_tbl + 0], eax

    ;; P2 table
    ; point each P2 table entry to a 2 MiB page
    mov ecx, 0 ; counter variable
.map_p2_tbl:
    mov eax, 0x200000 ; 2 MiB
    mul ecx ; eax = 2 MiB * ecx
    or eax, 0b10000011 ; present, R/W, page size
    mov [p2_tbl + ecx * 8], eax

    inc ecx,
    cmp ecx, 512 ; there are 4096 / 8 = 512 entries in a page table
    jne .map_p2_tbl

    ;; ENABLE PAGING!!!!!!!!!!
    mov eax, p4_tbl
    mov cr3, eax    ; load the address of P4 table into cr3 register

    ; enable PAE (page address extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit
    mov ecx, 0xC0000080 ; IA32_EFER MSR
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; we're ready to enable paging!
    mov eax, cr0
    or eax, 1 << 31
    or eax, 1 << 16
    mov cr0, eax

    ; load GDT
    lgdt [gdt64.pointer]

    ; update sectors
    mov ax, gdt64.data
    mov ss, ax
    mov ds, ax
    mov es, ax

    ; the long leap of faith!
    jmp gdt64.code:long_mode_start

section .text
bits 64
long_mode_start:
    call kernel_main
    hlt

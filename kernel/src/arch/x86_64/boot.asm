section .multiboot
header_start:
    dd 0xe85250d6                ; magic
    dd 0
    dd header_end - header_start ; header len
    ; checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

    ; section 3.1.10 of https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
    ; framebuffer tag
    align 8
    dw 5  ; type
    dw 0  ; flags
    dd 20 ; size
    dd 0  ; width (0 = no preference)
    dd 0  ; height (0 = no preference)
    dd 32 ; depth (bits per pixel)

    ; terminator tag
    align 8
    dw 0 ; type
    dw 0 ; flags
    dw 8 ; size
header_end:

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

section .bss
align 4096
p4_tbl:
    resb 4096
p3_tbl:
    resb 4096
p2_tbl:
    resb 4096
p1_tbl:
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

    mov edi, ebx ; save the multiboot info pointer for later use

    call check_multiboot
    call check_cpuid
    call check_long_mode
    call setup_page_tbl
    call enable_paging

    lgdt [gdt64.pointer]

    ; update sectors
    mov ax, gdt64.data
    mov ss, ax
    mov ds, ax
    mov es, ax

    ; LEAP OF FAITH!
    jmp gdt64.code:long_mode_start

setup_page_tbl:
    ; bit range
    ; bit - name: meaning
    ; 0 - present: the page is currently in memory
    ; 1 - writable: it’s allowed to write to this page
    ; 2 - user accessible: if not set, only kernel mode code can access this page
    ; 3 - write through caching: writes go directly to memory
    ; 4 - disable cache: no cache is used for this page
    ; 5 - accessed: the CPU sets this bit when this page is used
    ; 6 - dirty: the CPU sets this bit when a write to this page occurs
    ; 7 - huge page/null: must be 0 in P1 and P4, creates a 1GiB page in P3, creates a 2MiB page in P2
    ; 8 - global: page isn’t flushed from caches on address space switch (PGE bit of CR4 register must be set)
    ; 9-11 - available: can be used freely by the OS
    ; 52-62 - available: can be used freely by the OS
    ; 63 - no execute: forbid executing code on this page (the NXE bit in the EFER register must be set)

    ; recursively map the 511th entry of P4 to itself
    mov eax, p4_tbl
    or eax, 0b11 ; present + writable
    mov dword [p4_tbl + 511 * 8], eax
    mov dword [p4_tbl + 511 * 8 + 4], 0

    ;; P4 table
    mov eax, p3_tbl
    or eax, 0b11 ; present and R/W
    ; set the first entry of the P4 table to point to the P3 table
    mov dword [p4_tbl + 0], eax
    mov dword [p4_tbl + 4], 0

    ;; P3 table
    mov eax, p2_tbl
    or eax, 0b11 ; present and R/W
    ; set the first entry of the P3 table to point to the P2 table
    mov dword [p3_tbl + 0], eax
    mov dword [p3_tbl + 4], 0

    ;; P2 table
    mov eax, p1_tbl
    or eax, 0b11
    ;set the first entry of the P2 table to point to the P1 table
    mov dword [p2_tbl + 0], eax
    mov dword [p2_tbl + 4], 0

    ; point each P2 table entry to a 2 MiB page
;     mov ecx, 0 ; counter variable
; .map_p2_tbl:
;     mov eax, 0x200000  ; 2 MiB
;     mul ecx            ; eax = 2 MiB * ecx
;     or eax, 0b10000011 ; present, R/W, page size
;     mov [p2_tbl + ecx * 8], eax
;     mov [p2_tbl + ecx * 8 + 4], 0
;
;     inc ecx,
;     cmp ecx, 512 ; there are 4096 / 8 = 512 entries in a page table
;     jne .map_p2_tbl

   ;; map the rest of P2 as 2 MiB huge pages
    mov ecx, 1 ; Start at 1 because entry 0 is the P1 table
.map_p2_tbl:
    mov eax, 0x200000 ; 2 MiB
    mul ecx
    or eax, 0b10000011 ; present, writable, huge page
    mov [p2_tbl + ecx * 8], eax
    inc ecx
    cmp ecx, 512
    jne .map_p2_tbl

    ;; map P1 table (first 2 MiB as 4 KiB pages)
    mov ecx, 1 ; start at 1 to leave 0x0 unmapped!
.map_p1_tbl:
    mov eax, 0x1000 ; 4 KiB
    mul ecx         ; eax = 4 KiB * ecx
    or eax, 0b11    ; present, writable
    mov dword [p1_tbl + ecx * 8], eax
    mov dword [p1_tbl + ecx * 8 + 4], 0

    inc ecx
    cmp ecx, 512
    jne .map_p1_tbl

    ret

enable_paging:
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
    or eax, 1 << 11 ; NXE bit
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    or eax, 1 << 16 ; write protect bit
    mov cr0, eax

    ret

check_cpuid:
    ; check if the processor supports CPUID instruction
    ; CPUID instruction is supported if bit 21 of FLAGS register is modifiable

    ; copy FLAGS into EAX via stack
    pushfd
    pop eax
    mov ecx, eax ; for comparison later on
    xor eax, 1 << 21 ; try to flip bit 21

    ; copy EAX to FLAGS via stack
    push eax
    popfd

    ; copy FLAGS into EAX again, to check if bit 21 was flipped
    pushfd
    pop eax

    ; restore the original FLAGS
    push ecx
    popfd

    ; if they're equal that means the bit 21 wasn't flipped
    ; and our CPU doesn't support CPUID
    cmp eax, ecx
    je .no_cpuid

    ret
.no_cpuid:
    mov al, '1'
    jmp error

; source: https://en.wikipedia.org/wiki/CPUID
check_long_mode:
    ; check if long mode is supported by the CPU
    mov eax, 1 << 31    ; get the maximum supported extended function
    cpuid
    cmp eax, 1 << 31 | 1
    jb .no_long_mode

    ; use extended info to tets if long mode is available
    mov eax, 0x80000001 ; argument for extended processor info
    cpuid
    test edx, 1 << 29   ; bit 29 of edx indicates if long mode is supported
    jz .no_long_mode

    ret

.no_long_mode:
    mov al, '2'
    jmp error

check_multiboot:
    ; according to multiboot spec the bootloader
    ; must write the following magic value to eax
    ; before loading the kernel
    cmp eax, 0x36d76289
    jne .no_multiboot

    ret
.no_multiboot:
    mov al, '0'
    jmp error

; prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al

    hlt

section .text
bits 64
long_mode_start:
    call kernel_main

    hlt

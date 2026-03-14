ENTRY(_start)

SECTIONS {
    . = 2M;
    kernel_start = .;

    .text : ALIGN(4K) {
        KEEP(*(.multiboot))
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    .bss : ALIGN(4K) {
        *(COMMON)
        *(.bss .bss.*)
    }

    kernel_end = .;
}

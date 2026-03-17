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

    .got : ALIGN(4K) {
        *(.got .got.*)
    }

    .eh_frame : ALIGN(4K) {
        *(.eh_frame .eh_frame.*)
    }

    .eh_frame_hdr : ALIGN(4K) {
        *(.eh_frame_hdr .eh_frame_hdr.*)
    }

    kernel_end = .;
}

OUTPUT_FORMAT("{{OUTPUT_FORMAT}}")
OUTPUT_ARCH({{OUTPUT_ARCH}})
ENTRY(_start)
SECTIONS {
    __lma = {{PHYS_BASE}};
    . = {{PHYS_BASE}};
    .text.boot : AT(__lma) { KEEP(*(.text.boot)) }
    __lma = __lma + SIZEOF(.text.boot);
    .data.boot : AT(__lma) { *(.data.boot) }
    __lma = __lma + SIZEOF(.data.boot);
    . = ALIGN({{PAGE_SIZE}});
    .bss.boot (NOLOAD) : { *(.bss.boot) }
    __lma = ALIGN(ADDR(.bss.boot) + SIZEOF(.bss.boot), 0x200000);
    __lma_stack = {{PHYS_BASE}} + 0x01000000;
    . = __lma_stack;
    .boot_stack (NOLOAD) : AT(__lma_stack) {
        . = ALIGN(16);
        __boot_stack_bottom = .;
        . += 0x8000;
        __boot_stack_top = .;
    }
    . = __lma;
    .text : AT(__lma) {
        __text_vma_start = .;
        *(.text) *(.text.*)
        . = ALIGN(8);
        __text_vma_end = .;
    }
    __text_lma_start = LOADADDR(.text);
    __lma = __lma + SIZEOF(.text);
    .rodata : AT(__lma) {
        __rodata_vma_start = .;
        *(.rodata) *(.rodata.*)
        . = ALIGN(8);
        __rodata_vma_end = .;
    }
    __rodata_lma_start = LOADADDR(.rodata);
    __lma = __lma + SIZEOF(.rodata);
    .data : AT(__lma) {
        __data_vma_start = .;
        *(.data) *(.data.*)
        . = ALIGN(8);
        __data_vma_end = .;
    }
    __data_lma_start = LOADADDR(.data);
    __lma = __lma + SIZEOF(.data);
    .bss : {
        __bss_vma_start = .;
        *(.bss) *(.bss.*) *(COMMON)
        . = ALIGN(8);
        __bss_vma_end = .;
    }
    __bss_lma_start = LOADADDR(.data) + SIZEOF(.data);
    __bss_start = __bss_vma_start;
    __bss_end = __bss_vma_end;
    .stack (NOLOAD) : {
        . = ALIGN(16);
        __stack_bottom = .;
        . += {{STACK_SIZE}};
        __stack_top = .;
    }
    . = ALIGN({{PAGE_SIZE}});
    __kernel_end = .;
    /DISCARD/ : { *(.comment) *(.note.*) *(.eh_frame) *(.eh_frame_hdr) }
}

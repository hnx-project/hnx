/* 链接脚本模板 - 精简版 v0.0.1 */
OUTPUT_FORMAT("{{OUTPUT_FORMAT}}")
OUTPUT_ARCH({{OUTPUT_ARCH}})
ENTRY(_start)

/* 废弃：不再需要复杂的LMA/VMA分离 */
/* 使用简单的线性映射 */

SECTIONS {
    /* 内核加载地址（物理地址） */
    . = {{PHYS_BASE}};
    
    /* 启动代码段（必需） */
    .text.boot : {
        KEEP(*(.text.boot))
    }
    
    /* 主要代码段 */
    .text : {
        __text_start = .;
        *(.text .text.*)
        . = ALIGN(8);
        __text_end = .;
    }
    
    /* 只读数据 */
    .rodata : {
        __rodata_start = .;
        *(.rodata .rodata.*)
        . = ALIGN(8);
        __rodata_end = .;
    }
    
    /* 数据段 */
    .data : {
        __data_start = .;
        *(.data .data.*)
        . = ALIGN(8);
        __data_end = .;
    }
    
    /* BSS段（清零区域） */
    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
    }
    
    /* 废弃：.data.boot 和 .bss.boot 段 */
    /* 原版：.data.boot : AT(__lma) { *(.data.boot) } */
    /* 原版：.bss.boot (NOLOAD) : { *(.bss.boot) } */
    
    /* 启动栈（8KB） */
    . = ALIGN(16);
    .boot_stack (NOLOAD) : {
        __boot_stack_bottom = .;
        . += 0x2000;  /* 8KB */
        __boot_stack_top = .;
    }
    
    /* 主内核栈（可选，可在Rust中动态分配） */
    . = ALIGN(16);
    .stack (NOLOAD) : {
        __stack_bottom = .;
        . += {{STACK_SIZE}};
        __stack_top = .;
    }
    
    /* 内核结束标记 */
    . = ALIGN({{PAGE_SIZE}});
    __kernel_end = .;
    
    /* 废弃：复杂的LMA计算 */
    /* 原版：
    __lma = {{PHYS_BASE}};
    __lma = __lma + SIZEOF(...);
    __lma_stack = {{PHYS_BASE}} + 0x01000000;
    __text_lma_start = LOADADDR(.text);
    __text_vma_start = .;  <- 高地址偏移
    */
    
    /* 废弃：高地址相关符号 */
    /* 原版：
    __text_vma_start = .;  <- 这些用于高半内核
    __text_vma_end = .;
    */
    
    /* 废弃：EXEC_STAGE区域（进程切换相关） */
    
    /* 丢弃无用段 */
    /DISCARD/ : {
        *(.comment)
        *(.note.*)
        *(.eh_frame)
        *(.eh_frame_hdr)
        /* 废弃：原版的调试段 */
        *(.debug_*)
    }
}
/* aarch64 启动汇编代码 */
.section .text.boot

.section .bss.boot
.balign 8
.global __text_lma_value
__text_lma_value:
.quad 0
.section .text.boot

.global _start
_start:
    mrs x0, CurrentEL
    cmp x0, #0x8
    b.ne 0f
    msr sctlr_el2, xzr
    msr cptr_el2, xzr
    isb
    mov x1, #(1 << 31)
    msr hcr_el2, x1
    isb
    mov x1, #0x3C4
    adrp x2, _start_el1
    add  x2, x2, :lo12:_start_el1
    msr spsr_el2, x1
    msr elr_el2, x2
    isb
    eret
0:
    b _start_el1

.global _start_el1
_start_el1:
    /* 1. 使用物理启动栈 */
    ldr x0, =__boot_stack_top
    and x0, x0, #0xFFFFFFFFFFFFFFF0
    mov sp, x0
    
    /* 1.5 清零 BSS 段 - 暂时禁用，以免影响 .data.boot 中的页表 */
    /* .data.boot 在 0x40081000-0x40085000 */
    /* __bss_start 在 0x40317950，不应该覆盖 .data.boot */
    /* 但为了安全起见，暂时禁用 BSS 清零 */
1:
    
    /* 2. 设置异常向量表（不修改 SPSR，不进行 eret） */
    ldr x0, =exception_vector_table
    msr vbar_el1, x0
    isb
    
    mrs x0, cpacr_el1
    orr x0, x0, #(3 << 20)
    msr cpacr_el1, x0
    isb
    
    bl early_uart_init
    ldr x0, =early_msg
    bl early_uart_print
    
    
    /* 启用 MMU（最小流程）后跳转到 Rust */
    ldr x2, =__text_lma_start
    ldr x3, =__text_lma_value
    str x2, [x3]
    bl mmu_enable_boot
    /* 跳转到高地址入口 _start_high */
    adrp x0, _start_high
    add  x0, x0, :lo12:_start_high      /* 低地址 VA 的 _start_high */
    adrp x1, __text_vma_start
    add  x1, x1, :lo12:__text_vma_start /* 低地址 VA 的 .text 起始 */
    sub  x0, x0, x1                     /* 计算偏移 */
    ldr  x2, =0xFFFF800000000000        /* KERNEL_BASE */
    add  x0, x2, x0                     /* 高地址 VA = KERNEL_BASE + 偏移 */
    br   x0

.section .text
.global _start_high
_start_high:
    /* 现在在高地址空间执行，设置栈并输出 POST */
    ldr x0, =__stack_top
    and x0, x0, #0xFFFFFFFFFFFFFFF0
    mov sp, x0
    
    /* CRITICAL: Initialize L1_TABLE[256] to point to L2_TABLE1 
     * This MUST be done here because:
     * - In mmu_enable_boot, adrp uses physical addresses (0x4008xxxx)
     * - Here in _start_high, we're at high-half but need to access physical page tables
     * - We use movz/movk to load absolute physical addresses */
    
    /* Load L1_TABLE physical address = 0x40081000 */
    movz x2, #0x1000, lsl #0
    movk x2, #0x4008, lsl #16
    
    /* Calculate L1_TABLE[256] address = L1_TABLE + 256*8 = 0x40081000 + 0x800 = 0x40081800 */
    add x2, x2, #0x800
    
    /* Load L2_TABLE1 physical address = 0x40084000 */
    movz x3, #0x4000, lsl #0
    movk x3, #0x4008, lsl #16
    
    /* Create page table entry = L2_TABLE1 | 0x3 */
    orr x3, x3, #3
    
    /* Write L1_TABLE[256] = L2_TABLE1 | 0x3 */
    str x3, [x2]
    
    /* Flush to ensure write completes */
    dsb ish
    tlbi vmalle1
    dsb ish
    isb
    
    /* 重新设置异常向量到高半地址，保证在切换 TTBR0 后 EL1 始终可用 */
    adrp x0, exception_vector_table
    add  x0, x0, :lo12:exception_vector_table /* 低地址 VA 的向量表 */
    adrp x1, __text_vma_start
    add  x1, x1, :lo12:__text_vma_start      /* 低地址 VA 的 .text 起始 */
    sub  x0, x0, x1                          /* 计算偏移 */
    ldr x2, =0xFFFF800000000000             /* KERNEL_BASE */
    add  x0, x2, x0                          /* 高地址 VA = KERNEL_BASE + 偏移 */
    msr  vbar_el1, x0
    isb
    
    bl rust_main
    
    /* 6. 如果返回，则停止 */
    b .

/* 异常向量表 */
.align 11
.global exception_vector_table
exception_vector_table:
    /* 当前 EL 使用 SP0 */
    .balign 0x80
    b exception_handler_sync   /* 同步异常 */
    .balign 0x80
    b exception_handler_irq    /* IRQ */
    .balign 0x80
    b exception_handler_fiq    /* FIQ */
    .balign 0x80
    b exception_handler_serror /* SError */
    
    /* 当前 EL 使用 SPx */
    .balign 0x80
    b exception_handler_sync
    .balign 0x80
    b exception_handler_irq
    .balign 0x80
    b exception_handler_fiq
    .balign 0x80
    b exception_handler_serror
    
    /* 更低 EL 使用 AArch64 */
    .balign 0x80
    b exception_handler_sync
    .balign 0x80
    b exception_handler_irq
    .balign 0x80
    b exception_handler_fiq
    .balign 0x80
    b exception_handler_serror
    
    /* 更低 EL 使用 AArch32 */
    .balign 0x80
    b exception_handler_sync
    .balign 0x80
    b exception_handler_irq
    .balign 0x80
    b exception_handler_fiq
    .balign 0x80
    b exception_handler_serror

/* 异常处理程序（简化版） */
exception_handler_sync:
    /* 保存用户寄存器到可保留寄存器，防止被参数覆盖 */
    mov x19, x0
    mov x20, x1
    mov x21, x2
    mov x22, x3
    mov x23, x4
    mov x24, x5
    mov x25, x8

    /* 读取异常信息 */
    mrs x0, esr_el1
    mrs x1, elr_el1
    mrs x2, far_el1
    lsr x10, x0, #26
    and x10, x10, #0x3F
    mov x26, x0
    mov x27, x1
    mov x28, x2
    mov x0, x10
    mov x1, x26
    mov x2, x27
    mov x3, x28
    bl rust_exc_mark
    mov x0, x26
    mov x1, x27
    mov x2, x28
    cmp x10, #0x15
    b.ne 9f
    /* 传递 esr/elr/far 给 Rust 处理函数 */
    /* x0=esr, x1=elr, x2=far */
    bl rust_svc_handler
    /* 跳过触发异常的指令：ELR += 4 */
    add x1, x1, #4
    msr elr_el1, x1
    eret
9:
    mrs x0, esr_el1
    mrs x1, elr_el1
    mrs x2, far_el1
    mrs x3, tcr_el1
    mrs x4, sctlr_el1
    mrs x5, spsr_el1
    bl rust_sync_try_handle
    cmp x0, #0
    b.ne 10f
    bl rust_sync_panic
    b exception_panic
10:
    eret

exception_handler_irq:
    /* Save caller-saved registers (x0-x18) */
    stp x0, x1, [sp, #-16]!
    stp x2, x3, [sp, #-16]!
    stp x4, x5, [sp, #-16]!
    stp x6, x7, [sp, #-16]!
    stp x8, x9, [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x18, xzr, [sp, #-16]!
    bl rust_irq_handler
    /* Restore caller-saved registers */
    ldp x18, xzr, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x10, x11, [sp], #16
    ldp x8, x9, [sp], #16
    ldp x6, x7, [sp], #16
    ldp x4, x5, [sp], #16
    ldp x2, x3, [sp], #16
    ldp x0, x1, [sp], #16
    eret

exception_handler_fiq:
exception_handler_serror:
exception_panic:
    /* 无法恢复的异常，停止 */
    b .

/* 早期 UART 初始化与输出（物理地址） */
.section .text.boot
.global early_uart_init
early_uart_init:
    ldr x0, =0x09000000
    mov x1, #0
    str w1, [x0, #0x30]
    mov x1, #13
    str w1, [x0, #0x24]
    mov x1, #2
    str w1, [x0, #0x28]
    mov x1, #0x70
    str w1, [x0, #0x2C]
    mov x1, #0x301
    str w1, [x0, #0x30]
    mov x1, #0x7FF
    str w1, [x0, #0x44]
    ret

.global early_uart_print
early_uart_print:
    ldr x1, =0x09000000
    mov w4, #512
1:
    ldrb w2, [x0], #1
    cbz w2, 2f
    subs w4, w4, #1
    beq 2f
3:
    ldr w3, [x1, #0x18]
    tbnz w3, #5, 3b
    str w2, [x1, #0x00]
    b 1b
2:
    ret

.section .rodata.boot
early_msg:
    .asciz "Welcome\n"

/* 汇编版 EL0 切换入口 - 完全无栈版本 */
.section .text
.balign 4
.global arch_do_exec
arch_do_exec:
    /* x0 = elr, x1 = sp0, x2 = ttbr0_with_asid, x3-x6 = args (unused) */
    
    /* Immediately set all system registers WITHOUT touching stack or memory */
    
    /* Set SP_EL0 */
    msr sp_el0, x1
    
    /* Set ELR_EL1 */
    msr elr_el1, x0
    
    /* Set SPSR_EL1 = 0 (EL0t) */
    msr spsr_el1, xzr
    
    /* Ensure SPSel = 1 (use SP_EL1 for exceptions) */
    msr spsel, #1
    
    /* Set TTBR0_EL1 */
    msr ttbr0_el1, x2
    isb
    
    /* Invalidate TLB for ASID */
    lsr x10, x2, #48
    and x10, x10, #0xFFFF
    lsl x10, x10, #48
    tlbi aside1is, x10
    dsb ish
    
    /* Invalidate I-cache */
    ic iallu
    dsb ish
    isb
    
    /* Set preserved regs for SVC - these are for user space use */
    mov x19, x3
    mov x20, x4
    mov x21, x5
    
    /* Clear ALL other registers to ensure clean state */
    mov x0, xzr
    mov x1, xzr
    mov x2, xzr
    mov x3, xzr
    mov x4, xzr
    mov x5, xzr
    mov x6, xzr
    mov x7, xzr
    mov x8, xzr
    mov x9, xzr
    mov x10, xzr
    mov x11, xzr
    mov x12, xzr
    mov x13, xzr
    mov x14, xzr
    mov x15, xzr
    mov x16, xzr
    mov x17, xzr
    mov x18, xzr
    /* x19-x21 contain SVC args, don't clear */
    mov x22, xzr
    mov x23, xzr
    mov x24, xzr
    mov x25, xzr
    mov x26, xzr
    mov x27, xzr
    mov x28, xzr
    mov x29, xzr
    mov x30, xzr
    
    /* Final ISB */
    isb
    
    /* ERET to EL0 - NEVER returns */
    eret

/* removed legacy print helpers */
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
    
    /* Enable MMU with identity mapping */
    bl mmu_enable_boot
    
    /* MMU enabled, continue at physical address (identity mapped) */
    /* Setup main kernel stack */
    ldr x0, =__stack_top
    and x0, x0, #0xFFFFFFFFFFFFFFF0
    mov sp, x0
    
    /* Jump to Rust main */
    bl rust_main
    
    /* If rust_main returns, halt */
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
    /* 保存所有被调用者保存寄存器到栈中 (x19-x25) */
    stp x19, x20, [sp, #-16]!
    stp x21, x22, [sp, #-16]!
    stp x23, x24, [sp, #-16]!
    stp x25, xzr, [sp, #-16]!
    /* 保存所有调用者保存寄存器到栈中 (x0-x18) */
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

    /* 读取异常信息 */
    mrs x0, esr_el1
    mrs x1, elr_el1
    mrs x2, far_el1
    lsr x10, x0, #26
    and x10, x10, #0x3F
    mov x23, x0               /* 保存esr到x23 */
    mov x24, x1               /* 保存elr到x24 */
    mov x25, x2               /* 保存far到x25 */
    mov x14, x10              /* 保存ec到x14 */
    mov x0, x10               /* 参数1: ec */
    mov x1, x23               /* 参数2: esr */
    mov x2, x24               /* 参数3: elr */
    mov x3, x25               /* 参数4: far */
    bl rust_exc_mark
    /* 恢复esr/elr/far从x23-x25到x0-x2 */
    mov x0, x23               /* 恢复esr到x0 */
    mov x1, x24               /* 恢复elr到x1 */
    mov x2, x25               /* 恢复far到x2 */
    mov x10, x14              /* 恢复ec到x10 */
    cmp x10, #0x15
    b.ne 9f
    /* 传递 esr/elr/far/saved_x8/saved_sp 给 Rust 处理函数 */
    /* x0=esr, x1=elr, x2=far, x3=saved_x8, x4=saved_sp */
    mov x22, x1               /* 保存elr到x22 */
    /* 读取保存的x8从栈上 (sp+80) 到 x3 */
    add x3, sp, #80
    ldr x3, [x3]
    /* 传递当前sp (寄存器保存区域的开始) 到 x4 */
    mov x4, sp
    bl rust_svc_handler
    /* 保存系统调用返回值到栈上x0的位置 (sp+144) */
    add x9, sp, #144
    str x0, [x9]
    /* 跳过触发异常的指令：ELR += 4 */
    add x22, x22, #4
    msr elr_el1, x22
    /* 恢复所有调用者保存寄存器从栈中 */
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
    /* 恢复所有被调用者保存寄存器从栈中 (x19-x25) */
    ldp x25, xzr, [sp], #16
    ldp x23, x24, [sp], #16
    ldp x21, x22, [sp], #16
    ldp x19, x20, [sp], #16
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
    /* 恢复所有调用者保存寄存器从栈中 */
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
    /* 恢复所有被调用者保存寄存器从栈中 (x19-x25) */
    ldp x25, xzr, [sp], #16
    ldp x23, x24, [sp], #16
    ldp x21, x22, [sp], #16
    ldp x19, x20, [sp], #16
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
    mov x0, #0x9000000
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
    mov x1, #0x9000000
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
msg_before_mmu:
    .asciz "[BOOT] Before MMU enable\n"
msg_after_mmu:
    .asciz "[BOOT] After MMU enable\n"
msg_before_rust:
    .asciz "[BOOT] Jumping to rust_main\n"

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
    
    /* Set SPSR_EL1 = 0 (EL0t with interrupts enabled) */
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
    
    /* Debug: Read back critical registers before ERET */
    mrs x10, elr_el1
    mrs x11, sp_el0
    mrs x12, spsr_el1
    mrs x13, ttbr0_el1
    /* Registers checked: x10=ELR, x11=SP_EL0, x12=SPSR, x13=TTBR0 */
    
    /* ERET to EL0 - NEVER returns */
    eret

/* removed legacy print helpers */
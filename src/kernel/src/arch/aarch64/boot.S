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
    
    /* 1.5 清零 BSS 段（使用链接脚本别名） */
    ldr x0, =__bss_start
    ldr x1, =__bss_end
    cmp x0, x1
    b.eq 1f
0:
    str xzr, [x0], #8
    cmp x0, x1
    b.lo 0b
2:
    cmp x0, x1
    b.eq 1f
3:
    strb wzr, [x0], #1
    cmp x0, x1
    b.lo 3b
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
exec_msg1:
    .asciz "E1\n"
exec_msg2:
    .asciz "E2\n"
exec_msg3:
    .asciz "E3\n"
exec_msg4:
    .asciz "E4\n"
exec_msg5:
    .asciz "E5\n"

/* 汇编版 EL0 切换入口，参考 hnx-old */
.global arch_do_exec
arch_do_exec:
    adrp x12, EXEC_STAGE
    add  x12, x12, :lo12:EXEC_STAGE
    mov  x13, #1
    str  x13, [x12]
    adrp x0, exec_msg1
    add  x0, x0, :lo12:exec_msg1
    bl early_uart_print
    mov x19, x0
    mov x20, x1
    mov x21, x2  // x21 = ttbr0_with_asid (includes ASID in bits [63:48])
    
    /* CRITICAL SECURITY: Verify VBAR_EL1 points to kernel exception vectors */
    /* This prevents user code from redirecting exceptions to malicious handlers */
    mrs x10, vbar_el1
    adrp x11, exception_vector_table
    add  x11, x11, :lo12:exception_vector_table
    adrp x9, __text_vma_start
    add  x9, x9, :lo12:__text_vma_start
    sub  x11, x11, x9
    // Load KERNEL_BASE using movz/movk instead of ldr pseudo-instruction
    movz x9, #0x0000, lsl #0
    movk x9, #0x0000, lsl #16
    movk x9, #0x8000, lsl #32
    movk x9, #0xFFFF, lsl #48
    add  x11, x9, x11
    cmp  x10, x11
    b.eq 1f
    /* VBAR mismatch - security violation, halt */
    b .
1:
    
    /* Preflight with (elr, sp0, ttbr0) */
    mov x0, x19
    mov x1, x20
    mov x2, x21
    bl arch_exec_preflight
    mov  x13, #2
    str  x13, [x12]
    adrp x0, exec_msg2
    add  x0, x0, :lo12:exec_msg2
    bl early_uart_print
    msr sp_el0, x20
    mov  x13, #3
    str  x13, [x12]
    adrp x0, exec_msg3
    add  x0, x0, :lo12:exec_msg3
    bl early_uart_print
    
    /* CRITICAL SECURITY: Set TTBR0_EL1 with ASID to isolate address spaces */
    /* ASID is in bits [63:48], page table base in [47:0] */
    msr ttbr0_el1, x21
    isb
    
    /* SECURITY: Invalidate TLB entries for this ASID to prevent stale translations */
    /* We use TLBI ASIDE1IS to invalidate all entries for this ASID across all PEs */
    lsr x10, x21, #48        // Extract ASID from bits [63:48]
    and x10, x10, #0xFFFF
    lsl x10, x10, #48        // Prepare ASID for TLBI instruction
    tlbi aside1is, x10       // Invalidate all TLB entries for this ASID
    dsb ish
    isb
    
    mov  x13, #4
    str  x13, [x12]
    adrp x0, exec_msg4
    add  x0, x0, :lo12:exec_msg4
    bl early_uart_print
    msr elr_el1, x19
    /* Prepare args for postflight (ttbr0, sp_el0, spsr) via MRS reads below */
    mov x0, x19
    mov x1, x20
    mov x2, x21
    mov x8, x6
    /* Return to EL0t (SPSR=0x0) */
    mov x10, #0x0
    msr spsr_el1, x10
    /* Ensure EL1 exceptions use SP_EL1 (kernel stack) */
    msr spsel, #1
    mrs x0, ttbr0_el1
    mrs x1, sp_el0
    mrs x2, spsr_el1
    bl arch_exec_postflight
    /* Finally set preserved regs for first SVC (fd, buf, len) in x19..x21 */
    mov x19, x3
    mov x20, x4
    mov x21, x5
    mov  x13, #5
    str  x13, [x12]
    adrp x0, exec_msg5
    add  x0, x0, :lo12:exec_msg5
    bl early_uart_print
    isb
    eret

/* removed legacy print helpers */
.section .text
.global test_direct_eret
test_direct_eret:
    /* Directly test ERET without any Rust code interference */
    
    /* Set ELR_EL1 = 0x210248 (user entry) */
    mov x0, #0x210248
    msr elr_el1, x0
    
    /* Set SP_EL0 = 0x221000 (user stack) */
    mov x0, #0x221000
    msr sp_el0, x0
    
    /* Set SPSR_EL1 = 0 (EL0t) */
    msr spsr_el1, xzr
    
    /* Set TTBR0_EL1 with user page table + ASID */
    movz x0, #0x6000, lsl #16
    movk x0, #0x4039, lsl #32
    movk x0, #0x0001, lsl #48
    msr ttbr0_el1, x0
    isb
    
    /* Invalidate TLB */
    movz x0, #0x0001, lsl #48
    tlbi aside1is, x0
    dsb ish
    
    /* Invalidate I-cache */
    ic iallu
    dsb ish
    isb
    
    /* Clear all registers */
    mov x0, xzr
    mov x1, xzr
    mov x2, xzr
    mov x3, xzr
    
    /* ERET */
    eret

const PHYS_BASE: u64 = 0x4000_0000;

#[repr(C, align(4096))]
struct AlignedTable([u64; 512]);

#[no_mangle]
#[link_section = ".data.boot"]
pub static mut L1_TABLE_KERNEL: AlignedTable = AlignedTable([0; 512]);

#[no_mangle]
#[link_section = ".data.boot"]
pub static mut L2_TABLE_KERNEL: AlignedTable = AlignedTable([0; 512]);

#[no_mangle]
#[link_section = ".data.boot"]
pub static mut L1_TABLE_USER: AlignedTable = AlignedTable([0; 512]);

#[no_mangle]
#[link_section = ".data.boot"]
pub static mut L2_TABLE_USER: AlignedTable = AlignedTable([0; 512]);

#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn mmu_enable_boot() {
    let mair: u64 = 0xFF;
    let tcr: u64 = ((25u64
        | (0b10u64 << 12)
        | (0b01u64 << 10)
        | (0b01u64 << 8)
        | (25u64 << 16)
        | (0b10u64 << 30)
        | (0b10u64 << 28)
        | (0b01u64 << 26)
        | (0b01u64 << 24)))
        | (0b010u64 << 32);
    
    // crate::info!("mmu_boot: Identity mapping kernel at 0x40000000+");
    
    core::arch::asm!(
        "msr mair_el1, {mair}",
        "msr tcr_el1, {tcr}",
        "isb",
        
        // Setup TTBR1_EL1 for kernel identity mapping
        // L1_TABLE_KERNEL[1] -> L2_TABLE_KERNEL, maps 0x40000000-0x7FFFFFFF (1GB)
        "adrp x3, {l1_kern}",
        "add  x3, x3, :lo12:{l1_kern}",
        "add  x3, x3, #8",                   // L1_TABLE_KERNEL[1]
        "adrp x2, {l2_kern}",
        "add  x2, x2, :lo12:{l2_kern}",
        "orr  x2, x2, #3",                   // Table descriptor
        "str  x2, [x3]",
        
        // Map 256MB in L2_TABLE_KERNEL: 0x40000000-0x4FFFFFFF
        // 128 entries * 2MB each (covers all kernel code, data, and stack)
        "adrp x5, {l2_kern}",
        "add  x5, x5, :lo12:{l2_kern}",
        "mov  x6, #0",                       // Counter
        "mov  x7, {phys}",                   // Start PA = 0x40000000
        "mov  x1, #0",
        "orr  x1, x1, #(1 << 10)",           // AF
        "orr  x1, x1, #(2 << 8)",            // Normal memory
        
        "1:",
        "cmp  x6, #128",                     // Map 128 * 2MB = 256MB
        "b.ge 2f",
        "mov  x2, x7",
        "orr  x2, x2, #1",                   // Block descriptor
        "orr  x2, x2, x1",
        "str  x2, [x5]",
        "add  x5, x5, #8",
        "add  x7, x7, #(2 * 1024 * 1024)",
        "add  x6, x6, #1",
        "b    1b",
        "2:",

        // Map GIC device memory in L2_TABLE_KERNEL at index 128 (VA 0x50000000)
        "adrp x5, {l2_kern}",
        "add  x5, x5, :lo12:{l2_kern}",
        "mov  x6, #128",
        "add  x5, x5, x6, lsl #3",
        "mov  x1, #0",
        "orr  x1, x1, #(1 << 10)",           // AF
        "orr  x1, x1, #(1 << 2)",            // Device memory
        "mov  x2, {gicd}",
        "orr  x2, x2, #1",                   // Block descriptor
        "orr  x2, x2, x1",
        "str  x2, [x5]",

        // Setup TTBR0_EL1 for user space
        // L1_TABLE_USER[0] -> L2_TABLE_USER
        "adrp x3, {l1_user}",
        "add  x3, x3, :lo12:{l1_user}",
        "adrp x2, {l2_user}",
        "add  x2, x2, :lo12:{l2_user}",
        "orr  x2, x2, #3",
        "str  x2, [x3]",
        
        // Map devices in L2_TABLE_USER
        // GIC at index 64: VA 0x08000000
        "adrp x5, {l2_user}",
        "add  x5, x5, :lo12:{l2_user}",
        "mov  x6, #64",
        "add  x5, x5, x6, lsl #3",
        "mov  x1, #0",
        "orr  x1, x1, #(1 << 10)",           // AF
        "orr  x1, x1, #(1 << 2)",            // Device memory
        "mov  x2, {gicd}",
        "orr  x2, x2, #1",
        "orr  x2, x2, x1",
        "str  x2, [x5]",
        
        // UART at index 72: VA 0x09000000
        "adrp x5, {l2_user}",
        "add  x5, x5, :lo12:{l2_user}",
        "mov  x6, #72",
        "add  x5, x5, x6, lsl #3",
        "mov  x1, #0",
        "orr  x1, x1, #(1 << 10)",
        "orr  x1, x1, #(1 << 2)",
        "mov  x2, {uart}",
        "orr  x2, x2, #1",
        "orr  x2, x2, x1",
        "str  x2, [x5]",
        
        // UART16550 at index 128: VA 0x10000000
        "adrp x5, {l2_user}",
        "add  x5, x5, :lo12:{l2_user}",
        "mov  x6, #128",
        "add  x5, x5, x6, lsl #3",
        "mov  x1, #0",
        "orr  x1, x1, #(1 << 10)",
        "orr  x1, x1, #(1 << 2)",
        "mov  x2, {uart16550}",
        "orr  x2, x2, #1",
        "orr  x2, x2, x1",
        "str  x2, [x5]",
        
        // Physical memory at L1_TABLE_USER[1]: VA 0x40000000+
        "adrp x3, {l1_user}",
        "add  x3, x3, :lo12:{l1_user}",
        "add  x3, x3, #8",                   // L1_TABLE_USER[1]
        "mov  x1, #0",
        "orr  x1, x1, #(1 << 10)",
        "orr  x1, x1, #(2 << 8)",
        "mov  x2, {phys}",
        "orr  x2, x2, #1",                   // 1GB block
        "orr  x2, x2, x1",
        "str  x2, [x3]",
        
        // Load TTBRs
        "adrp x9, {l1_kern}",
        "add  x9, x9, :lo12:{l1_kern}",
        "msr  ttbr1_el1, x9",
        
        "adrp x9, {l1_user}",
        "add  x9, x9, :lo12:{l1_user}",
        "msr  ttbr0_el1, x9",
        
        // Enable MMU
        "dsb  ish",
        "tlbi vmalle1",
        "dsb  ish",
        "ic   iallu",
        "dsb  ish",
        "isb",
        
        "mrs  x5, sctlr_el1",
        "orr  x5, x5, #1",                   // M bit
        "orr  x5, x5, #(1 << 2)",            // C bit
        "orr  x5, x5, #(1 << 12)",           // I bit
        "msr  sctlr_el1, x5",
        "isb",
        
        // Return to caller (still at physical address)
        "ret",
        
        mair = in(reg) mair,
        tcr = in(reg) tcr,
        phys = const PHYS_BASE,
        l1_kern = sym L1_TABLE_KERNEL,
        l2_kern = sym L2_TABLE_KERNEL,
        l1_user = sym L1_TABLE_USER,
        l2_user = sym L2_TABLE_USER,
        gicd = const 0x0800_0000,
        uart = const 0x0900_0000,
        uart16550 = const 0x1000_0000,
        options(nostack)
    );
}
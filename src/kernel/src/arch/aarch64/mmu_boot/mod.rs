const PHYS_BASE: u64 = 0x4000_0000;
const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;

#[repr(C, align(4096))]
struct AlignedTable([u64; 512]);

#[no_mangle]
#[link_section = ".data.boot"]
static mut L1_TABLE: AlignedTable = AlignedTable([0; 512]);
#[no_mangle]
#[link_section = ".data.boot"]
static mut L1_TABLE0: AlignedTable = AlignedTable([0; 512]);
#[no_mangle]
#[link_section = ".data.boot"]
static mut L2_TABLE1: AlignedTable = AlignedTable([0; 512]);
#[no_mangle]
#[link_section = ".data.boot"]
static mut L2_TABLE0: AlignedTable = AlignedTable([0; 512]);

#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn mmu_enable_boot() {
    extern "C" {
        static mut __text_lma_value: u64;
        static __text_vma_start: u8;
        fn _start_high();
    }
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
    core::arch::asm!(
        "mov x0, {uart}",
        "msr mair_el1, {mair}",
        "msr tcr_el1, {tcr}",
        "isb",
        "adrp x2, {l1sym}",
        "add  x2, x2, :lo12:{l1sym}",
        "movz x4, #0x0000, lsl #0",
        "movk x4, #0x0000, lsl #16",
        "movk x4, #0x8000, lsl #32",
        "movk x4, #0xFFFF, lsl #48",
        "lsr  x6, x4, #39",
        "and  x6, x6, #0x1FF",
        "lsl  x5, x6, #3",
        "add  x10, x2, x5",
        "adrp x11, {l2sym}",
        "add  x11, x11, :lo12:{l2sym}",
        "orr  x11, x11, #3",
        "str  x11, [x10]",
        "adrp x5, {l2sym}",
        "add  x5, x5, :lo12:{l2sym}",
        "adrp x7, {text_lma}",
        "add  x7, x7, :lo12:{text_lma}",
        "ldr  x7, [x7]",
        "mov x1, #0",
        "orr x1, x1, #(1 << 10)",
        "orr x1, x1, #(2 << 8)",
        "orr x1, x1, #(1 << 6)",
        "mov x2, x7",
        "orr x2, x2, #1",
        "orr x2, x2, x1",
        "str x2, [x5]",
        "mov x6, #0",
        "adrp x5, {l2sym}",
        "add  x5, x5, :lo12:{l2sym}",
        "add x5, x5, #8",
        "add x7, x7, #(2 * 1024 * 1024)",
        "1:",
        "cmp x6, #63",
        "b.ge 2f",
        "mov x1, #0",
        "orr x1, x1, #(1 << 10)",
        "orr x1, x1, #(2 << 8)",
        "mov x2, x7",
        "orr x2, x2, #1",
        "orr x2, x2, x1",
        "str x2, [x5]",
        "add x5, x5, #8",
        "add x7, x7, #(2 * 1024 * 1024)",
        "add x6, x6, #1",
        "b 1b",
        "2:",
        "adrp x3, {l10sym}",
        "add  x3, x3, :lo12:{l10sym}",
        "mov x4, #0",
        "lsl x4, x4, #3",
        "add x3, x3, x4",
        "adrp x2, {l20sym}",
        "add  x2, x2, :lo12:{l20sym}",
        "orr x2, x2, #3",
        "str x2, [x3]",
        "adrp x5, {l20sym}",
        "add  x5, x5, :lo12:{l20sym}",
        "mov x6, #64",
        "lsl x6, x6, #3",
        "add x5, x5, x6",
        "mov x1, #0",
        "orr x1, x1, #(1 << 10)",
        "orr x1, x1, #(1 << 2)",
        "mov x2, {gicd}",
        "orr x2, x2, #1",
        "orr x2, x2, x1",
        "str x2, [x5]",
        "adrp x5, {l20sym}",
        "add  x5, x5, :lo12:{l20sym}",
        "mov x6, #72",
        "lsl x6, x6, #3",
        "add x5, x5, x6",
        "mov x1, #0",
        "orr x1, x1, #(1 << 10)",
        "orr x1, x1, #(1 << 2)",
        "mov x2, {uart}",
        "orr x2, x2, #1",
        "orr x2, x2, x1",
        "str x2, [x5]",
        "adrp x5, {l20sym}",
        "add  x5, x5, :lo12:{l20sym}",
        "mov x6, #128",
        "lsl x6, x6, #3",
        "add x5, x5, x6",
        "mov x1, #0",
        "orr x1, x1, #(1 << 10)",
        "orr x1, x1, #(1 << 2)",
        "mov x2, {uart16550}",
        "orr x2, x2, #1",
        "orr x2, x2, x1",
        "str x2, [x5]",
        "mov x1, #0",
        "orr x1, x1, #(1 << 10)",
        "orr x1, x1, #(2 << 8)",
        "mov x2, {phys}",
        "orr x2, x2, #1",
        "orr x2, x2, x1",
        "adrp x3, {l10sym}",
        "add  x3, x3, :lo12:{l10sym}",
        "mov x4, #1",
        "lsl x4, x4, #3",
        "add x3, x3, x4",
        "str x2, [x3]",
        "adrp x3, {l10sym}",
        "add  x3, x3, :lo12:{l10sym}",
        "adrp x11, {l20sym}",
        "add  x11, x11, :lo12:{l20sym}",
        "orr  x11, x11, #3",
        "str  x11, [x3]",
        "adrp x9, {l1sym}",
        "add  x9, x9, :lo12:{l1sym}",
        "msr ttbr1_el1, x9",
        "adrp x9, {l10sym}",
        "add  x9, x9, :lo12:{l10sym}",
        "msr ttbr0_el1, x9",
        "dsb ish",
        "tlbi vmalle1",
        "dsb ish",
        "ic iallu",
        "dsb ish",
        "isb",
        "mrs x5, sctlr_el1",
        "orr x5, x5, #1",
        "orr x5, x5, #(1 << 2)",
        "orr x5, x5, #(1 << 12)",
        "msr sctlr_el1, x5",
        "isb",
        "adrp x0, {starth}",
        "add  x0, x0, :lo12:{starth}",
        "br   x0",
        mair = in(reg) mair,
        tcr = in(reg) tcr,
        phys = const PHYS_BASE,
        text_lma = sym __text_lma_value,
        starth = sym _start_high,
        l1sym = sym L1_TABLE,
        l10sym = sym L1_TABLE0,
        l2sym = sym L2_TABLE1,
        l20sym = sym L2_TABLE0,
        gicd = const 0x0800_0000,
        uart = const 0x0900_0000,
        uart16550 = const 0x1000_0000,
        options(nostack, preserves_flags)
    );
}

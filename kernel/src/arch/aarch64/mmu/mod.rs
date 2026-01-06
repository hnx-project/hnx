pub mod mmu_boot;
pub fn init() {}

pub fn enable() {}

pub fn flush_icache_all() {
    unsafe {
        core::arch::asm!("ic iallu; dsb ish; isb");
    }
}

pub fn clean_dcache_range(addr: usize, size: usize) {
    if size == 0 { return; }
    let mut p = addr & !63usize;
    let end = addr.saturating_add(size);
    unsafe {
        while p < end {
            core::arch::asm!("dc cvau, {x}", x = in(reg) p);
            p = p.saturating_add(64);
        }
        core::arch::asm!("dsb ish; isb");
    }
}

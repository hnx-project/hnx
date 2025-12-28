//! ARMv8 (aarch64) 架构实现

pub struct Arch;
pub const ARCH_NAME: &str = "aarch64";

pub mod boot;
pub mod console;
pub mod interrupt;
pub mod mmu;
pub mod timer;

pub fn init() {
    interrupt::init();
    mmu::init();
    timer::init();
    crate::security::init();
}

pub fn cpu_id() -> u32 {
    let mut cpu_id: u64;
    unsafe {
        core::arch::asm!("mrs {}, mpidr_el1", out(reg) cpu_id);
    }
    (cpu_id & 0xFF) as u32
}

pub fn disable() {
    interrupt::disable();
}

pub fn exec_preflight(elr: usize) {
    unsafe {
        let mut spsr: u64 = 0;
        let mut cel: u64 = 0;
        let mut vbar: u64 = 0;
        let mut tt0: u64 = 0;
        let mut tt1: u64 = 0;
        let mut sp_el0: u64 = 0;
        core::arch::asm!("mrs {s}, spsr_el1", s = out(reg) spsr);
        core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cel);
        core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
        core::arch::asm!("mrs {t0}, ttbr0_el1", t0 = out(reg) tt0);
        core::arch::asm!("mrs {t1}, ttbr1_el1", t1 = out(reg) tt1);
        core::arch::asm!("mrs {sp}, sp_el0", sp = out(reg) sp_el0);
        crate::debug!(
            "arch/aarch64 exec preflight: ELR=0x{:016X} SPSR=0x{:016X} VBAR=0x{:016X} CurrentEL=0x{:016X} SP_EL0=0x{:016X} TTBR0=0x{:016X} TTBR1=0x{:016X}",
            elr as u64, spsr, vbar, cel, sp_el0, tt0, tt1
        );
    }
}

pub fn dump_panic_state() {
    unsafe {
        let mut x: [u64; 31] = [0; 31];
        core::arch::asm!(
            "mov {x0}, x0", x0 = out(reg) x[0]
        );
        // minimal dump of EL/TTBR/TCR/SCTLR
        let mut cur_el: u64 = 0;
        let mut vbar: u64 = 0;
        let mut tt0: u64 = 0;
        let mut tt1: u64 = 0;
        let mut spsr: u64 = 0;
        let mut elr: u64 = 0;
        let mut tcr: u64 = 0;
        let mut sctlr: u64 = 0;
        core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cur_el);
        core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
        core::arch::asm!("mrs {t0}, ttbr0_el1", t0 = out(reg) tt0);
        core::arch::asm!("mrs {t1}, ttbr1_el1", t1 = out(reg) tt1);
        core::arch::asm!("mrs {s}, spsr_el1", s = out(reg) spsr);
        core::arch::asm!("mrs {e}, elr_el1", e = out(reg) elr);
        core::arch::asm!("mrs {t}, tcr_el1", t = out(reg) tcr);
        core::arch::asm!("mrs {sctl}, sctlr_el1", sctl = out(reg) sctlr);
        crate::print!(
            "EL=0x{:016X} VBAR=0x{:016X} TTBR0=0x{:016X} TTBR1=0x{:016X}\n",
            cur_el, vbar, tt0, tt1
        );
        crate::print!("SPSR=0x{:016X} ELR=0x{:016X}\n", spsr, elr);
        crate::print!("TCR=0x{:016X} SCTLR=0x{:016X}\n", tcr, sctlr);
        extern "C" {
            static LAST_ESR: core::sync::atomic::AtomicU64;
            static LAST_ELR: core::sync::atomic::AtomicU64;
            static LAST_FAR: core::sync::atomic::AtomicU64;
            static LAST_EC: core::sync::atomic::AtomicU64;
        }
        let esr = LAST_ESR.load(core::sync::atomic::Ordering::Relaxed);
        let elr_last = LAST_ELR.load(core::sync::atomic::Ordering::Relaxed);
        let far_last = LAST_FAR.load(core::sync::atomic::Ordering::Relaxed);
        let ec = LAST_EC.load(core::sync::atomic::Ordering::Relaxed);
        if esr != 0 || elr_last != 0 || far_last != 0 || ec != 0 {
            crate::print!(
                "LastExc EC=0x{:X} ESR=0x{:016X} ELR=0x{:016X} FAR=0x{:016X}\n",
                ec, esr, elr_last, far_last
            );
        }
    }
}
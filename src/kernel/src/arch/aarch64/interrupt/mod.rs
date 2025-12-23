use crate::debug;
use crate::info;
use crate::kernel_core::scheduler;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static PROGRAM_BREAK: AtomicUsize = AtomicUsize::new(0);
#[no_mangle]
pub static LAST_ESR: AtomicU64 = AtomicU64::new(0);
#[no_mangle]
pub static LAST_ELR: AtomicU64 = AtomicU64::new(0);
#[no_mangle]
pub static LAST_FAR: AtomicU64 = AtomicU64::new(0);
#[no_mangle]
pub static LAST_EC: AtomicU64 = AtomicU64::new(0);
#[no_mangle]
pub static EXEC_STAGE: AtomicU64 = AtomicU64::new(0);
static EXEC_STAGE_PRINTED: AtomicU64 = AtomicU64::new(0);

/// Maximum allowed interrupt nesting depth for stack safety
const MAX_IRQ_NESTING_DEPTH: usize = 8;

pub fn init() {
    crate::drivers::gic::init();
}

pub fn enable() {
    unsafe {
        core::arch::asm!("msr daifclr, #2");
    }
}

pub fn disable() {
    unsafe {
        core::arch::asm!("msr daifset, #2");
    }
}

/// Assembly helper to enable IRQ in handlers (allows preemption)
#[inline(always)]
unsafe fn enable_irq() {
    core::arch::asm!("msr daifclr, #2");  // Clear I bit in DAIF
}

/// Assembly helper to disable IRQ in handlers (prevent preemption)
#[inline(always)]
unsafe fn disable_irq() {
    core::arch::asm!("msr daifset, #2");  // Set I bit in DAIF
}

#[no_mangle]
pub extern "C" fn rust_svc_handler(esr: u64, elr: u64, far: u64) {
    info!("arch/aarch64 SVC");
    let ec = (esr >> 26) & 0x3F;
    
    // CRITICAL SECURITY: Verify exception came from EL0
    // Check SPSR_EL1 to determine exception source
    let mut spsr: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {s}, spsr_el1", s = out(reg) spsr);
    }
    
    // SPSR[3:0] = M[3:0] contains the exception level and mode
    // 0b0000 (0x0) = EL0t (user mode)
    // 0b0100 (0x4) = EL1t (kernel mode with SP_EL0)
    // 0b0101 (0x5) = EL1h (kernel mode with SP_EL1)
    let exception_level = spsr & 0xF;
    
    if exception_level != 0x0 {
        // SVC from EL1 - this should not happen in normal operation
        panic!(
            "SVC from EL1 detected - possible privilege escalation attempt! SPSR=0x{:X} ELR=0x{:X}",
            spsr, elr
        );
    }
    
    if ec == 0x15 {
        let mut cel: u64 = 0;
        let mut spsel: u64 = 0;
        let mut ttbr0: u64 = 0;
        let mut ttbr1: u64 = 0;
        unsafe {
            core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cel);
            core::arch::asm!("mrs {p}, SPSel", p = out(reg) spsel);
            core::arch::asm!("mrs {t0}, ttbr0_el1", t0 = out(reg) ttbr0);
            core::arch::asm!("mrs {t1}, ttbr1_el1", t1 = out(reg) ttbr1);
        }
        
        // Extract ASID from TTBR0_EL1[63:48]
        let asid = (ttbr0 >> 48) & 0xFFFF;
        
        info!("arch/aarch64 svc enter: esr=0x{:016X} elr=0x{:016X} spsr=0x{:016X} currentEL=0x{:016X} spsel={} ttbr0=0x{:016X} asid={} ttbr1=0x{:016X}", esr, elr, spsr, cel, spsel & 1, ttbr0, asid, ttbr1);
        let imm = esr & 0xFFFF;
        if imm == 0 {
            let mut num: usize = 0;
            let mut a0: usize = 0;
            let mut a1: usize = 0;
            let mut a2: usize = 0;
            let mut a3: usize = 0;
            let mut a4: usize = 0;
            let mut a5: usize = 0;
            unsafe {
                core::arch::asm!("mov {n}, x8", n = out(reg) num);
                core::arch::asm!("mov {x0}, x0", x0 = out(reg) a0);
                core::arch::asm!("mov {x1}, x1", x1 = out(reg) a1);
                core::arch::asm!("mov {x2}, x2", x2 = out(reg) a2);
                core::arch::asm!("mov {x3}, x3", x3 = out(reg) a3);
                core::arch::asm!("mov {x4}, x4", x4 = out(reg) a4);
                core::arch::asm!("mov {x5}, x5", x5 = out(reg) a5);
            }
            debug!(
                "arch/aarch64 svc#0 num={} args x0=0x{:X} x1=0x{:X} x2=0x{:X}",
                num, a0, a1, a2
            );
            let ret = crate::process::syscall::dispatch(num as u32, a0, a1, a2, a3, a4, a5) as u64;
            debug!("arch/aarch64 svc#0 ret=0x{:X}", ret);
            unsafe {
                core::arch::asm!("mov x0, {r}", r = in(reg) ret);
            }
        } else {
            match imm {
                1 => {
                    let mut fd: usize = 0;
                    let mut buf: usize = 0;
                    let mut len: usize = 0;
                    unsafe {
                        core::arch::asm!("mov {x19v}, x19", x19v = out(reg) fd);
                        core::arch::asm!("mov {x20v}, x20", x20v = out(reg) buf);
                        core::arch::asm!("mov {x21v}, x21", x21v = out(reg) len);
                    }
                    let ret = crate::process::syscall::dispatch(
                        hnx_abi::HNX_SYS_WRITE,
                        fd,
                        buf,
                        len,
                        0,
                        0,
                        0,
                    ) as u64;
                    unsafe {
                        core::arch::asm!("mov x0, {r}", r = in(reg) ret);
                    }
                }
                2 => {
                    debug!("arch/aarch64 exit enter");
                    let mut a0: usize = 0;
                    unsafe {
                        core::arch::asm!("mov {x0}, x0", x0 = out(reg) a0);
                    }
                    let _ = crate::process::syscall::dispatch(
                        hnx_abi::HNX_SYS_EXIT,
                        a0,
                        0,
                        0,
                        0,
                        0,
                        0,
                    );
                }
                3 => {
                    let mut new_brk: u64 = 0;
                    unsafe {
                        core::arch::asm!("mov {nb}, x19", nb = out(reg) new_brk);
                    }
                    let cur = PROGRAM_BREAK.load(Ordering::Relaxed);
                    if cur == 0 {
                        PROGRAM_BREAK.store(0x8000_0000, Ordering::Relaxed);
                    }
                    if new_brk != 0 {
                        PROGRAM_BREAK.store(new_brk as usize, Ordering::Relaxed);
                    }
                    let retv = PROGRAM_BREAK.load(Ordering::Relaxed) as u64;
                    unsafe {
                        core::arch::asm!("mov x0, {ret}", ret = in(reg) retv);
                    }
                }
                4 => {
                    crate::process::syscall::exec::sys_execve_start();
                }
                _ => {}
            }
        }
    } else {
        panic!(
            "sync exception: EC=0x{:X} ESR=0x{:016X} ELR=0x{:016X} FAR=0x{:016X}",
            ec, esr, elr, far
        );
    }
}

#[no_mangle]
pub extern "C" fn rust_sync_panic(
    esr: u64,
    elr: u64,
    far: u64,
    tcr: u64,
    sctlr: u64,
    spsr: u64,
) -> ! {
    panic!("sync exception: ESR=0x{:016X} ELR=0x{:016X} FAR=0x{:016X} TCR=0x{:016X} SCTLR=0x{:016X} SPSR=0x{:016X}", esr, elr, far, tcr, sctlr, spsr);
}

#[no_mangle]
pub extern "C" fn rust_sync_try_handle(
    esr: u64,
    elr: u64,
    far: u64,
    _tcr: u64,
    _sctlr: u64,
    _spsr: u64,
) -> u64 {
    let ec = (esr >> 26) & 0x3F;
    if (ec == 0x20 || ec == 0x24)
        && crate::memory::virtual_::handle_page_fault(far as usize) {
            return 1;
        }
    0
}

#[no_mangle]
pub extern "C" fn rust_irq_handler() {
    unsafe {
        // Get current running priority before acknowledging interrupt
        let old_priority = crate::drivers::gic::get_running_priority();
        
        // Acknowledge interrupt (this updates the running priority in GIC)
        let iar = crate::drivers::gic::read_iar();
        let intid = iar & 0x3FF;
        
        // Get new running priority after acknowledge
        let new_priority = crate::drivers::gic::get_running_priority();
        
        // Check if this is a preemption (higher priority interrupted lower priority)
        // Lower priority value = higher priority
        let was_preemption = new_priority < old_priority;
        
        // Record statistics
        {
            let mut stats = crate::drivers::gic::stats::IRQ_STATS.lock();
            
            // Check nesting depth for safety
            if stats.current_nesting >= MAX_IRQ_NESTING_DEPTH {
                crate::warn!(
                    "IRQ nesting depth limit reached ({}), not allowing further nesting",
                    MAX_IRQ_NESTING_DEPTH
                );
                // Don't enable interrupts if we're at max depth
            } else {
                stats.record_irq(intid, was_preemption);
            }
        }
        
        if was_preemption {
            debug!(
                "IRQ {} (priority {}) preempted priority {}",
                intid, new_priority, old_priority
            );
        }
        
        // Enable interrupts to allow higher-priority interrupts to preempt this handler
        // Only if we haven't exceeded nesting depth
        let current_nesting = crate::drivers::gic::stats::IRQ_STATS.lock().current_nesting;
        if current_nesting < MAX_IRQ_NESTING_DEPTH {
            enable_irq();
        }
        
        // NEW: Check if this IRQ should be forwarded to user-space
        if crate::drivers::device_manager::DEVICE_MANAGER.lock().is_user_space_irq(intid as u32) {
            // Forward to user-space driver
            crate::drivers::device_manager::DEVICE_MANAGER.lock().forward_interrupt(intid as u32);
            
            // Disable interrupts before EOI to prevent race conditions
            disable_irq();
            
            // End of interrupt (EOI)
            crate::drivers::gic::write_eoi(iar);
            
            // Record exit from interrupt handler
            crate::drivers::gic::stats::IRQ_STATS.lock().exit_irq();
            
            // Return early - handled by user-space
            return;
        }
        
        // Dispatch to handler based on interrupt ID
        match intid {
            30 => {
                // Timer interrupt - scheduling tick
                scheduler::schedule_rr_tick();
            }
            32..=39 => {
                // SGI (Software Generated Interrupts) - reserved for future use
            }
            48..=55 => {
                // Additional device interrupts - reserved for future use
            }
            _ => {
                // Unknown interrupt
                debug!("Unhandled IRQ: {}", intid);
            }
        }
        
        // Disable interrupts before EOI to prevent race conditions
        disable_irq();
        
        // Check exec stage for debugging
        let s = EXEC_STAGE.load(Ordering::Relaxed);
        let ps = EXEC_STAGE_PRINTED.load(Ordering::Relaxed);
        if s != 0 && s != ps {
            info!("arch/aarch64 exec stage {}", s);
            EXEC_STAGE_PRINTED.store(s, Ordering::Relaxed);
        }
        
        // End of interrupt
        crate::drivers::gic::write_eoi(iar);
        
        // Record exit from interrupt handler
        crate::drivers::gic::stats::IRQ_STATS.lock().exit_irq();
    }
}

#[no_mangle]
pub extern "C" fn rust_exc_mark(ec: u64, esr: u64, elr: u64, far: u64) {
    LAST_EC.store(ec, Ordering::Relaxed);
    LAST_ESR.store(esr, Ordering::Relaxed);
    LAST_ELR.store(elr, Ordering::Relaxed);
    LAST_FAR.store(far, Ordering::Relaxed);
    info!("arch/aarch64 exc enter: ec=0x{:X} esr=0x{:016X}", ec, esr);
}

#[no_mangle]
pub extern "C" fn arch_exec_preflight(elr: u64, sp0: u64, ttbr0: u64) {
    crate::console::write_raw("arch_exec_preflight enter\n");
    info!("arch/aarch64 exec preflight enter");
    let mut spsr: u64 = 0;
    let mut cel: u64 = 0;
    let mut vbar: u64 = 0;
    let mut tt0: u64 = 0;
    let mut tt1: u64 = 0;
    let mut sp_el0: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {s}, spsr_el1", s = out(reg) spsr);
        core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cel);
        core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
        core::arch::asm!("mrs {t0}, ttbr0_el1", t0 = out(reg) tt0);
        core::arch::asm!("mrs {t1}, ttbr1_el1", t1 = out(reg) tt1);
        core::arch::asm!("mrs {sp}, sp_el0", sp = out(reg) sp_el0);
    }
    
    // SECURITY AUDIT: Verify exception vector is in kernel space
    const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;
    if vbar < KERNEL_BASE {
        panic!(
            "SECURITY VIOLATION: VBAR_EL1 points to user space! vbar=0x{:016X}",
            vbar
        );
    }
    
    // SECURITY AUDIT: Verify TTBR1 points to kernel page tables (should be high address)
    if tt1 < KERNEL_BASE {
        panic!(
            "SECURITY VIOLATION: TTBR1_EL1 invalid! ttbr1=0x{:016X}",
            tt1
        );
    }
    
    // Extract ASID from TTBR0 for logging
    let asid = (ttbr0 >> 48) & 0xFFFF;
    
    info!(
        "arch/aarch64 exec preflight: ELR=0x{:016X} SPSR=0x{:016X} VBAR=0x{:016X} CurrentEL=0x{:016X} SP_EL0=0x{:016X} TTBR0=0x{:016X} ASID={} TTBR1=0x{:016X}",
        { elr }, spsr, vbar, cel, sp_el0, ttbr0, asid, tt1
    );
}

#[no_mangle]
pub extern "C" fn arch_exec_postflight(ttbr0: u64, sp_el0: u64, spsr: u64) {
    info!(
        "arch/aarch64 exec postflight: TTBR0=0x{:016X} SP_EL0=0x{:016X} SPSR=0x{:016X}",
        ttbr0, sp_el0, spsr
    );
}
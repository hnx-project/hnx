use crate::debug;
use crate::info;
use crate::error;
use crate::core::scheduler;
use crate::arch::common::traits::InterruptController;
use crate::arch::context;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use shared::abi;

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
    crate::arch::cpu::enable_interrupts();
}

pub fn disable() {
    crate::arch::cpu::disable_interrupts();
}

/// Assembly helper to enable IRQ in handlers (allows preemption)
#[inline(always)]
unsafe fn enable_irq() {
    crate::arch::cpu::enable_interrupts();
}

/// Assembly helper to disable IRQ in handlers (prevent preemption)
#[inline(always)]
unsafe fn disable_irq() {
    crate::arch::cpu::disable_interrupts();
}

#[no_mangle]
pub extern "C" fn rust_svc_handler(esr: u64, elr: u64, far: u64, saved_x8: u64, saved_sp: u64) {
    info!("arch/aarch64 SVC handler entered");

    // 简化的调试输出
    let ec = (esr >> 26) & 0x3F;
    let imm = esr & 0xFFFF;
    info!("RUST_SVC_HANDLER: ec=0x{:X}, imm=0x{:X}, saved_x8=0x{:X}, elr=0x{:X}", ec, imm, saved_x8, elr);

    // 特殊调试：检查系统调用号
    if saved_x8 != 0x1001 && saved_x8 != 0x18 {
        debug!("SPECIAL_DEBUG: Non-standard syscall num=0x{:X}", saved_x8);
        if saved_x8 == 0x103 || saved_x8 == 259 {
            debug!("SPAWN_SERVICE_DETECTED: saved_x8=0x{:X} matches HNX_SYS_SPAWN_SERVICE", saved_x8);
        }
    }

    let ec = (esr >> 26) & 0x3F;
    info!("arch/aarch64 svc: ec=0x{:X} esr=0x{:016X} elr=0x{:016X} far=0x{:016X} saved_x8=0x{:016X} saved_sp=0x{:016X}", ec, esr, elr, far, saved_x8, saved_sp);
    
    // CRITICAL SECURITY: Verify exception came from EL0
    // Check SPSR_EL1 to determine exception source
    let spsr = crate::arch::context::get_spsr() as u64;

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

    debug!("DEBUG: ec=0x{:X} checking if == 0x15", ec);
    if ec != 0x15 {
        crate::warn!("WARNING: ec=0x{:X} != 0x15 - SVC call may not be handled correctly!", ec);
    }

    if ec == 0x15 {
        info!("EC=0x15 branch entered - standard SVC call");
        let cel = crate::arch::context::get_current_el() as u64;
        let spsel = crate::arch::context::get_spsel() as u64;
        let ttbr0 = crate::arch::context::get_ttbr0() as u64;
        let ttbr1 = crate::arch::context::get_ttbr1() as u64;

        // Extract ASID from TTBR0_EL1[63:48]
        let asid = (ttbr0 >> 48) & 0xFFFF;

        debug!("arch/aarch64 svc enter: esr=0x{:016X} elr=0x{:016X} spsr=0x{:016X} currentEL=0x{:016X} spsel={} ttbr0=0x{:016X} asid={} ttbr1=0x{:016X}", esr, elr, spsr, cel, spsel & 1, ttbr0, asid, ttbr1);
        let imm = esr & 0xFFFF;
        if imm == 0 {
            // 调试：打印saved_x8和hnx_abi常量的值
            debug!("IMM0_DEBUG: saved_x8=0x{:X}, HNX_SYS_SPAWN_SERVICE={}", saved_x8,shared::abi::syscalls::HNX_SYS_SPAWN_SERVICE);

            // Use saved_sp passed from assembly (start of register save area)
            let sp = saved_sp as usize;
            // Read saved user registers from stack
            // Stack layout: x0-x1 at sp+144, x2-x3 at sp+128, x4-x5 at sp+112
            // x8 is already passed as saved_x8 parameter (from sp+80)
            let saved_x0_ptr = (sp + 144) as *const usize;
            let saved_x1_ptr = (sp + 152) as *const usize;
            let saved_x2_ptr = (sp + 128) as *const usize;
            let saved_x3_ptr = (sp + 136) as *const usize;
            let saved_x4_ptr = (sp + 112) as *const usize;
            let saved_x5_ptr = (sp + 120) as *const usize;

            let a0 = unsafe { saved_x0_ptr.read() };
            let a1 = unsafe { saved_x1_ptr.read() };
            let a2 = unsafe { saved_x2_ptr.read() };
            let a3 = unsafe { saved_x3_ptr.read() };
            let a4 = unsafe { saved_x4_ptr.read() };
            let a5 = unsafe { saved_x5_ptr.read() };

            // Debug: dump stack memory to verify offsets
            crate::debug!("DEBUG: Stack dump around saved_sp (0x{:X}):", sp);
            for i in (0..160).step_by(16) {
                let addr0 = sp + i;
                let addr1 = sp + i + 8;
                let val0 = unsafe { (addr0 as *const usize).read() };
                let val1 = unsafe { (addr1 as *const usize).read() };
                crate::debug!("  sp+{:3}: 0x{:016X} 0x{:016X}", i, val0, val1);
            }

            info!("arch/aarch64 svc#0: saved registers - x8=0x{:X}, x0=0x{:X}, x1=0x{:X}, x2=0x{:X}, x3=0x{:X}, x4=0x{:X}, x5=0x{:X}, saved_sp=0x{:X}",
                  saved_x8, a0, a1, a2, a3, a4, a5, sp);
            info!("HNX_SYS_WRITE={}, HNX_SYS_YIELD={}",shared::abi::syscalls::HNX_SYS_WRITE,shared::abi::syscalls::HNX_SYS_YIELD);
            // Use saved x8 as system call number
            // Check if it's a valid syscall number from abi
            // 调试：检查是否是spawn_service
            debug!("IMM0_BRANCH: saved_x8=0x{:X}", saved_x8);

            if saved_x8 == 0x103 || saved_x8 == 259 {
                debug!("SPAWN_SERVICE_DETECTED_IN_IMM0_BRANCH: saved_x8=0x{:X}, saved_x8_u32=0x{:X}", saved_x8, saved_x8 as u32);
            }
            let saved_x8_u32 = saved_x8 as u32;
            debug!("DEBUG: Checking if saved_x8_u32=0x{:X} is in syscall list, saved_x8=0x{:X}", saved_x8_u32, saved_x8);
            debug!("DEBUG: HNX_SYS_SPAWN_SERVICE=0x{:X}",shared::abi::syscalls::HNX_SYS_SPAWN_SERVICE);

            // 特殊检查spawn_service
            debug!("DEBUG: saved_x8_u32=0x{:X} ({})", saved_x8_u32, saved_x8_u32);
            debug!("DEBUG: HNX_SYS_SPAWN_SERVICE=0x{:X} ({})",shared::abi::syscalls::HNX_SYS_SPAWN_SERVICE,shared::abi::syscalls::HNX_SYS_SPAWN_SERVICE);
            if saved_x8_u32 ==shared::abi::syscalls::HNX_SYS_SPAWN_SERVICE {
                debug!("DEBUG: Special check: saved_x8_u32 == HNX_SYS_SPAWN_SERVICE is TRUE");
            } else {
                debug!("DEBUG: Special check: saved_x8_u32 == HNX_SYS_SPAWN_SERVICE is FALSE");
            }

            // 简化：总是使用saved_x8_u32作为系统调用号
            // 问题：条件判断失败，但saved_x8=0x103应该有效
            let syscall_num = saved_x8_u32;
            debug!("DEBUG: Using syscall_num=0x{:X} (saved_x8_u32) for spawn_service?", syscall_num);
            debug!("DEBUG: saved_x8=0x{:X}, a0=0x{:X}, a1=0x{:X}, a2=0x{:X}", saved_x8, a0, a1, a2);
            debug!("arch/aarch64 svc#0 using syscall_num=0x{:X} (saved_x8=0x{:X})", syscall_num, saved_x8);
            let ret = crate::process::syscall::dispatch(syscall_num, a0, a1, a2, a3, a4, a5) as u64;
            debug!("arch/aarch64 svc#0 ret=0x{:X}", ret);
            // Update saved x0 on stack so it gets restored on exception return
            unsafe {
                let saved_x0_ptr = (sp + 144) as *mut usize;
                debug!("DEBUG: sp=0x{:X}, saved_x0_ptr=sp+144=0x{:X}", sp, saved_x0_ptr as usize);
                let old_value = saved_x0_ptr.read();
                crate::debug!("DEBUG: Before update - saved_x0_ptr=0x{:X}, old_value=0x{:X}, ret=0x{:X}", saved_x0_ptr as usize, old_value, ret);

                // Write the return value to the saved x0 location
                core::ptr::write_volatile(saved_x0_ptr, ret as usize);

                // Ensure the write is visible to the CPU and memory subsystem
                crate::arch::memory::data_sync_barrier();  // Data Synchronization Barrier
                core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

                let new_value = core::ptr::read_volatile(saved_x0_ptr);
                crate::debug!("DEBUG: After update - new_value=0x{:X}", new_value);
            }
        } else {
            match imm {
                1 => {
                    // 从保存的栈中读取寄存器值
                    let saved_sp_usize = saved_sp as usize;
                    let fd = context::get_saved_gpr(saved_sp_usize, 19);  // x19
                    let buf = context::get_saved_gpr(saved_sp_usize, 20); // x20
                    let len = context::get_saved_gpr(saved_sp_usize, 21); // x21
                    let ret = crate::process::syscall::dispatch(
                       shared::abi::syscalls::HNX_SYS_WRITE,
                        fd,
                        buf,
                        len,
                        0,
                        0,
                        0,
                    ) as u64;
                    // Update saved x0 on stack (same offset as above)
                    unsafe {
                        let saved_x0_ptr = (saved_sp as usize + 144) as *mut usize;
                        saved_x0_ptr.write(ret as usize);
                    }
                }
                2 => {
                    debug!("arch/aarch64 exit enter");
                    // 从保存的栈中读取x0寄存器值
                    let saved_sp_usize = saved_sp as usize;
                    let a0 = context::get_saved_gpr(saved_sp_usize, 0);  // x0
                    let _ = crate::process::syscall::dispatch(
                       shared::abi::syscalls::HNX_SYS_EXIT,
                        a0,
                        0,
                        0,
                        0,
                        0,
                        0,
                    );
                }
                3 => {
                    // 从保存的栈中读取x19寄存器值
                    let saved_sp_usize = saved_sp as usize;
                    let new_brk = context::get_saved_gpr(saved_sp_usize, 19) as u64;
                    let cur = PROGRAM_BREAK.load(Ordering::Relaxed);
                    if cur == 0 {
                        PROGRAM_BREAK.store(0x8000_0000, Ordering::Relaxed);
                    }
                    if new_brk != 0 {
                        PROGRAM_BREAK.store(new_brk as usize, Ordering::Relaxed);
                    }
                    let retv = PROGRAM_BREAK.load(Ordering::Relaxed) as u64;
                    // 更新栈上保存的x0值
                    unsafe {
                        let saved_x0_ptr = (saved_sp_usize + 144) as *mut usize;
                        saved_x0_ptr.write(retv as usize);
                    }
                }
                4 => {
                    // crate::process::syscall::exec::sys_execve_start();
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
    let ec = (esr >> 26) & 0x3F;
    let iss = esr & 0xFFFFFF;
    crate::error!("SYNC EXCEPTION:");
    crate::error!("  EC=0x{:02X} ({}) ISS=0x{:06X}", ec, exception_class_name(ec), iss);
    crate::error!("  ESR=0x{:016X} ELR=0x{:016X} FAR=0x{:016X}", esr, elr, far);
    crate::error!("  TCR=0x{:016X} SCTLR=0x{:016X} SPSR=0x{:016X}", tcr, sctlr, spsr);
    
    let ttbr0 = crate::arch::context::get_ttbr0() as u64;
    let ttbr1 = crate::arch::context::get_ttbr1() as u64;
    crate::error!("  TTBR0=0x{:016X} TTBR1=0x{:016X}", ttbr0, ttbr1);
    
    panic!("sync exception: EC=0x{:X} ESR=0x{:016X} ELR=0x{:016X} FAR=0x{:016X} TCR=0x{:016X} SCTLR=0x{:016X} SPSR=0x{:016X}", ec, esr, elr, far, tcr, sctlr, spsr);
}

fn exception_class_name(ec: u64) -> &'static str {
    match ec {
        0x00 => "Unknown",
        0x01 => "Trapped WFI/WFE",
        0x03 => "Trapped MCR/MRC (CP15)",
        0x04 => "Trapped MCRR/MRRC (CP15)",
        0x05 => "Trapped MCR/MRC (CP14)",
        0x06 => "Trapped LDC/STC (CP14)",
        0x07 => "Trapped access to SVE/SIMD/FP",
        0x0C => "Trapped MRRC (CP14)",
        0x0E => "Illegal Execution State",
        0x11 => "SVC in AArch32",
        0x15 => "SVC in AArch64",
        0x18 => "Trapped MSR/MRS/System (AArch64)",
        0x19 => "Trapped access to SVE",
        0x20 => "Instruction Abort (lower EL)",
        0x21 => "Instruction Abort (same EL)",
        0x22 => "PC alignment fault",
        0x24 => "Data Abort (lower EL)",
        0x25 => "Data Abort (same EL)",
        0x26 => "SP alignment fault",
        0x30 => "Breakpoint (lower EL)",
        0x31 => "Breakpoint (same EL)",
        0x32 => "Software Step (lower EL)",
        0x33 => "Software Step (same EL)",
        0x34 => "Watchpoint (lower EL)",
        0x35 => "Watchpoint (same EL)",
        0x38 => "BKPT in AArch32",
        0x3C => "BRK in AArch64",
        _ => "Reserved/Unknown",
    }
}

/// Try to handle a sync exception
///
/// # Arguments
/// * `esr` - Exception Syndrome Register value
/// * `elr` - Exception Link Register value
/// * `far` - Fault Address Register value
/// * `_tcr` - Translation Control Register value (unused)
/// * `_sctlr` - System Control Register value (unused)
/// * `_spsr` - Saved Program Status Register value (unused)
///
/// # Returns
/// * `1` if the exception was handled successfully
/// * `0` if the exception was not handled
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

    if ec == 0x15 {
        info!("WARNING: rust_sync_try_handle received EC=0x15 (SVC) - this should go to rust_svc_handler!");
    }

    if ec == 0x20 || ec == 0x24 {
        // Read TTBR0_EL1 directly from register (avoid scheduler lock in exception context)
        let ttbr0 = crate::arch::context::get_ttbr0() as u64;
        // Extract page table base address (lower 48 bits), mask off ASID in bits [63:48]
        let pt_base = (ttbr0 & 0x0000_FFFF_FFFF_FFFF) as usize;

        info!("arch/aarch64 page fault: far=0x{:016X} elr=0x{:016X} ttbr0=0x{:016X}", far, elr, ttbr0);

        if crate::memory::virtual_::handle_page_fault(pt_base, far as usize, esr) {
            info!("arch/aarch64 page fault handled");
            return 1;
        } else {
            info!("arch/aarch64 page fault NOT handled");
        }
    } else {
        // Not a page fault, pretend it was handled
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
                crate::debug!(
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
    info!("arch/aarch64 exc enter: ec=0x{:X} esr=0x{:016X} elr=0x{:016X} far=0x{:016X}", ec, esr, elr, far);
}

#[no_mangle]
pub extern "C" fn arch_exec_preflight(elr: u64, sp0: u64, ttbr0: u64) {
    crate::console::write_raw("arch_exec_preflight enter\n");
    info!("arch/aarch64 exec preflight enter");
    let spsr = crate::arch::context::get_spsr() as u64;
    let cel = crate::arch::context::get_current_el() as u64;
    let vbar = crate::arch::context::get_vbar() as u64;
    let tt0 = crate::arch::context::get_ttbr0() as u64;
    let tt1 = crate::arch::context::get_ttbr1() as u64;
    let sp_el0 = crate::arch::context::get_sp() as u64;
    
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

/// AArch64 中断控制器实现 (GIC 包装)
pub struct AArch64InterruptController;

impl InterruptController for AArch64InterruptController {
    fn init() {
        crate::drivers::gic::init();
    }

    fn enable_irq(irq: u32) {
        // 假设 GIC 驱动程序提供了 enable_irq 函数
        // 如果不存在，可能需要调用适当的函数
        // 暂时使用默认实现
        crate::drivers::gic::enable_irq(irq);
    }

    fn disable_irq(irq: u32) {
        crate::drivers::gic::disable_irq(irq);
    }

    fn ack_irq(irq: u32) {
        // 确认中断（写入 EOIR 寄存器）
        crate::drivers::gic::ack_irq(irq);
    }
}
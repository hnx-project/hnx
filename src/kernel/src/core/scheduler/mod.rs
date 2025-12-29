use crate::arch::common::mmu::MmuFlags;
use crate::console;
use crate::info;
use crate::memory::virtual_::map_in_pt;
use crate::process::task::{Task, TaskState};
use spin::Mutex;

static CURRENT: Mutex<Option<Task>> = Mutex::new(None);

pub fn run_task(task: Task) -> ! {
    {
        let mut cur = CURRENT.lock();
        *cur = Some(task);
    }
    info!("process running task");
    if let Some(t) = CURRENT.lock().as_ref() {
        if t.ttbr0_base != 0 {
            let base = t.ttbr0_base;
            let asid = t.asid;
            let sp0 = t.context.sp;
            let elr = t.entry_point;
            
            // Encode ASID in TTBR0_EL1[63:48] as per ARMv8 spec
            let ttbr0_with_asid = (base & 0x0000_FFFF_FFFF_FFFF) | ((asid as usize) << 48);
            
            info!(
                "process set EL0 ttbr0=0x{:016X} asid={} sp=0x{:016X} elr=0x{:016X}",
                ttbr0_with_asid, asid, sp0, elr
            );
            
            info!("=> About to call arch_do_exec - THIS NEVER RETURNS");
            
            // Use architecture-abstracted context switching
            use crate::arch::Context;

            crate::info!("=> Using architecture-abstracted context switching");
            crate::arch::context::exec_user(
                elr,
                sp0,
                base,  // page_table_base (without ASID)
                asid,
                (0, 0, 0, 0),  // args (a0, a1, a2, a8)
            );
        } else {
            let entry: fn() -> ! = unsafe { core::mem::transmute(t.entry_point) };
            entry()
        }
    }
    loop {
        crate::arch::cpu::wait_for_interrupt();
    }
}

pub fn run_task_with_args(task: Task, a0: usize, a1: usize, a2: usize, a8: usize) -> ! {
    {
        let mut cur = CURRENT.lock();
        *cur = Some(task);
    }
    info!("process running task with args");
    if let Some(t) = CURRENT.lock().as_ref() {
        if t.ttbr0_base != 0 {
            let base = t.ttbr0_base;
            let asid = t.asid;
            let sp0 = t.context.sp;
            let elr = t.entry_point;
            
            // Encode ASID in TTBR0_EL1[63:48] as per ARMv8 spec
            let ttbr0_with_asid = (base & 0x0000_FFFF_FFFF_FFFF) | ((asid as usize) << 48);
            
            info!(
                "process set EL0 ttbr0=0x{:016X} asid={} sp=0x{:016X} elr=0x{:016X}",
                ttbr0_with_asid, asid, sp0, elr
            );
            crate::arch::exec_preflight(elr);
            // Use architecture-abstracted context switching
            use crate::arch::Context;
            crate::arch::context::exec_user(
                elr,
                sp0,
                base,  // page_table_base (without ASID)
                asid,
                (a0, a1, a2, a8),  // args
            );
        } else {
            let entry: fn() -> ! = unsafe { core::mem::transmute(t.entry_point) };
            entry()
        }
    }
    loop {
        crate::arch::cpu::wait_for_interrupt();
    }
}

pub fn run() -> ! {
    crate::info!("scheduler started - HNX kernel successfully booted!");
    crate::console::write_raw("Scheduler started - HNX kernel successfully booted!\n");
    // For now, let's just loop waiting for interrupts
    loop {
        // For now, we just wait for interrupts
        // In a more complete implementation, this would implement proper scheduling
        crate::arch::cpu::wait_for_interrupt();
    }
}

pub fn on_tick() {
    crate::arch::timer::set_next_tick(500);
    
    // Update current process ticks
    let current_pid = current_pid();
    if current_pid != 0 {
        crate::process::increment_process_ticks(current_pid as usize);
    }
}

pub fn schedule_rr_tick() {
    on_tick();
    crate::process::on_rr_tick();
}

// Enhanced scheduler with priority-based scheduling
pub fn schedule_priority() {
    on_tick();
    
    // For now, we'll use a simple priority-based scheduling algorithm
    // In a more advanced implementation, we could use a multi-level feedback queue
    // or other sophisticated scheduling algorithms
    
    // Update current process state
    let current_pid = current_pid();
    if current_pid != 0 {
        // Set current process to ready state before rescheduling
        crate::process::set_process_state(current_pid as usize, crate::process::ProcState::Running);
    }
    
    // Trigger round-robin for now (could be enhanced with priority-based selection)
    crate::process::on_rr_tick();
}

pub fn current_pid() -> u64 {
    if let Some(ref t) = *CURRENT.lock() {
        t.id
    } else {
        0
    }
}

/// Get the current process's TTBR0 base address
///
/// Returns None if no process is currently running
pub fn current_ttbr0_base() -> Option<usize> {
    // Try to acquire lock without spinning to avoid deadlock in exception context
    if let Some(cur) = CURRENT.try_lock() {
        cur.as_ref().map(|t| t.ttbr0_base)
    } else {
        // Lock unavailable, return None
        crate::warn!("scheduler::current_ttbr0_base: lock unavailable, returning None");
        None
    }
}

pub fn exit_current() -> ! {
    {
        let mut cur = CURRENT.lock();
        *cur = None;
    }
    info!("process task exited");
    run()
}
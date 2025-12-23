use crate::arch::common::mmu::MmuFlags;
use crate::console;
use crate::info;
use crate::memory::virtual_::map_in_pt;
use crate::process::task::{Task, TaskState};
use spin::Mutex;
unsafe extern "C" {
    unsafe fn arch_do_exec(
        elr: usize,
        sp0: usize,
        ttbr0: usize,
        a0: usize,
        a1: usize,
        a2: usize,
        a8: usize,
    ) -> !;
}

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
            crate::arch::exec_preflight(elr);
            unsafe {
                arch_do_exec(elr, sp0, ttbr0_with_asid, 0, 0, 0, 0);
            }
        } else {
            let entry: fn() -> ! = unsafe { core::mem::transmute(t.entry_point) };
            entry()
        }
    }
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
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
            unsafe {
                arch_do_exec(elr, sp0, ttbr0_with_asid, a0, a1, a2, a8);
            }
        } else {
            let entry: fn() -> ! = unsafe { core::mem::transmute(t.entry_point) };
            entry()
        }
    }
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

pub fn run() -> ! {
    crate::info!("scheduler started - HNX kernel successfully booted!");
    crate::console::write_raw("Scheduler started - HNX kernel successfully booted!\n");
    // For now, let's just loop waiting for interrupts
    loop {
        // For now, we just wait for interrupts
        // In a more complete implementation, this would implement proper scheduling
        unsafe {
            core::arch::asm!("wfi");
        }
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

pub fn current_ttbr0_base() -> Option<usize> {
    CURRENT.lock().as_ref().map(|t| t.ttbr0_base)
}

pub fn exit_current() -> ! {
    {
        let mut cur = CURRENT.lock();
        *cur = None;
    }
    info!("process task exited");
    run()
}
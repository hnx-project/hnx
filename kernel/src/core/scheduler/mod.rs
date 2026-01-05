use crate::arch::common::mmu::MmuFlags;
use crate::console;
use crate::debug;
use crate::error;
use crate::info;
use crate::memory::virtual_::map_in_pt;
use crate::process::task::{Task, TaskState};
use shared::sync::mutex::Mutex;

static CURRENT: Mutex<Option<Task>> = Mutex::new(None);

pub fn run_task(task: Task) -> ! {
    {
        let mut cur = CURRENT.lock();
        *cur = Some(task);
    }
    info!("process running task");
    // Extract task fields while holding lock briefly
    let task_fields = {
        let cur = CURRENT.lock();
        cur.as_ref().map(|t| (t.ttbr0_base != 0, t.entry_point, t.context.sp, t.ttbr0_base, t.asid))
    };

    if let Some((has_user_space, entry_point, stack_pointer, ttbr0_base, asid)) = task_fields {
        if has_user_space {
            // Encode ASID in TTBR0_EL1[63:48] as per ARMv8 spec
            let ttbr0_with_asid = (ttbr0_base & 0x0000_FFFF_FFFF_FFFF) | ((asid as usize) << 48);

            info!(
                "process set EL0 ttbr0=0x{:016X} asid={} sp=0x{:016X} elr=0x{:016X}",
                ttbr0_with_asid, asid, stack_pointer, entry_point
            );

            info!("=> About to call arch_do_exec - THIS NEVER RETURNS");

            // Use architecture-abstracted context switching
            use crate::arch::Context;

            crate::info!("=> Using architecture-abstracted context switching");
            crate::arch::context::exec_user(
                entry_point,
                stack_pointer,
                ttbr0_base,  // page_table_base (without ASID)
                asid,
                (0, 0, 0, 0),  // args (a0, a1, a2, a8)
            );
        } else {
            let entry: fn() -> ! = unsafe { core::mem::transmute(entry_point) };
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
    // Use try_lock to avoid deadlock in exception context
    if let Some(cur) = CURRENT.try_lock() {
        cur.as_ref().map(|t| t.id).unwrap_or(0)
    } else {
        // Lock unavailable - might be in exception context
        crate::warn!("scheduler::current_pid: lock unavailable, returning 0");
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

/// Switch to the next ready process
///
/// This implements cooperative multitasking - the current process yields CPU
/// and we switch to the next process in the ready queue.
pub fn switch_to_next_process() -> ! {

    let current_pid = current_pid();
    crate::debug!("switch_to_next_process: current_pid={}", current_pid);

    // Save current process context before switching away
    let effective_pid = if current_pid != 0 {
        // Get current PC and SP from saved exception context
        let current_pc = crate::arch::context::get_elr();
        let current_sp = crate::arch::context::get_sp();
        crate::debug!("switch_to_next_process: saving context for PID {}: PC=0x{:X}, SP=0x{:X}",
                     current_pid, current_pc, current_sp);

        // Update PCB with current context using public API
        if crate::process::update_process_context(current_pid as u32, current_pc, current_sp) {
            crate::debug!("switch_to_next_process: updated PCB for PID {}", current_pid);
        } else {
            crate::error!("switch_to_next_process: failed to update PCB for PID {}", current_pid);
        }

        current_pid
    } else {
        // Lock was unavailable, but we know init is PID 1
        1
    };

    crate::debug!("switch_to_next_process: pushing PID {} to ready queue", effective_pid);
    crate::process::ready_queue_push(effective_pid as u32);

    // Get the next process from ready queue
    if let Some(mut next_pid) = crate::process::ready_queue_pop() {
        crate::debug!("switch_to_next_process: popped PID {}", next_pid);

        // Avoid switching to the same process if possible
        if next_pid as u64 == effective_pid {
            if let Some(another_pid) = crate::process::ready_queue_pop() {
                // Push the original back to queue
                crate::process::ready_queue_push(next_pid);
                next_pid = another_pid;
                crate::debug!("[SCHED] new next_pid={}", next_pid);
            }
        }
        crate::debug!("switch_to_next_process: switching to next_pid={}", next_pid);
        // Get process information for context switching
        if let Some((entry_point, stack_pointer, ttbr0_base, asid)) =
            crate::process::get_process_for_scheduling(next_pid)
        {
            info!("scheduler: switching from PID {} to PID {}", current_pid, next_pid);
            debug!("scheduler: entry=0x{:X} sp=0x{:X} ttbr0=0x{:X} asid={}",
                entry_point, stack_pointer, ttbr0_base, asid);

            // Update current task in scheduler
            {
                let mut cur = CURRENT.lock();
                *cur = Some(crate::process::Task {
                    id: next_pid as u64,
                    state: crate::process::TaskState::Running,
                    context: crate::process::TaskContext {
                        sp: stack_pointer,
                        pc: entry_point,
                    },
                    entry_point,
                    stack: 0..0, // Not used for user processes
                    ttbr0_base,
                    asid,
                });
            }

            // Perform architecture-specific context switch
            crate::debug!("[SCHED] About to call exec_user for PID {}", next_pid);
            use crate::arch::Context;
            crate::arch::context::exec_user(
                entry_point,
                stack_pointer,
                ttbr0_base,
                asid,
                (0, 0, 0, 0),  // No arguments for now
            );
        } else {
            error!("scheduler: PID {} not found in PCB table", next_pid);
            loop {
                crate::arch::cpu::wait_for_interrupt();
            }
        }
    } else {
        // No process ready, just idle
        debug!("scheduler: no ready processes, idling");
        loop {
            crate::arch::cpu::wait_for_interrupt();
        }
    }
}
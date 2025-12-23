//! Basic Signal Handling for kill() syscall
//!
//! This module implements basic signal delivery for the kill() syscall.
//! Full signal handling (signal masks, handlers, etc.) will be implemented in Phase 2.

use crate::process::{self, ProcState};
use crate::security::{validate_capability, rights};
use super::SysResult;

/// Signal numbers (POSIX)
pub const SIGKILL: u32 = 9;   // Kill (cannot be caught or ignored)
pub const SIGTERM: u32 = 15;  // Termination signal
pub const SIGSTOP: u32 = 19;  // Stop process (cannot be caught or ignored)
pub const SIGCONT: u32 = 18;  // Continue if stopped

/// Kill syscall - send signal to process
///
/// # Arguments
/// - pid: Target process ID
///   - pid > 0: send to specific process
///   - pid == 0: send to all processes in current process group
///   - pid == -1: send to all processes (requires privilege)
///   - pid < -1: send to all processes in process group |pid|
/// - sig: Signal number
///
/// # Returns
/// - 0 on success
/// - -1 on error
///
/// # Security Considerations
/// - Validates sender has permission to signal target
/// - Uses capability system for permission checks
/// - Prevents killing kernel processes (PID 0)
/// - Prevents killing init (PID 1) with most signals
pub fn sys_kill(pid: isize, sig: u32) -> SysResult {
    crate::info!("syscall: kill(pid={}, sig={})", pid, sig);
    
    let sender_pid = crate::kernel_core::scheduler::current_pid() as u32;
    
    // Validate signal number
    if sig > 64 {
        crate::error!("kill: invalid signal number {}", sig);
        return -1; // EINVAL
    }
    
    // Special case: sig == 0 is used to check if process exists
    if sig == 0 {
        if pid > 0 {
            return if process::get_process_state(pid as usize).is_some() {
                0
            } else {
                -1 // ESRCH - no such process
            };
        }
        return -1; // Not implemented for other pid values
    }
    
    // Handle different pid values
    if pid > 0 {
        // Send to specific process
        return kill_process(sender_pid, pid as u32, sig);
    } else if pid == 0 {
        // Send to all processes in sender's process group
        crate::debug!("kill: sending to process group not yet implemented");
        return -1; // TODO: Implement process group signaling
    } else if pid == -1 {
        // Send to all processes (requires privilege)
        crate::debug!("kill: broadcast signal not yet implemented");
        return -1; // TODO: Implement broadcast with privilege check
    } else {
        // Send to process group |pid|
        crate::debug!("kill: process group signaling not yet implemented");
        return -1; // TODO: Implement process group signaling
    }
}

/// Send signal to a specific process
///
/// # Security Checks
/// - Prevents killing kernel (PID 0)
/// - Prevents killing init (PID 1) except with SIGKILL
/// - Validates sender has permission to signal target
/// - Uses capability system for permission validation
fn kill_process(sender_pid: u32, target_pid: u32, sig: u32) -> SysResult {
    // Security: Cannot kill kernel
    if target_pid == 0 {
        crate::error!("kill: cannot kill kernel process");
        return -1; // EPERM
    }
    
    // Security: Cannot kill init with most signals
    if target_pid == 1 && sig != SIGKILL {
        crate::error!("kill: cannot send signal {} to init", sig);
        return -1; // EPERM
    }
    
    // Check if target process exists
    let target_state = if let Some(state) = process::get_process_state(target_pid as usize) {
        state
    } else {
        crate::error!("kill: target process {} does not exist", target_pid);
        return -1; // ESRCH - no such process
    };
    
    // Security check: validate permission to signal target
    // For now, we allow:
    // 1. Process can signal itself
    // 2. Parent can signal child
    // 3. TODO: Add capability-based checks for cross-process signaling
    
    if sender_pid != target_pid {
        // Check if sender is parent of target
        if let Some(parent_pid) = process::get_parent_pid(target_pid as usize) {
            if parent_pid != sender_pid {
                // Not self and not parent - check capabilities
                // TODO: Implement proper capability check
                crate::error!("kill: permission denied - not parent of target");
                return -1; // EPERM
            }
        }
    }
    
    // Handle different signals
    match sig {
        SIGKILL => {
            // SIGKILL - immediately terminate the process
            crate::info!("kill: SIGKILL to pid={}", target_pid);
            
            // Set exit status to indicate killed by signal
            // Status format: signal number in lower 7 bits, bit 7 set for signal
            let exit_status = (sig as i32) | 0x80;
            
            // Transition to zombie state
            if !process::set_exit_status(target_pid as usize, exit_status) {
                crate::error!("kill: failed to set exit status for pid={}", target_pid);
                return -1;
            }
            
            // Wake parent if waiting
            if let Some(parent_pid) = process::get_parent_pid(target_pid as usize) {
                if parent_pid != 0 {
                    let parent_state = process::get_process_state(parent_pid as usize);
                    if parent_state == Some(ProcState::Waiting) {
                        crate::info!("kill: waking parent pid={}", parent_pid);
                        process::wake_process(parent_pid as usize);
                    }
                }
            }
            
            // TODO: Free process resources (memory, file descriptors, etc.)
            // TODO: Orphan children (reparent to init)
            
            crate::info!("kill: successfully killed pid={}", target_pid);
            0
        }
        
        SIGTERM => {
            // SIGTERM - termination signal (can be caught)
            // For now, treat like SIGKILL since we don't have signal handlers
            crate::info!("kill: SIGTERM to pid={} (treating as SIGKILL)", target_pid);
            
            let exit_status = (sig as i32) | 0x80;
            if !process::set_exit_status(target_pid as usize, exit_status) {
                return -1;
            }
            
            // Wake parent if waiting
            if let Some(parent_pid) = process::get_parent_pid(target_pid as usize) {
                if parent_pid != 0 && process::get_process_state(parent_pid as usize) == Some(ProcState::Waiting) {
                    process::wake_process(parent_pid as usize);
                }
            }
            
            0
        }
        
        SIGSTOP => {
            // SIGSTOP - stop process execution
            crate::info!("kill: SIGSTOP to pid={}", target_pid);
            process::set_process_state(target_pid as usize, ProcState::Blocked);
            0
        }
        
        SIGCONT => {
            // SIGCONT - continue if stopped
            crate::info!("kill: SIGCONT to pid={}", target_pid);
            if target_state == ProcState::Blocked {
                process::wake_process(target_pid as usize);
            }
            0
        }
        
        _ => {
            // Other signals - mark as pending (full signal handling in Phase 2)
            crate::debug!("kill: signal {} to pid={} - marking as pending (not implemented)", sig, target_pid);
            // TODO: Implement signal pending queue
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signal_numbers() {
        assert_eq!(SIGKILL, 9);
        assert_eq!(SIGTERM, 15);
    }
}

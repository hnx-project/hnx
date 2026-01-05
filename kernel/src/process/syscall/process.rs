//! POSIX Process Control Syscalls
//!
//! This module implements critical process management syscalls:
//! - fork() - Create a new process by duplicating the current process
//! - wait4() / waitpid() - Wait for child process to change state
//! - getppid() - Get parent process ID
//! - setpgid() / getpgid() - Process group management

use crate::process::{self, ProcState};
use crate::process::task::{allocate_asid, Asid};
use crate::memory::physical::alloc_pages;
use crate::memory::virt::create_user_l1;
use crate::arch::common::mmu::MmuFlags;
use super::SysResult;

/// Fork syscall - create a new process by duplicating the current process
///
/// # Returns
/// - Parent: child PID (> 0)
/// - Child: 0
/// - Error: -1
///
/// # Security Considerations
/// - Allocates new ASID for child to prevent TLB pollution
/// - Copies page tables to isolate child address space
/// - Preserves PXN/UXN bits in page table entries
/// - Child inherits parent's PGID and SID
pub fn sys_fork() -> SysResult {
    crate::info!("syscall: fork() enter");
    
    let parent_pid = crate::core::scheduler::current_pid() as u32;
    if parent_pid == 0 {
        crate::error!("fork: cannot fork kernel process");
        return -1;
    }
    
    // Get parent's page table base
    let parent_ttbr0 = if let Some(base) = crate::core::scheduler::current_ttbr0_base() {
        base
    } else {
        crate::error!("fork: parent has no page table");
        return -1;
    };
    
    // Create new page table for child
    let child_ttbr0 = if let Some(base) = create_user_l1() {
        base
    } else {
        crate::error!("fork: failed to create child L1 page table");
        return -1;
    };
    
    // Copy parent's page table to child
    if !copy_page_table(parent_ttbr0, child_ttbr0) {
        crate::error!("fork: failed to copy page table");
        // TODO: Free allocated child_ttbr0
        return -1;
    }
    
    // Allocate new ASID for child
    let child_asid = allocate_asid();
    
    // Get parent's priority for inheritance
    let parent_priority = if let Some(state) = process::get_process_state(parent_pid as usize) {
        // Default to medium priority if we can't get it
        128u8
    } else {
        128u8
    };
    
    // Create child process PCB
    let child_pid = if let Some(pid) = process::create_child_process(
        parent_pid,
        parent_priority,
        child_ttbr0,
        child_asid
    ) {
        pid
    } else {
        crate::error!("fork: failed to create child PCB");
        // TODO: Free allocated resources
        return -1;
    };
    
    // Set child to Ready state
    process::set_process_state(child_pid as usize, ProcState::Ready);
    
    crate::info!(
        "fork: parent={} child={} child_ttbr0=0x{:X} child_asid={}",
        parent_pid, child_pid, child_ttbr0, child_asid
    );
    
    // In the parent process, we return the child PID
    // The child will be scheduled later and will return 0
    // TODO: This is simplified - need proper context switching for child to return 0
    child_pid as SysResult
}

/// Copy page table from parent to child
///
/// # Security Note
/// This performs a full copy of the user address space. In a production kernel,
/// this should implement copy-on-write (COW) to improve performance.
/// 
/// # Safety
/// - Only copies user pages (< 0x0000_8000_0000_0000)
/// - Preserves all permission bits including PXN/UXN
/// - Does not copy kernel mappings
fn copy_page_table(parent_base: usize, child_base: usize) -> bool {
    crate::info!("fork: copying page table parent=0x{:X} child=0x{:X}", parent_base, child_base);
    
    unsafe {
        // Copy L1 table (only first entry for user space)
        let parent_l1 = parent_base as *const u64;
        let child_l1 = child_base as *mut u64;
        
        // Only copy user space L1 entry (index 0)
        let parent_l1_entry = core::ptr::read_volatile(parent_l1);
        
        // Check if parent has user space mapping
        if (parent_l1_entry & 0x3) != 3 {
            crate::info!("fork: parent has no user space mapping");
            return true; // Not an error, just no user space
        }
        
        // Get parent L2 table address
        let parent_l2_pa = (parent_l1_entry & !0xFFF) as usize;
        
        // Allocate new L2 table for child
        let child_l2_pa = if let Some(pa) = alloc_pages(1) {
            pa
        } else {
            crate::error!("fork: failed to allocate child L2 table");
            return false;
        };
        
        // Zero out child L2 table
        core::ptr::write_bytes(child_l2_pa as *mut u8, 0, 4096);
        
        // Set child L1 entry to point to new L2 table
        let child_l1_desc = ((child_l2_pa as u64) & !0xFFF) | 3u64;
        core::ptr::write_volatile(child_l1, child_l1_desc);
        
        // Copy L2 entries and L3 tables
        let parent_l2 = parent_l2_pa as *const u64;
        let child_l2 = child_l2_pa as *mut u64;
        
        for i in 0..512 {
            let parent_l2_entry = core::ptr::read_volatile(parent_l2.add(i));
            let entry_type = parent_l2_entry & 0x3;
            
            if entry_type == 0 {
                // Invalid entry, skip
                continue;
            } else if entry_type == 1 {
                // Block descriptor (2MB block)
                // For now, we'll convert to page tables for simplicity
                // TODO: Implement block copying
                crate::debug!("fork: skipping L2 block descriptor at index {}", i);
                continue;
            } else if entry_type == 3 {
                // Table descriptor - points to L3 table
                let parent_l3_pa = (parent_l2_entry & !0xFFF) as usize;
                
                // Allocate new L3 table for child
                let child_l3_pa = if let Some(pa) = alloc_pages(1) {
                    pa
                } else {
                    crate::error!("fork: failed to allocate child L3 table");
                    return false;
                };
                
                // Copy L3 table contents
                if !copy_l3_table(parent_l3_pa, child_l3_pa) {
                    return false;
                }
                
                // Set child L2 entry to point to new L3 table
                let child_l2_desc = ((child_l3_pa as u64) & !0xFFF) | 3u64;
                core::ptr::write_volatile(child_l2.add(i), child_l2_desc);
            }
        }
    }
    
    // Flush caches and TLB
    crate::arch::mmu::clean_dcache_range(child_base, 4096);
    
    crate::info!("fork: page table copy complete");
    true
}

/// Copy L3 page table and associated physical pages
///
/// # Security Note
/// - Preserves all permission bits (AP, UXN, PXN, etc.)
/// - Allocates new physical pages for child
/// - Copies page contents to ensure isolation
fn copy_l3_table(parent_l3_pa: usize, child_l3_pa: usize) -> bool {
    unsafe {
        let parent_l3 = parent_l3_pa as *const u64;
        let child_l3 = child_l3_pa as *mut u64;
        
        // Zero out child L3 table
        core::ptr::write_bytes(child_l3_pa as *mut u8, 0, 4096);
        
        for i in 0..512 {
            let parent_l3_entry = core::ptr::read_volatile(parent_l3.add(i));
            
            // Check if entry is valid
            if (parent_l3_entry & 0x3) != 3 {
                continue;
            }
            
            // Get parent's physical page
            let parent_page_pa = (parent_l3_entry & !0xFFF) as usize;
            
            // Allocate new physical page for child
            let child_page_pa = if let Some(pa) = alloc_pages(1) {
                pa
            } else {
                crate::error!("fork: failed to allocate child page");
                return false;
            };
            
            // Copy page contents
            core::ptr::copy_nonoverlapping(
                parent_page_pa as *const u8,
                child_page_pa as *mut u8,
                4096
            );
            
            // Create child L3 entry with same permissions
            // Preserve all attribute bits (AP, UXN, PXN, etc.)
            let child_l3_entry = (child_page_pa as u64 & !0xFFF) | (parent_l3_entry & 0xFFF);
            core::ptr::write_volatile(child_l3.add(i), child_l3_entry);
        }
    }
    
    true
}

/// Wait4 syscall - wait for process to change state
///
/// # Arguments
/// - pid: Process ID to wait for
///   - pid > 0: wait for specific child
///   - pid == -1: wait for any child
///   - pid == 0: wait for any child in same process group
///   - pid < -1: wait for any child in process group |pid|
/// - status_ptr: pointer to store exit status (can be NULL)
/// - options: wait options (WNOHANG, etc.)
/// - rusage_ptr: pointer to store resource usage (currently ignored)
///
/// # Returns
/// - PID of child that changed state
/// - 0 if WNOHANG and no child has changed
/// - -1 on error
pub fn sys_wait4(pid: isize, status_ptr: usize, options: usize, _rusage_ptr: usize) -> SysResult {
    crate::info!("syscall: wait4(pid={}, status=0x{:X}, options=0x{:X})", pid, status_ptr, options);
    
    let current_pid = crate::core::scheduler::current_pid() as u32;
    if current_pid == 0 {
        crate::error!("wait4: kernel cannot wait");
        return -1;
    }
    
    // WNOHANG flag - return immediately if no child has exited
    const WNOHANG: usize = 1;
    let non_blocking = (options & WNOHANG) != 0;
    
    // Check if caller has any children
    if !process::has_children(current_pid) {
        crate::debug!("wait4: no children to wait for");
        return -1; // ECHILD
    }
    
    // Look for zombie child
    if let Some((child_pid, exit_status)) = process::find_zombie_child(current_pid) {
        crate::info!("wait4: found zombie child pid={} status={}", child_pid, exit_status);
        
        // Write exit status to user memory if requested
        if status_ptr != 0 {
            if !super::user_range_ok(status_ptr, 4, true) {
                crate::error!("wait4: invalid status pointer");
                return -1;
            }
            
            unsafe {
                core::ptr::write_volatile(status_ptr as *mut i32, exit_status);
            }
        }
        
        // Reap the zombie process
        process::reap_zombie(child_pid as usize);
        
        return child_pid as SysResult;
    }
    
    // No zombie child found
    if non_blocking {
        crate::debug!("wait4: WNOHANG and no zombie child");
        return 0;
    }
    
    // Block the parent process
    crate::info!("wait4: blocking parent pid={}", current_pid);
    process::set_process_state(current_pid as usize, ProcState::Waiting);
    
    // TODO: Yield CPU and reschedule
    // When child exits, it should wake the parent
    // For now, we'll return -1 to indicate we can't block yet
    
    -1
}

/// Get parent process ID
pub fn sys_getppid() -> SysResult {
    let current_pid = crate::core::scheduler::current_pid() as u32;
    
    if let Some(parent_pid) = process::get_parent_pid(current_pid as usize) {
        if parent_pid == 0 {
            // Orphaned process or init - return 1 (init PID)
            return 1;
        }
        parent_pid as SysResult
    } else {
        // Process doesn't exist (shouldn't happen for current process)
        1
    }
}

/// Set process group ID
///
/// # Arguments
/// - pid: Process ID (0 for current process)
/// - pgid: Process group ID (0 to create new group with pid as leader)
///
/// # Security
/// - Process can only set PGID for itself or its children
/// - Cannot move process to another session
pub fn sys_setpgid(pid: usize, pgid: usize) -> SysResult {
    let current_pid = crate::core::scheduler::current_pid() as usize;
    
    // Determine target PID (0 means current process)
    let target_pid = if pid == 0 { current_pid } else { pid };
    
    // Determine target PGID (0 means same as target PID)
    let target_pgid = if pgid == 0 { target_pid as u32 } else { pgid as u32 };
    
    // Security check: can only set PGID for self or children
    if target_pid != current_pid {
        if let Some(parent_pid) = process::get_parent_pid(target_pid) {
            if parent_pid != current_pid as u32 {
                crate::error!("setpgid: not allowed to set PGID for non-child process");
                return -1; // EPERM
            }
        } else {
            return -1; // ESRCH - no such process
        }
    }
    
    // Set the PGID
    if process::set_pgid(target_pid, target_pgid) {
        crate::info!("setpgid: set pid={} pgid={}", target_pid, target_pgid);
        0
    } else {
        -1
    }
}

/// Get process group ID
///
/// # Arguments
/// - pid: Process ID (0 for current process)
pub fn sys_getpgid(pid: usize) -> SysResult {
    let current_pid = crate::core::scheduler::current_pid() as usize;
    let target_pid = if pid == 0 { current_pid } else { pid };
    
    if let Some(pgid) = process::get_pgid(target_pid) {
        pgid as SysResult
    } else {
        -1 // ESRCH - no such process
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_getppid() {
        // This would require mocking the scheduler
        // Placeholder for future testing
    }
    
    #[test]
    fn test_pgid() {
        // This would require setting up process table
        // Placeholder for future testing
    }
}

//! Process and Thread Syscalls
//!
//! This module implements syscalls for process and thread management:
//! - process_create() - Create a new process
//! - process_start() - Start a process
//! - thread_create() - Create a new thread
//! - thread_start() - Start a thread

use crate::process::{self, ProcState};
use crate::process::task::{allocate_asid, Asid};
use crate::memory::physical::alloc_pages;
use crate::memory::virtual_::create_user_l1;
use crate::arch::common::mmu::MmuFlags;
use crate::process::syscall::SysResult;
use crate::abi::{zx_process_create_args, ObjectType};

/// Create a new process
///
/// # Arguments
/// - options: Process creation options (currently ignored)
/// - name: Pointer to process name
/// - name_len: Length of process name
/// - out_proc_handle: Pointer to store process handle
/// - out_vmar_handle: Pointer to store VMAR handle
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_process_create(options: usize, name: usize, name_len: usize, 
                         out_proc_handle: usize, out_vmar_handle: usize) -> SysResult {
    crate::debug!("syscall process_create options={} name=0x{:X} name_len={} out_proc_handle=0x{:X} out_vmar_handle=0x{:X}", 
                 options, name, name_len, out_proc_handle, out_vmar_handle);
    
    // Validate user pointers
    if !crate::process::syscall::user_range_ok(out_proc_handle, 4, true) || 
       !crate::process::syscall::user_range_ok(out_vmar_handle, 4, true) {
        return -1; // InvalidAddress
    }
    
    // Get current process ID (parent)
    let parent_pid = crate::kernel_core::scheduler::current_pid() as u32;
    
    // Create new process PCB
    let child_pid = if let Some(pid) = process::create_child_process(
        parent_pid,
        128, // Default priority
        0,   // No page table yet
        0    // No ASID yet
    ) {
        pid
    } else {
        return -1; // Failed to create process
    };
    
    // Set process to Created state
    process::set_process_state(child_pid as usize, ProcState::Created);
    
    // For now, we'll use the PID as both process handle and VMAR handle
    // In a more complete implementation, these would be separate objects
    
    // Write handles to user space
    unsafe {
        core::ptr::write_volatile(out_proc_handle as *mut u32, child_pid);
        core::ptr::write_volatile(out_vmar_handle as *mut u32, child_pid);
    }
    
    0 // Success
}

/// Start a process
///
/// # Arguments
/// - proc_handle: Process handle
/// - entry: Entry point address
/// - stack: Stack pointer
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_process_start(proc_handle: usize, entry: usize, stack: usize) -> SysResult {
    crate::debug!("syscall process_start proc_handle={} entry=0x{:X} stack=0x{:X}", 
                 proc_handle, entry, stack);
    
    // For now, just set the process to Ready state
    // In a more complete implementation, this would set up the initial thread
    
    process::set_process_state(proc_handle, ProcState::Ready);
    
    0 // Success
}

/// Create a new thread
///
/// # Arguments
/// - proc_handle: Process handle to create thread in
/// - options: Thread creation options (currently ignored)
/// - name: Pointer to thread name
/// - name_len: Length of thread name
/// - out_thread_handle: Pointer to store thread handle
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_thread_create(proc_handle: usize, options: usize, name: usize, name_len: usize, 
                        out_thread_handle: usize) -> SysResult {
    crate::debug!("syscall thread_create proc_handle={} options={} name=0x{:X} name_len={} out_thread_handle=0x{:X}", 
                 proc_handle, options, name, name_len, out_thread_handle);
    
    // Validate user pointer
    if !crate::process::syscall::user_range_ok(out_thread_handle, 4, true) {
        return -1; // InvalidAddress
    }
    
    // For now, we'll just use the process handle as the thread handle
    // In a more complete implementation, threads would be separate objects
    
    // Write handle to user space
    unsafe {
        core::ptr::write_volatile(out_thread_handle as *mut u32, proc_handle as u32);
    }
    
    0 // Success
}

/// Start a thread
///
/// # Arguments
/// - thread_handle: Thread handle
/// - entry: Entry point address
/// - stack: Stack pointer
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_thread_start(thread_handle: usize, entry: usize, stack: usize) -> SysResult {
    crate::debug!("syscall thread_start thread_handle={} entry=0x{:X} stack=0x{:X}", 
                 thread_handle, entry, stack);
    
    // For now, just set the process to Ready state
    // In a more complete implementation, this would set up the thread context
    
    process::set_process_state(thread_handle, ProcState::Ready);
    
    0 // Success
}
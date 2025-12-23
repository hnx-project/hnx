//! Channel Syscalls
//!
//! This module implements syscalls for channel operations:
//! - channel_create() - Create a new channel pair
//! - channel_write() - Write a message to a channel
//! - channel_read() - Read a message from a channel

use crate::kernel_core::ipc::{Endpoint, endpoint_create, endpoint_send_sync, endpoint_recv_sync, IpcMessage, IpcResponse, EndpointCapabilities};
use crate::process::syscall::{SysResult, user_range_ok, copy_from_user, copy_to_user};
use crate::abi::{zx_channel_create_args, ObjectType};

/// Create a new channel pair
///
/// # Arguments
/// - options: Channel creation options (currently ignored)
/// - out0: Pointer to store first channel handle
/// - out1: Pointer to store second channel handle
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_channel_create(options: usize, out0: usize, out1: usize) -> SysResult {
    crate::debug!("syscall channel_create options={} out0=0x{:X} out1=0x{:X}", options, out0, out1);
    
    // Validate user pointers
    if !user_range_ok(out0, 4, true) || !user_range_ok(out1, 4, true) {
        return -1; // InvalidAddress
    }
    
    // Create two endpoints for the channel pair
    let ep0 = match endpoint_create(EndpointCapabilities { 
        read: true, 
        write: true, 
        admin: true 
    }) {
        Ok(ep) => ep,
        Err(_) => return -1, // Failed to create endpoint
    };
    
    let ep1 = match endpoint_create(EndpointCapabilities { 
        read: true, 
        write: true, 
        admin: true 
    }) {
        Ok(ep) => ep,
        Err(_) => return -1, // Failed to create endpoint
    };
    
    // Get current process ID
    let pid = crate::kernel_core::scheduler::current_pid() as usize;
    
    // Allocate handles for both endpoints
    let handle0 = crate::process::syscall::caps_alloc_handle(pid, ep0, 0x7); // SEND|RECV|MANAGE
    let handle1 = crate::process::syscall::caps_alloc_handle(pid, ep1, 0x7); // SEND|RECV|MANAGE
    
    if handle0 == usize::MAX || handle1 == usize::MAX {
        return -1; // Failed to allocate handles
    }
    
    // Write handles to user space
    unsafe {
        core::ptr::write_volatile(out0 as *mut u32, handle0 as u32);
        core::ptr::write_volatile(out1 as *mut u32, handle1 as u32);
    }
    
    0 // Success
}

/// Write a message to a channel
///
/// # Arguments
/// - handle: Channel handle to write to
/// - options: Write options (currently ignored)
/// - bytes: Pointer to message data
/// - num_bytes: Size of message data
/// - handles: Pointer to handle data (currently ignored)
/// - num_handles: Number of handles (currently ignored)
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_channel_write(handle: usize, options: usize, bytes: usize, num_bytes: usize, 
                       handles: usize, num_handles: usize) -> SysResult {
    crate::debug!("syscall channel_write handle={} options={} bytes=0x{:X} num_bytes={} handles=0x{:X} num_handles={}", 
                 handle, options, bytes, num_bytes, handles, num_handles);
    
    // Validate user buffer
    if !user_range_ok(bytes, num_bytes, false) {
        return -1; // InvalidAddress
    }
    
    // Get current process ID
    let pid = crate::kernel_core::scheduler::current_pid() as usize;
    
    // Lookup the endpoint handle
    let cap = match crate::process::syscall::caps_lookup(pid, handle) {
        Some(cap) => cap,
        None => return -1, // Invalid handle
    };
    
    // Validate capability rights
    if (cap.rights & 0x1) == 0 { // SEND right
        return -1; // Permission denied
    }
    
    // Copy message data from user space
    let mut data = [0u8; 256];
    let copy_len = core::cmp::min(num_bytes, data.len());
    let n = copy_from_user(bytes, copy_len, &mut data);
    if n == 0 {
        return -1; // Failed to copy data
    }
    
    // Send message to endpoint
    let msg = IpcMessage {
        src_pid: pid as u32,
        dst_epid: cap.ep_id,
        op: 0,
        priority: crate::kernel_core::ipc::Priority::Normal,
        data_len: n,
        data,
        timestamp: crate::arch::timer::now_us(),
    };
    
    match endpoint_send_sync(cap.ep_id, msg) {
        Ok(_) => 0,  // Success
        Err(_) => -1, // Failed to send
    }
}

/// Read a message from a channel
///
/// # Arguments
/// - handle: Channel handle to read from
/// - options: Read options (currently ignored)
/// - bytes: Pointer to buffer for message data
/// - handles: Pointer to buffer for handle data (currently ignored)
/// - num_bytes: Size of bytes buffer
/// - num_handles: Size of handles buffer (currently ignored)
/// - actual_bytes: Pointer to store actual bytes read
/// - actual_handles: Pointer to store actual handles read (currently ignored)
///
/// # Returns
/// - 0 on success
/// - Negative error code on failure
pub fn sys_channel_read(handle: usize, options: usize, bytes: usize, handles: usize,
                       num_bytes: usize, num_handles: usize, actual_bytes: usize, actual_handles: usize) -> SysResult {
    crate::debug!("syscall channel_read handle={} options={} bytes=0x{:X} handles=0x{:X} num_bytes={} num_handles={} actual_bytes=0x{:X} actual_handles=0x{:X}", 
                 handle, options, bytes, handles, num_bytes, num_handles, actual_bytes, actual_handles);
    
    // Validate user buffers
    if !user_range_ok(bytes, num_bytes, true) || !user_range_ok(actual_bytes, 4, true) {
        return -1; // InvalidAddress
    }
    
    // Get current process ID
    let pid = crate::kernel_core::scheduler::current_pid() as usize;
    
    // Lookup the endpoint handle
    let cap = match crate::process::syscall::caps_lookup(pid, handle) {
        Some(cap) => cap,
        None => return -1, // Invalid handle
    };
    
    // Validate capability rights
    if (cap.rights & 0x2) == 0 { // RECV right
        return -1; // Permission denied
    }
    
    // Receive message from endpoint
    let msg = match endpoint_recv_sync(cap.ep_id, None) {
        Ok(msg) => msg,
        Err(_) => return -1, // Failed to receive
    };
    
    // Copy message data to user space
    let copy_len = core::cmp::min(msg.data_len, num_bytes);
    let n = copy_to_user(bytes, &msg.data[..copy_len]);
    
    // Write actual bytes count to user space
    if user_range_ok(actual_bytes, 4, true) {
        unsafe {
            core::ptr::write_volatile(actual_bytes as *mut usize, n);
        }
    }
    
    // Write actual handles count to user space (always 0 for now)
    if actual_handles != 0 && user_range_ok(actual_handles, 4, true) {
        unsafe {
            core::ptr::write_volatile(actual_handles as *mut usize, 0);
        }
    }
    
    0 // Success
}
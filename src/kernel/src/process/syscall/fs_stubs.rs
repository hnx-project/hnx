//! File System Syscall Stubs - IPC Delegation
//!
//! These syscalls delegate to the VFS user space service via IPC.
//! They replace the old direct kernel VFS implementation.

use crate::ipc_services::{ServiceOp, ipc_delegate};
use hnx_abi::SysResult;

// VFS service endpoint ID (well-known)
const VFS_EPID: u32 = 1;

/// Copy a string from user space
fn copy_string_from_user(user_ptr: usize, max_len: usize) -> Result<heapless::String<256>, ()> {
    use heapless::String;
    
    if !super::user_range_ok(user_ptr, max_len, false) {
        return Err(());
    }
    
    let mut s = String::new();
    unsafe {
        for i in 0..max_len {
            let byte = core::ptr::read((user_ptr + i) as *const u8);
            if byte == 0 {
                break;
            }
            if s.push(byte as char).is_err() {
                break;
            }
        }
    }
    Ok(s)
}

/// Open a file - Delegate to VFS service
///
/// # Arguments
/// * `path_user` - User space pointer to path string
/// * `flags` - Open flags (O_RDONLY, O_WRONLY, O_RDWR, etc.)
/// * `mode` - File mode for creation
///
/// # Returns
/// * File descriptor (>= 0) on success
/// * Negative errno on error
pub fn sys_open(path_user: usize, flags: u32, _mode: u32) -> SysResult {
    crate::debug!("sys_open: path_ptr=0x{:x} flags=0x{:x}", path_user, flags);
    
    // Copy path from user space
    let path = match copy_string_from_user(path_user, 256) {
        Ok(p) => p,
        Err(_) => {
            crate::warn!("sys_open: invalid user pointer");
            return -14; // -EFAULT
        }
    };
    
    crate::debug!("sys_open: delegating to VFS service for path '{}'", path.as_str());
    
    // Delegate to VFS service
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsOpen,
        |req| req.with_str(path.as_str()).with_u32(flags)
    )
}

/// Read from file - Delegate to VFS service
pub fn sys_read(fd: usize, buf_user: usize, len: usize) -> SysResult {
    crate::debug!("sys_read: fd={} buf=0x{:x} len={}", fd, buf_user, len);
    
    if !super::user_range_ok(buf_user, len, true) {
        return -14; // -EFAULT
    }
    
    // For now, delegate to VFS service
    // Note: Large reads should use shared memory instead of message passing
    let read_len = len.min(256); // Limit to message size
    
    crate::debug!("sys_read: delegating to VFS service");
    
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsRead,
        |req| req.with_u32(fd as u32).with_u32(read_len as u32)
    )
}

/// Write to file - Delegate to VFS service
pub fn sys_write(fd: usize, buf_user: usize, len: usize) -> SysResult {
    crate::info!("sys_write: fd={} buf=0x{:x} len={}", fd, buf_user, len);

    if !super::user_range_ok(buf_user, len, false) {
        crate::info!("sys_write: user_range_ok failed for buf=0x{:x} len={}", buf_user, len);
        return -14; // -EFAULT
    }
    
    // For now, delegate to VFS service
    // Note: Large writes should use shared memory
    let write_len = len.min(200); // Leave room for fd in message
    
    // Copy data from user space
    crate::info!("sys_write: copying {} bytes from 0x{:X}", write_len, buf_user);
    let mut data = [0u8; 200];
    unsafe {
        core::ptr::copy_nonoverlapping(
            buf_user as *const u8,
            data.as_mut_ptr(),
            write_len
        );
    }
    crate::info!("sys_write: copy completed, first byte=0x{:X}", data[0]);
    
    crate::info!("sys_write: delegating to VFS service, fd={}, len={}", fd, write_len);

    // Delegate to VFS service via IPC
    let ipc_result = ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsWrite,
        |req| req.with_u32(fd as u32).with_str(core::str::from_utf8(&data[..write_len]).unwrap_or("[invalid utf8]"))
    );

    // If IPC succeeds, return the result
    if ipc_result >= 0 {
        return ipc_result;
    }

    // IPC failed, fall back to console output
    crate::warn!("sys_write: IPC delegation failed ({}), falling back to console output", ipc_result);
    use crate::console::write_raw;
    let s = core::str::from_utf8(&data[..write_len]).unwrap_or("[invalid utf8]");
    write_raw(s);
    write_raw("\n");
    write_len as isize
}

/// Close file - Delegate to VFS service
pub fn sys_close(fd: usize) -> SysResult {
    crate::debug!("sys_close: fd={}", fd);
    
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsClose,
        |req| req.with_u32(fd as u32)
    )
}

/// Create file - Delegate to VFS service
pub fn sys_creat(path_user: usize, mode: u32) -> SysResult {
    crate::debug!("sys_creat: path_ptr=0x{:x} mode=0x{:x}", path_user, mode);
    
    let path = match copy_string_from_user(path_user, 256) {
        Ok(p) => p,
        Err(_) => return -14, // -EFAULT
    };
    
    crate::debug!("sys_creat: delegating to VFS service for path '{}'", path.as_str());
    
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsCreate,
        |req| req.with_str(path.as_str()).with_u32(mode)
    )
}

/// Unlink file - Delegate to VFS service
pub fn sys_unlink(path_user: usize) -> SysResult {
    crate::debug!("sys_unlink: path_ptr=0x{:x}", path_user);
    
    let path = match copy_string_from_user(path_user, 256) {
        Ok(p) => p,
        Err(_) => return -14, // -EFAULT
    };
    
    crate::debug!("sys_unlink: delegating to VFS service for path '{}'", path.as_str());
    
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsUnlink,
        |req| req.with_str(path.as_str())
    )
}

/// Create directory - Delegate to VFS service
pub fn sys_mkdir(path_user: usize, mode: u32) -> SysResult {
    crate::debug!("sys_mkdir: path_ptr=0x{:x} mode=0x{:x}", path_user, mode);
    
    let path = match copy_string_from_user(path_user, 256) {
        Ok(p) => p,
        Err(_) => return -14, // -EFAULT
    };
    
    crate::debug!("sys_mkdir: delegating to VFS service for path '{}'", path.as_str());
    
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsMkdir,
        |req| req.with_str(path.as_str()).with_u32(mode)
    )
}

/// Remove directory - Delegate to VFS service
pub fn sys_rmdir(path_user: usize) -> SysResult {
    crate::debug!("sys_rmdir: path_ptr=0x{:x}", path_user);
    
    let path = match copy_string_from_user(path_user, 256) {
        Ok(p) => p,
        Err(_) => return -14, // -EFAULT
    };
    
    crate::debug!("sys_rmdir: delegating to VFS service for path '{}'", path.as_str());
    
    ipc_delegate(
        VFS_EPID,
        ServiceOp::VfsRmdir,
        |req| req.with_str(path.as_str())
    )
}
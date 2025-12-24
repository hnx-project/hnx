//! Network syscall implementations
//!
//! This module implements the network syscalls that redirect to IPC
//! instead of direct kernel calls.

extern crate alloc;

// use crate::network::manager::get_network_manager;
use crate::process::syscall::SysResult;

/// Network syscall error codes
#[derive(Debug)]
pub enum SyscallError {
    InvalidArgument,
    InvalidAddress,
    OutOfMemory,
    PermissionDenied,
    WouldBlock,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    AddressInUse,
    AddressNotAvailable,
    Timeout,
    UnknownError,
}

impl SyscallError {
    pub fn to_errno(&self) -> i32 {
        match self {
            SyscallError::InvalidArgument => 22,     // EINVAL
            SyscallError::InvalidAddress => 14,      // EFAULT
            SyscallError::OutOfMemory => 12,         // ENOMEM
            SyscallError::PermissionDenied => 13,    // EACCES
            SyscallError::WouldBlock => 11,          // EAGAIN
            SyscallError::ConnectionRefused => 111,  // ECONNREFUSED
            SyscallError::ConnectionReset => 104,    // ECONNRESET
            SyscallError::HostUnreachable => 113,    // EHOSTUNREACH
            SyscallError::NetworkUnreachable => 101, // ENETUNREACH
            SyscallError::AddressInUse => 98,        // EADDRINUSE
            SyscallError::AddressNotAvailable => 99, // EADDRNOTAVAIL
            SyscallError::Timeout => 110,            // ETIMEDOUT
            SyscallError::UnknownError => 1,         // EPERM
        }
    }
}

/// Create a socket
pub fn sys_socket(_domain: u32, _sock_type: u32, _protocol: u32) -> SysResult {
    // For now, just return a dummy socket fd
    1000
}

/// Bind a socket to an address
pub fn sys_bind(_sockfd: u32, _addr_ptr: usize, _addr_len: usize) -> SysResult {
    // For now, just return success
    0
}

/// Listen for connections on a socket
pub fn sys_listen(_sockfd: u32, _backlog: u32) -> SysResult {
    // For now, just return success
    0
}

/// Accept a connection on a socket
pub fn sys_accept(_sockfd: u32, _addr_ptr: usize, _addr_len_ptr: usize) -> SysResult {
    // For now, just return a dummy socket fd
    1001
}

/// Connect a socket to a remote address
pub fn sys_connect(_sockfd: u32, _addr_ptr: usize, _addr_len: usize) -> SysResult {
    // For now, just return success
    0
}

/// Send data through a socket
pub fn sys_send(_sockfd: u32, _buf_ptr: usize, len: usize, _flags: u32) -> SysResult {
    // For now, just return the length
    len as isize
}

/// Receive data from a socket
pub fn sys_recv(_sockfd: u32, _buf_ptr: usize, _len: usize, _flags: u32) -> SysResult {
    // For now, just return 0 bytes
    0
}

/// Close a socket
pub fn sys_close(_sockfd: u32) -> SysResult {
    // For now, just return success
    0
}
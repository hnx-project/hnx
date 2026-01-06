//! Low-level system interface for HNX
//!
//! This crate provides low-level system call interfaces and bindings
//! generated from the official HNX ABI headers.

#![no_std]

use core::arch::asm;

// System call numbers (must match kernel definitions)
// Use constants from shared::abi crate
use shared::abi::syscalls::*;

/// System call wrappers
pub mod syscalls {
    use super::*;

    /// Generic system call interface
    unsafe fn syscall0(num: usize) -> usize {
        let ret: usize;
        asm!(
            "svc #0",
            in("x8") num,
            lateout("x0") ret,
            options(nostack)
        );
        ret
    }

    /// Generic system call interface with 1 argument
    unsafe fn syscall1(num: usize, arg1: usize) -> usize {
        let ret: usize;
        asm!(
            "svc #0",
            in("x8") num,
            in("x0") arg1,
            lateout("x0") ret,
            options(nostack)
        );
        ret
    }

    /// Generic system call interface with 2 arguments
    unsafe fn syscall2(num: usize, arg1: usize, arg2: usize) -> usize {
        let ret: usize;
        asm!(
            "svc #0",
            in("x8") num,
            in("x0") arg1,
            in("x1") arg2,
            lateout("x0") ret,
            options(nostack)
        );
        ret
    }

    /// Generic system call interface with 3 arguments
    unsafe fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
        let ret: usize;
        asm!(
            "svc #0",
            in("x8") num,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            lateout("x0") ret,
            options(nostack)
        );
        ret
    }

    /// Write data to a file descriptor
    ///
    /// # Arguments
    /// * `fd` - File descriptor to write to
    /// * `buf` - Pointer to the data buffer
    /// * `count` - Number of bytes to write
    ///
    /// # Returns
    /// Number of bytes written on success, negative error code on failure
    pub unsafe fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
        // Debug: hardcode syscall number to verify
        syscall3(0x1001, fd as usize, buf as usize, count) as isize
    }

    /// Exit the current process
    ///
    /// # Arguments
    /// * `status` - Exit status code
    pub unsafe fn sys_exit(status: i32) -> ! {
        syscall1(HNX_SYS_EXIT as usize, status as usize);
        loop {}
    }

    /// Read data from a file descriptor
    ///
    /// # Arguments
    /// * `fd` - File descriptor to read from
    /// * `buf` - Pointer to the buffer to read into
    /// * `count` - Maximum number of bytes to read
    ///
    /// # Returns
    /// Number of bytes read on success, negative error code on failure
    pub unsafe fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize {
        syscall3(HNX_SYS_READ as usize, fd as usize, buf as usize, count) as isize
    }

    /// Open a file
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `flags` - Open flags
    /// * `mode` - File mode (permissions)
    ///
    /// # Returns
    /// File descriptor on success, negative error code on failure
    pub unsafe fn sys_open(path: *const u8, flags: i32, mode: i32) -> i32 {
        syscall3(HNX_SYS_OPEN as usize, path as usize, flags as usize, mode as usize) as i32
    }

    /// Close a file descriptor
    ///
    /// # Arguments
    /// * `fd` - File descriptor to close
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_close(fd: i32) -> i32 {
        syscall1(HNX_SYS_CLOSE as usize, fd as usize) as i32
    }

    /// Create a socket
    ///
    /// # Arguments
    /// * `domain` - Address family
    /// * `type_` - Socket type
    /// * `protocol` - Protocol
    ///
    /// # Returns
    /// Socket file descriptor on success, negative error code on failure
    pub unsafe fn sys_socket(domain: i32, type_: i32, protocol: i32) -> i32 {
        syscall3(HNX_SYS_SOCKET as usize, domain as usize, type_ as usize, protocol as usize) as i32
    }

    /// Bind a socket to an address
    ///
    /// # Arguments
    /// * `sockfd` - Socket file descriptor
    /// * `addr` - Address to bind to
    /// * `addrlen` - Length of address
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_bind(sockfd: i32, addr: *const u8, addrlen: usize) -> i32 {
        syscall3(HNX_SYS_BIND as usize, sockfd as usize, addr as usize, addrlen) as i32
    }

    /// Connect a socket to an address
    ///
    /// # Arguments
    /// * `sockfd` - Socket file descriptor
    /// * `addr` - Address to connect to
    /// * `addrlen` - Length of address
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_connect(sockfd: i32, addr: *const u8, addrlen: usize) -> i32 {
        syscall3(HNX_SYS_CONNECT as usize, sockfd as usize, addr as usize, addrlen) as i32
    }

    /// Send data on a socket
    ///
    /// # Arguments
    /// * `sockfd` - Socket file descriptor
    /// * `buf` - Data to send
    /// * `len` - Length of data
    /// * `flags` - Send flags
    ///
    /// # Returns
    /// Number of bytes sent on success, negative error code on failure
    pub unsafe fn sys_send(sockfd: i32, buf: *const u8, len: usize, flags: i32) -> isize {
        syscall3(HNX_SYS_SEND as usize, sockfd as usize, buf as usize, len) as isize
    }

    /// Receive data from a socket
    ///
    /// # Arguments
    /// * `sockfd` - Socket file descriptor
    /// * `buf` - Buffer to receive into
    /// * `len` - Maximum length to receive
    /// * `flags` - Receive flags
    ///
    /// # Returns
    /// Number of bytes received on success, negative error code on failure
    pub unsafe fn sys_recv(sockfd: i32, buf: *mut u8, len: usize, flags: i32) -> isize {
        syscall3(HNX_SYS_RECV as usize, sockfd as usize, buf as usize, len) as isize
    }

    /// Listen for connections on a socket
    ///
    /// # Arguments
    /// * `sockfd` - Socket file descriptor
    /// * `backlog` - Maximum length of the queue for pending connections
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_listen(sockfd: i32, backlog: i32) -> i32 {
        syscall2(HNX_SYS_LISTEN as usize, sockfd as usize, backlog as usize) as i32
    }

    /// Accept a connection on a socket
    ///
    /// # Arguments
    /// * `sockfd` - Socket file descriptor
    /// * `addr` - Address of the client
    /// * `addrlen` - Length of client address
    ///
    /// # Returns
    /// New socket file descriptor on success, negative error code on failure
    pub unsafe fn sys_accept(sockfd: i32, addr: *mut u8, addrlen: *mut usize) -> i32 {
        syscall3(HNX_SYS_ACCEPT as usize, sockfd as usize, addr as usize, addrlen as usize) as i32
    }

    /// Create a channel pair
    ///
    /// # Arguments
    /// * `options` - Channel creation options
    /// * `out0` - Pointer to store first channel handle
    /// * `out1` - Pointer to store second channel handle
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_channel_create(options: usize, out0: *mut u32, out1: *mut u32) -> i32 {
        syscall3(HNX_SYS_CHANNEL_CREATE as usize, options, out0 as usize, out1 as usize) as i32
    }

    /// Write to a channel
    ///
    /// # Arguments
    /// * `handle` - Channel handle to write to
    /// * `options` - Write options
    /// * `bytes` - Pointer to message data
    /// * `num_bytes` - Size of message data
    /// * `handles` - Pointer to handle data
    /// * `num_handles` - Number of handles
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_channel_write(handle: usize, options: usize, bytes: *const u8, 
                                   num_bytes: usize, handles: usize, num_handles: usize) -> i32 {
        syscall3(HNX_SYS_CHANNEL_WRITE as usize, handle, bytes as usize, num_bytes) as i32
    }

    /// Read from a channel
    ///
    /// # Arguments
    /// * `handle` - Channel handle to read from
    /// * `options` - Read options
    /// * `bytes` - Pointer to buffer for message data
    /// * `handles` - Pointer to buffer for handle data
    /// * `num_bytes` - Size of bytes buffer
    /// * `num_handles` - Size of handles buffer
    /// * `actual_bytes` - Pointer to store actual bytes read
    /// * `actual_handles` - Pointer to store actual handles read
    ///
    /// # Returns
    /// 0 on success, negative error code on failure
    pub unsafe fn sys_channel_read(handle: usize, options: usize, bytes: *mut u8, handles: usize,
                                  num_bytes: usize, num_handles: usize, 
                                  actual_bytes: *mut usize, actual_handles: *mut usize) -> i32 {
        syscall3(HNX_SYS_CHANNEL_READ as usize, handle, bytes as usize, num_bytes) as i32
    }
}

// Basic C-compatible functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write(fd: i32, buf: *const u8, count: usize) -> isize {
    syscalls::sys_write(fd, buf, count)
}

// Print utilities
pub mod print;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn exit(status: i32) -> ! {
    syscalls::sys_exit(status)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn read(fd: i32, buf: *mut u8, count: usize) -> isize {
    syscalls::sys_read(fd, buf, count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn open(path: *const u8, flags: i32, mode: i32) -> i32 {
    syscalls::sys_open(path, flags, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn close(fd: i32) -> i32 {
    syscalls::sys_close(fd)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn socket(domain: i32, type_: i32, protocol: i32) -> i32 {
    syscalls::sys_socket(domain, type_, protocol)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bind(sockfd: i32, addr: *const u8, addrlen: usize) -> i32 {
    syscalls::sys_bind(sockfd, addr, addrlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn connect(sockfd: i32, addr: *const u8, addrlen: usize) -> i32 {
    syscalls::sys_connect(sockfd, addr, addrlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn send(sockfd: i32, buf: *const u8, len: usize, flags: i32) -> isize {
    syscalls::sys_send(sockfd, buf, len, flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn recv(sockfd: i32, buf: *mut u8, len: usize, flags: i32) -> isize {
    syscalls::sys_recv(sockfd, buf, len, flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn listen(sockfd: i32, backlog: i32) -> i32 {
    syscalls::sys_listen(sockfd, backlog)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn accept(sockfd: i32, addr: *mut u8, addrlen: *mut usize) -> i32 {
    syscalls::sys_accept(sockfd, addr, addrlen)
}
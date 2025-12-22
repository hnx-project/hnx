//! Kernel-side ABI definitions synchronized with C headers
#![allow(non_camel_case_types)]

// System call numbers - synchronized with include/hnx/abi/syscalls.h
pub const SYS_CHANNEL_CREATE: usize = 0x0001;
pub const SYS_CHANNEL_WRITE: usize = 0x0002;
pub const SYS_CHANNEL_READ: usize = 0x0003;

pub const SYS_PROCESS_CREATE: usize = 0x0101;
pub const SYS_PROCESS_START: usize = 0x0102;

pub const SYS_THREAD_CREATE: usize = 0x0201;
pub const SYS_THREAD_START: usize = 0x0202;

pub const SYS_VMO_CREATE: usize = 0x0301;
pub const SYS_VMO_READ: usize = 0x0302;
pub const SYS_VMO_WRITE: usize = 0x0303;

// POSIX compatible syscall numbers
pub const SYS_WRITE: usize = 0x1001;
pub const SYS_READ: usize = 0x1002;
pub const SYS_OPEN: usize = 0x1003;
pub const SYS_CLOSE: usize = 0x1004;
pub const SYS_EXIT: usize = 0x1005;

// File operation syscall numbers
pub const SYS_CREAT: usize = 85;
pub const SYS_UNLINK: usize = 87;
pub const SYS_MKDIR: usize = 83;
pub const SYS_RMDIR: usize = 84;

// Memory management syscall numbers
pub const SYS_MMAP: usize = 90;
pub const SYS_MUNMAP: usize = 91;
pub const SYS_MPROTECT: usize = 92;

// Process management syscall numbers
pub const SYS_FORK: usize = 57;
pub const SYS_KILL: usize = 62;
pub const SYS_SETPGID: usize = 109;
pub const SYS_GETPGID: usize = 121;
pub const SYS_GETPPID: usize = 110;
pub const SYS_WAIT4: usize = 61;

// Driver syscall numbers
pub const SYS_DRIVER_REGISTER: usize = 2001;
pub const SYS_DRIVER_REQUEST_IRQ: usize = 2002;
pub const SYS_DRIVER_MAP_MMIO: usize = 2003;
pub const SYS_DRIVER_DMA_ALLOC: usize = 2004;

// Network syscall numbers
pub const SYS_SOCKET: usize = 41;
pub const SYS_BIND: usize = 49;
pub const SYS_CONNECT: usize = 42;
pub const SYS_LISTEN: usize = 50;
pub const SYS_ACCEPT: usize = 43;
pub const SYS_SEND: usize = 44;
pub const SYS_RECV: usize = 45;

// Dynamic linking syscall numbers
pub const SYS_DLOPEN: usize = 1001;
pub const SYS_DLCLOSE: usize = 1002;
pub const SYS_DLSYM: usize = 1003;

// IPC syscall numbers
pub const SYS_YIELD: usize = 24;
pub const SYS_IPC_WAIT: usize = 301;
pub const SYS_IPC_WAKE: usize = 302;
pub const SYS_EP_CREATE: usize = 303;
pub const SYS_EP_SEND: usize = 304;
pub const SYS_EP_RECV: usize = 305;

// Error codes - synchronized with include/hnx/abi/errors.h
pub const ZX_OK: isize = 0;
pub const ZX_ERR_INTERNAL: isize = -1;
pub const ZX_ERR_NOT_SUPPORTED: isize = -2;
pub const ZX_ERR_NO_RESOURCES: isize = -3;
pub const ZX_ERR_NO_MEMORY: isize = -4;
pub const ZX_ERR_INVALID_ARGS: isize = -10;
pub const ZX_ERR_BAD_HANDLE: isize = -11;
pub const ZX_ERR_WRONG_TYPE: isize = -12;
pub const ZX_ERR_BAD_STATE: isize = -13;
pub const ZX_ERR_TIMED_OUT: isize = -14;
pub const ZX_ERR_SHOULD_WAIT: isize = -15;
pub const ZX_ERR_CANCELED: isize = -16;
pub const ZX_ERR_PEER_CLOSED: isize = -17;
pub const ZX_ERR_NOT_FOUND: isize = -18;
pub const ZX_ERR_ALREADY_EXISTS: isize = -19;
pub const ZX_ERR_PERMISSION_DENIED: isize = -30;

// ABI Version - synchronized with include/hnx/abi/version.h
pub const HNX_ABI_VERSION_MAJOR: u32 = 0;
pub const HNX_ABI_VERSION_MINOR: u32 = 1;
pub const HNX_ABI_VERSION_PATCH: u32 = 0;

// File open flags - synchronized with POSIX
pub const O_RDONLY: u32 = 0o0;
pub const O_WRONLY: u32 = 0o1;
pub const O_RDWR: u32 = 0o2;
pub const O_CREAT: u32 = 0o100;
pub const O_TRUNC: u32 = 0o1000;
pub const O_APPEND: u32 = 0o2000;

// Object types - synchronized with include/hnx/abi/types.h
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    NONE = 0,
    PROCESS = 1,
    THREAD = 2,
    VMO = 3,
    CHANNEL = 4,
    EVENT = 5,
    EVENT_PAIR = 6,
    PORT = 7,
    TIMER = 9,
    SOCKET = 14,
    RESOURCE = 15,
    JOB = 17,
}

// Data structures - synchronized with include/hnx/abi/types.h
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct zx_channel_create_args {
    pub options: u32,
    pub out0: *mut u32,
    pub out1: *mut u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct zx_process_create_args {
    pub options: u32,
    pub name: *const u8,
    pub name_len: usize,
    pub out_proc_handle: *mut u32,
    pub out_vmar_handle: *mut u32,
}

// File descriptor entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FdEntry {
    pub name: [u8; 128],
    pub name_len: usize,
    pub size: usize,
    pub pos: usize,
    pub flags: u32,
    pub writable: bool,
}

// System call result type
pub type SysResult = isize;
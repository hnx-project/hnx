// Basic type definitions
pub type size_t = usize;
pub type ssize_t = isize;

// Process ID type
pub type pid_t = i32;

// File descriptor
pub type fd_t = i32;

// Handle type
pub type handle_t = u32;

// Error code type
pub type status_t = i32;

// Memory permissions
pub type mmap_prot_t = u32;

// Simple result type
pub type sys_result_t = i64;

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

pub type SysResult = isize;
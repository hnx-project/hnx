// src/space/abi/src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_camel_case_types)]

// 根据特性选择 std 或 no_std
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "no_std")]
extern crate core as std;

// 错误码
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    OK = 0,
    ERR_INTERNAL = -1,
    ERR_NOT_SUPPORTED = -2,
    ERR_NO_RESOURCES = -3,
    ERR_NO_MEMORY = -4,
    ERR_INVALID_ARGS = -10,
    ERR_BAD_HANDLE = -11,
    ERR_WRONG_TYPE = -12,
    ERR_BAD_STATE = -13,
    ERR_TIMEOUT = -14,
    ERR_SHOULD_WAIT = -15,
    ERR_CANCELED = -16,
    ERR_PEER_CLOSED = -17,
    ERR_NOT_FOUND = -18,
    ERR_ALREADY_EXISTS = -19,
    ERR_PERMISSION_DENIED = -30,
}

// 对象类型
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

// 系统调用号
pub mod syscalls {
    // 通道操作
    pub const SYS_CHANNEL_CREATE: u32 = 0x0001;
    pub const SYS_CHANNEL_WRITE: u32 = 0x0002;
    pub const SYS_CHANNEL_READ: u32 = 0x0003;
    
    // 进程操作
    pub const SYS_PROCESS_CREATE: u32 = 0x0101;
    pub const SYS_PROCESS_START: u32 = 0x0102;
    
    // 线程操作
    pub const SYS_THREAD_CREATE: u32 = 0x0201;
    pub const SYS_THREAD_START: u32 = 0x0202;
    
    // VMO 操作
    pub const SYS_VMO_CREATE: u32 = 0x0301;
    pub const SYS_VMO_READ: u32 = 0x0302;
    pub const SYS_VMO_WRITE: u32 = 0x0303;
    
    // 保留 POSIX 兼容系统调用号段
    pub const SYS_WRITE: u32 = 0x1001;
    pub const SYS_READ: u32 = 0x1002;
    pub const SYS_OPEN: u32 = 0x1003;
    pub const SYS_CLOSE: u32 = 0x1004;
    pub const SYS_EXIT: u32 = 0x1005;
}

// 共享数据结构
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

// 系统调用函数声明（供 bindgen 使用）
extern "C" {
    pub fn zx_channel_create(
        options: u32,
        out0: *mut u32,
        out1: *mut u32,
    ) -> i32;
    
    pub fn zx_channel_write(
        handle: u32,
        options: u32,
        bytes: *const u8,
        num_bytes: usize,
        handles: *const u32,
        num_handles: usize,
    ) -> i32;
    
    pub fn zx_channel_read(
        handle: u32,
        options: u32,
        bytes: *mut u8,
        handles: *mut u32,
        num_bytes: usize,
        num_handles: usize,
        actual_bytes: *mut usize,
        actual_handles: *mut usize,
    ) -> i32;
}
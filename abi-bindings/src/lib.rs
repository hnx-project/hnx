// abi-bindings/src/lib.rs
#![no_std]

// 包含生成的绑定
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// 或者包含源码目录的副本（供 IDE 使用）
// include!("generated.rs");

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
#[inline(always)]
pub unsafe fn syscall0(n: u32) -> isize {
    let ret: isize;
    core::arch::asm!(
        "svc #0",
        in("x8") n,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall1(n: u32, arg0: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "svc #0",
        in("x8") n,
        in("x0") arg0,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall2(n: u32, arg0: usize, arg1: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "svc #0",
        in("x8") n,
        in("x0") arg0,
        in("x1") arg1,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall3(n: u32, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "svc #0",
        in("x8") n,
        in("x0") arg0,
        in("x1") arg1,
        in("x2") arg2,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall4(n: u32, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "svc #0",
        in("x8") n,
        in("x0") arg0,
        in("x1") arg1,
        in("x2") arg2,
        in("x3") arg3,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}


//! 自动生成的系统调用常量
//! 请勿手动修改

#![allow(non_upper_case_globals, dead_code)]

pub const HNX_SYS_CHANNEL_CREATE: usize = 0x0001;
pub const HNX_SYS_CHANNEL_WRITE: usize = 0x0002;
pub const HNX_SYS_CHANNEL_READ: usize = 0x0003;
pub const HNX_SYS_PROCESS_CREATE: usize = 0x0101;
pub const HNX_SYS_PROCESS_START: usize = 0x0102;
pub const HNX_SYS_THREAD_CREATE: usize = 0x0201;
pub const HNX_SYS_THREAD_START: usize = 0x0202;
pub const HNX_SYS_VMO_CREATE: usize = 0x0301;
pub const HNX_SYS_VMO_READ: usize = 0x0302;
pub const HNX_SYS_VMO_WRITE: usize = 0x0303;
pub const HNX_SYS_WRITE: usize = 0x1001;
pub const HNX_SYS_READ: usize = 0x1002;
pub const HNX_SYS_OPEN: usize = 0x1003;
pub const HNX_SYS_CLOSE: usize = 0x1004;
pub const HNX_SYS_EXIT: usize = 0x1005;
pub const HNX_SYS_CREAT: usize = 0x0055;
pub const HNX_SYS_UNLINK: usize = 0x0057;
pub const HNX_SYS_MKDIR: usize = 0x0053;
pub const HNX_SYS_RMDIR: usize = 0x0054;
pub const HNX_SYS_MMAP: usize = 0x005A;
pub const HNX_SYS_MUNMAP: usize = 0x005B;
pub const HNX_SYS_MPROTECT: usize = 0x005C;
pub const HNX_SYS_GETPID: usize = 0x0014;
pub const HNX_SYS_FORK: usize = 0x0039;
pub const HNX_SYS_KILL: usize = 0x003E;
pub const HNX_SYS_SETPGID: usize = 0x006D;
pub const HNX_SYS_GETPGID: usize = 0x0079;
pub const HNX_SYS_GETPPID: usize = 0x006E;
pub const HNX_SYS_WAIT4: usize = 0x003D;
pub const HNX_SYS_DRIVER_REGISTER: usize = 0x07D1;
pub const HNX_SYS_DRIVER_REQUEST_IRQ: usize = 0x07D2;
pub const HNX_SYS_DRIVER_MAP_MMIO: usize = 0x07D3;
pub const HNX_SYS_DRIVER_DMA_ALLOC: usize = 0x07D4;
pub const HNX_SYS_SOCKET: usize = 0x0029;
pub const HNX_SYS_BIND: usize = 0x0031;
pub const HNX_SYS_CONNECT: usize = 0x002A;
pub const HNX_SYS_LISTEN: usize = 0x0032;
pub const HNX_SYS_ACCEPT: usize = 0x002B;
pub const HNX_SYS_SEND: usize = 0x002C;
pub const HNX_SYS_RECV: usize = 0x002D;
pub const HNX_SYS_DLOPEN: usize = 0x03E9;
pub const HNX_SYS_DLCLOSE: usize = 0x03EA;
pub const HNX_SYS_DLSYM: usize = 0x03EB;
pub const HNX_SYS_YIELD: usize = 0x0018;
pub const HNX_SYS_IPC_WAIT: usize = 0x012D;
pub const HNX_SYS_IPC_WAKE: usize = 0x012E;
pub const HNX_SYS_EP_CREATE: usize = 0x012F;
pub const HNX_SYS_EP_SEND: usize = 0x0130;
pub const HNX_SYS_EP_RECV: usize = 0x0131;

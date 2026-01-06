//! 自动生成的系统调用常量
//! 请勿手动修改

#![allow(non_upper_case_globals, dead_code)]

pub const HNX_SYS_CHANNEL_CREATE: u32 = 0x0001;
pub const HNX_SYS_CHANNEL_WRITE: u32 = 0x0002;
pub const HNX_SYS_CHANNEL_READ: u32 = 0x0003;
pub const HNX_SYS_PROCESS_CREATE: u32 = 0x0101;
pub const HNX_SYS_PROCESS_START: u32 = 0x0102;
pub const HNX_SYS_SPAWN_SERVICE: u32 = 0x0103;
pub const HNX_SYS_THREAD_CREATE: u32 = 0x0201;
pub const HNX_SYS_THREAD_START: u32 = 0x0202;
pub const HNX_SYS_VMO_CREATE: u32 = 0x0301;
pub const HNX_SYS_VMO_READ: u32 = 0x0302;
pub const HNX_SYS_VMO_WRITE: u32 = 0x0303;
pub const HNX_SYS_WRITE: u32 = 0x1001;
pub const HNX_SYS_READ: u32 = 0x1002;
pub const HNX_SYS_OPEN: u32 = 0x1003;
pub const HNX_SYS_CLOSE: u32 = 0x1004;
pub const HNX_SYS_EXIT: u32 = 0x1005;
pub const HNX_SYS_CREAT: u32 = 0x0055;
pub const HNX_SYS_UNLINK: u32 = 0x0057;
pub const HNX_SYS_MKDIR: u32 = 0x0053;
pub const HNX_SYS_RMDIR: u32 = 0x0054;
pub const HNX_SYS_MMAP: u32 = 0x005A;
pub const HNX_SYS_MUNMAP: u32 = 0x005B;
pub const HNX_SYS_MPROTECT: u32 = 0x005C;
pub const HNX_SYS_GETPID: u32 = 0x0014;
pub const HNX_SYS_FORK: u32 = 0x0039;
pub const HNX_SYS_KILL: u32 = 0x003E;
pub const HNX_SYS_SETPGID: u32 = 0x006D;
pub const HNX_SYS_GETPGID: u32 = 0x0079;
pub const HNX_SYS_GETPPID: u32 = 0x006E;
pub const HNX_SYS_WAIT4: u32 = 0x003D;
pub const HNX_SYS_DRIVER_REGISTER: u32 = 0x07D1;
pub const HNX_SYS_DRIVER_REQUEST_IRQ: u32 = 0x07D2;
pub const HNX_SYS_DRIVER_MAP_MMIO: u32 = 0x07D3;
pub const HNX_SYS_DRIVER_DMA_ALLOC: u32 = 0x07D4;
pub const HNX_SYS_SOCKET: u32 = 0x0029;
pub const HNX_SYS_BIND: u32 = 0x0031;
pub const HNX_SYS_CONNECT: u32 = 0x002A;
pub const HNX_SYS_LISTEN: u32 = 0x0032;
pub const HNX_SYS_ACCEPT: u32 = 0x002B;
pub const HNX_SYS_SEND: u32 = 0x002C;
pub const HNX_SYS_RECV: u32 = 0x002D;
pub const HNX_SYS_DLOPEN: u32 = 0x03E9;
pub const HNX_SYS_DLCLOSE: u32 = 0x03EA;
pub const HNX_SYS_DLSYM: u32 = 0x03EB;
pub const HNX_SYS_YIELD: u32 = 0x0018;
pub const HNX_SYS_IPC_WAIT: u32 = 0x012D;
pub const HNX_SYS_IPC_WAKE: u32 = 0x012E;
pub const HNX_SYS_EP_CREATE: u32 = 0x012F;
pub const HNX_SYS_EP_SEND: u32 = 0x0130;
pub const HNX_SYS_EP_RECV: u32 = 0x0131;

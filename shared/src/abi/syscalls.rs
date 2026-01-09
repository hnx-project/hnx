// Channel operations
pub const HNX_SYS_CHANNEL_CREATE: u32 = 0x0001; // 1
pub const HNX_SYS_CHANNEL_WRITE: u32 = 0x0002; // 2
pub const HNX_SYS_CHANNEL_READ: u32 = 0x0003; // 3

// Process operations
pub const HNX_SYS_PROCESS_CREATE: u32 = 0x0101; // 257
pub const HNX_SYS_PROCESS_START: u32 = 0x0102; // 258
pub const HNX_SYS_SPAWN_SERVICE: u32 = 0x0103; // 259 - Spawn service from initrd

// Thread operations
pub const HNX_SYS_THREAD_CREATE: u32 = 0x0201; // 513
pub const HNX_SYS_THREAD_START: u32 = 0x0202; // 514

// VMO operations
pub const HNX_SYS_VMO_CREATE: u32 = 0x0301; // 769
pub const HNX_SYS_VMO_READ: u32 = 0x0302; // 770
pub const HNX_SYS_VMO_WRITE: u32 = 0x0303; // 771

// POSIX compatible syscall number range
pub const HNX_SYS_WRITE: u32 = 0x1001; // 4097
pub const HNX_SYS_READ: u32 = 0x1002; // 4098
pub const HNX_SYS_OPEN: u32 = 0x1003; // 4099
pub const HNX_SYS_CLOSE: u32 = 0x1004; // 4100
pub const HNX_SYS_EXIT: u32 = 0x1005; // 4101

// File operations
pub const HNX_SYS_CREAT: u32 = 0x0055; // 85
pub const HNX_SYS_UNLINK: u32 = 0x0057; // 87
pub const HNX_SYS_MKDIR: u32 = 0x0053; // 83
pub const HNX_SYS_RMDIR: u32 = 0x0054; // 84

// Memory management
pub const HNX_SYS_MMAP: u32 = 0x005A; // 90
pub const HNX_SYS_MUNMAP: u32 = 0x005B; // 91
pub const HNX_SYS_MPROTECT: u32 = 0x005C; // 92

// Process management
pub const HNX_SYS_GETPID: u32 = 0x0014; // 20
pub const HNX_SYS_FORK: u32 = 0x0039; // 57
pub const HNX_SYS_KILL: u32 = 0x003E; // 62
pub const HNX_SYS_SETPGID: u32 = 0x006D; // 109
pub const HNX_SYS_GETPGID: u32 = 0x0079; // 121
pub const HNX_SYS_GETPPID: u32 = 0x006E; // 110
pub const HNX_SYS_WAIT4: u32 = 0x003D; // 61

// Driver
pub const HNX_SYS_DRIVER_REGISTER: u32 = 0x07D1; // 2001
pub const HNX_SYS_DRIVER_REQUEST_IRQ: u32 = 0x07D2; // 2002
pub const HNX_SYS_DRIVER_MAP_MMIO: u32 = 0x07D3; // 2003
pub const HNX_SYS_DRIVER_DMA_ALLOC: u32 = 0x07D4; // 2004

// Network
pub const HNX_SYS_SOCKET: u32 = 0x0029; // 41
pub const HNX_SYS_BIND: u32 = 0x0031; // 49
pub const HNX_SYS_CONNECT: u32 = 0x002A; // 42
pub const HNX_SYS_LISTEN: u32 = 0x0032; // 50
pub const HNX_SYS_ACCEPT: u32 = 0x002B; // 43
pub const HNX_SYS_SEND: u32 = 0x002C; // 44
pub const HNX_SYS_RECV: u32 = 0x002D; // 45

// Dynamic linking
pub const HNX_SYS_DLOPEN: u32 = 0x03E9; // 1001
pub const HNX_SYS_DLCLOSE: u32 = 0x03EA; // 1002
pub const HNX_SYS_DLSYM: u32 = 0x03EB; // 1003

// IPC
pub const HNX_SYS_YIELD: u32 = 0x0018; // 24
pub const HNX_SYS_IPC_WAIT: u32 = 0x012D; // 301
pub const HNX_SYS_IPC_WAKE: u32 = 0x012E; // 302
pub const HNX_SYS_EP_CREATE: u32 = 0x012F; // 303
pub const HNX_SYS_EP_SEND: u32 = 0x0130; // 304
pub const HNX_SYS_EP_RECV: u32 = 0x0131; // 305

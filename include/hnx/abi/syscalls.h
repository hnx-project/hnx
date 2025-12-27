// include/hnx/abi/syscalls.h
#include "types.h"
#include "errors.h"

#ifndef _HNX_ABI_SYSCALLS_H
#define _HNX_ABI_SYSCALLS_H

#include <stdint.h>

// 定义 size_t（如果 stddef.h 不可用）
#ifndef _SIZE_T_DEFINED
#define _SIZE_T_DEFINED
typedef uintptr_t size_t;
#endif

// 通道操作
#define HNX_SYS_CHANNEL_CREATE      0x0001  // 1
#define HNX_SYS_CHANNEL_WRITE      0x0002  // 2
#define HNX_SYS_CHANNEL_READ       0x0003  // 3

// 进程操作
#define HNX_SYS_PROCESS_CREATE     0x0101  // 257
#define HNX_SYS_PROCESS_START      0x0102  // 258
#define HNX_SYS_SPAWN_SERVICE      0x0103  // 259 - Spawn service from initrd

// 线程操作
#define HNX_SYS_THREAD_CREATE      0x0201  // 513
#define HNX_SYS_THREAD_START       0x0202  // 514

// VMO 操作
#define HNX_SYS_VMO_CREATE         0x0301  // 769
#define HNX_SYS_VMO_READ           0x0302  // 770
#define HNX_SYS_VMO_WRITE          0x0303  // 771

// POSIX 兼容系统调用号段
#define HNX_SYS_WRITE              0x1001  // 4097
#define HNX_SYS_READ               0x1002  // 4098
#define HNX_SYS_OPEN               0x1003  // 4099
#define HNX_SYS_CLOSE              0x1004  // 4100
#define HNX_SYS_EXIT               0x1005  // 4101

// 文件操作
#define HNX_SYS_CREAT              0x0055  // 85
#define HNX_SYS_UNLINK             0x0057  // 87
#define HNX_SYS_MKDIR              0x0053  // 83
#define HNX_SYS_RMDIR              0x0054  // 84

// 内存管理
#define HNX_SYS_MMAP               0x005A  // 90
#define HNX_SYS_MUNMAP             0x005B  // 91
#define HNX_SYS_MPROTECT           0x005C  // 92

// 进程管理
#define HNX_SYS_GETPID             0x0014  // 20
#define HNX_SYS_FORK               0x0039  // 57
#define HNX_SYS_KILL               0x003E  // 62
#define HNX_SYS_SETPGID            0x006D  // 109
#define HNX_SYS_GETPGID            0x0079  // 121
#define HNX_SYS_GETPPID            0x006E  // 110
#define HNX_SYS_WAIT4              0x003D  // 61

// 驱动程序
#define HNX_SYS_DRIVER_REGISTER    0x07D1  // 2001
#define HNX_SYS_DRIVER_REQUEST_IRQ 0x07D2  // 2002
#define HNX_SYS_DRIVER_MAP_MMIO    0x07D3  // 2003
#define HNX_SYS_DRIVER_DMA_ALLOC   0x07D4  // 2004

// 网络
#define HNX_SYS_SOCKET             0x0029  // 41
#define HNX_SYS_BIND               0x0031  // 49
#define HNX_SYS_CONNECT            0x002A  // 42
#define HNX_SYS_LISTEN             0x0032  // 50
#define HNX_SYS_ACCEPT             0x002B  // 43
#define HNX_SYS_SEND               0x002C  // 44
#define HNX_SYS_RECV               0x002D  // 45

// 动态链接
#define HNX_SYS_DLOPEN             0x03E9  // 1001
#define HNX_SYS_DLCLOSE            0x03EA  // 1002
#define HNX_SYS_DLSYM              0x03EB  // 1003

// IPC
#define HNX_SYS_YIELD              0x0018  // 24
#define HNX_SYS_IPC_WAIT           0x012D  // 301
#define HNX_SYS_IPC_WAKE           0x012E  // 302
#define HNX_SYS_EP_CREATE          0x012F  // 303
#define HNX_SYS_EP_SEND            0x0130  // 304
#define HNX_SYS_EP_RECV            0x0131  // 305

// 系统调用函数原型（用户空间使用）
#ifdef __cplusplus
extern "C" {
#endif

// 注意：这些是用户空间调用的包装函数原型
// 实际实现在libc或hnxlib中
int32_t zx_channel_create(uint32_t options, uint32_t* out0, uint32_t* out1);
int32_t zx_channel_write(uint32_t handle, uint32_t options, const uint8_t* bytes, size_t num_bytes, const uint32_t* handles, size_t num_handles);
int32_t zx_channel_read(uint32_t handle, uint32_t options, uint8_t* bytes, uint32_t* handles, size_t num_bytes, size_t num_handles, size_t* actual_bytes, size_t* actual_handles);

#ifdef __cplusplus
}
#endif

#endif // _HNX_ABI_SYSCALLS_H
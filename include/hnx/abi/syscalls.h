#ifndef _HNX_ABI_SYSCALLS_H
#define _HNX_ABI_SYSCALLS_H

#include <stdint.h>

// 通道操作
#define SYS_CHANNEL_CREATE      0x0001
#define SYS_CHANNEL_WRITE      0x0002
#define SYS_CHANNEL_READ       0x0003

// 进程操作
#define SYS_PROCESS_CREATE     0x0101
#define SYS_PROCESS_START      0x0102

// 线程操作
#define SYS_THREAD_CREATE      0x0201
#define SYS_THREAD_START       0x0202

// VMO 操作
#define SYS_VMO_CREATE         0x0301
#define SYS_VMO_READ           0x0302
#define SYS_VMO_WRITE          0x0303

// POSIX 兼容系统调用号段
#define SYS_WRITE              0x1001
#define SYS_READ               0x1002
#define SYS_OPEN               0x1003
#define SYS_CLOSE              0x1004
#define SYS_EXIT               0x1005

// 文件操作
#define SYS_CREAT              85
#define SYS_UNLINK             87
#define SYS_MKDIR              83
#define SYS_RMDIR              84

// 内存管理
#define SYS_MMAP               90
#define SYS_MUNMAP             91
#define SYS_MPROTECT           92

// 进程管理
#define SYS_FORK               57
#define SYS_KILL               62
#define SYS_SETPGID            109
#define SYS_GETPGID            121
#define SYS_GETPPID            110
#define SYS_WAIT4              61

// 驱动程序
#define SYS_DRIVER_REGISTER    2001
#define SYS_DRIVER_REQUEST_IRQ 2002
#define SYS_DRIVER_MAP_MMIO    2003
#define SYS_DRIVER_DMA_ALLOC   2004

// 网络
#define SYS_SOCKET             41
#define SYS_BIND               49
#define SYS_CONNECT            42
#define SYS_LISTEN             50
#define SYS_ACCEPT             43
#define SYS_SEND               44
#define SYS_RECV               45

// 动态链接
#define SYS_DLOPEN             1001
#define SYS_DLCLOSE            1002
#define SYS_DLSYM              1003

// IPC
#define SYS_YIELD              24
#define SYS_IPC_WAIT           301
#define SYS_IPC_WAKE           302
#define SYS_EP_CREATE          303
#define SYS_EP_SEND            304
#define SYS_EP_RECV            305

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
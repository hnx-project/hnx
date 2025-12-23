// include/hnx/abi/types.h
#ifndef _HNX_ABI_TYPES_H
#define _HNX_ABI_TYPES_H

#include <stdint.h>

// 基础类型定义
typedef uintptr_t size_t;
typedef intptr_t ssize_t;

// 进程ID类型
typedef int32_t pid_t;

// 文件描述符
typedef int32_t fd_t;

// 句柄类型
typedef uint32_t handle_t;

// 错误码类型
typedef int32_t status_t;

// 内存权限
typedef uint32_t mmap_prot_t;

// 简单的结果类型
typedef int64_t sys_result_t;

// Zircon 风格的结构
typedef struct {
    uint32_t options;
    uint32_t* out0;
    uint32_t* out1;
} zx_channel_create_args_t;

#endif // _HNX_ABI_TYPES_H
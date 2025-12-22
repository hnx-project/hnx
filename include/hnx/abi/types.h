#ifndef _HNX_ABI_TYPES_H
#define _HNX_ABI_TYPES_H

#include <stdint.h>
#include <stddef.h>

// 对象类型
typedef enum {
    ZX_OBJ_TYPE_NONE = 0,
    ZX_OBJ_TYPE_PROCESS = 1,
    ZX_OBJ_TYPE_THREAD = 2,
    ZX_OBJ_TYPE_VMO = 3,
    ZX_OBJ_TYPE_CHANNEL = 4,
    ZX_OBJ_TYPE_EVENT = 5,
    ZX_OBJ_TYPE_EVENTPAIR = 6,
    ZX_OBJ_TYPE_PORT = 7,
    ZX_OBJ_TYPE_TIMER = 9,
    ZX_OBJ_TYPE_SOCKET = 14,
    ZX_OBJ_TYPE_RESOURCE = 15,
    ZX_OBJ_TYPE_JOB = 17,
} zx_obj_type_t;

// 数据结构
typedef struct {
    uint32_t options;
    uint32_t* out0;
    uint32_t* out1;
} zx_channel_create_args_t;

typedef struct {
    uint32_t options;
    const uint8_t* name;
    size_t name_len;
    uint32_t* out_proc_handle;
    uint32_t* out_vmar_handle;
} zx_process_create_args_t;

#endif // _HNX_ABI_TYPES_H
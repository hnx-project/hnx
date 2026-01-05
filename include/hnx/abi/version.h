#ifndef _HNX_ABI_VERSION_H
#define _HNX_ABI_VERSION_H

#define HNX_ABI_VERSION_MAJOR 0
#define HNX_ABI_VERSION_MINOR 3
#define HNX_ABI_VERSION_PATCH 0
#define HNX_ABI_VERSION "0.3.0-alpha.1+20260105.8b778bd"

// 检查兼容性的宏
#define HNX_ABI_CHECK_VERSION(major, minor, patch) \
    ((major == HNX_ABI_VERSION_MAJOR) && \
     (minor <= HNX_ABI_VERSION_MINOR))

#endif // _HNX_ABI_VERSION_H

// include/hnx/abi/errors.h
#ifndef _HNX_ABI_ERRORS_H
#define _HNX_ABI_ERRORS_H

// Zircon 风格的错误码
#define ZX_OK                    0
#define ZX_ERR_INTERNAL         -1
#define ZX_ERR_NOT_SUPPORTED    -2
#define ZX_ERR_NO_RESOURCES     -3
#define ZX_ERR_NO_MEMORY        -4
#define ZX_ERR_INVALID_ARGS     -10
#define ZX_ERR_BAD_HANDLE       -11
#define ZX_ERR_WRONG_TYPE       -12
#define ZX_ERR_BAD_STATE        -13
#define ZX_ERR_TIMEOUT          -14
#define ZX_ERR_SHOULD_WAIT      -15
#define ZX_ERR_CANCELED         -16
#define ZX_ERR_PEER_CLOSED      -17
#define ZX_ERR_NOT_FOUND        -18
#define ZX_ERR_ALREADY_EXISTS   -19
#define ZX_ERR_PERMISSION_DENIED -30

// POSIX 兼容错误码
#define ESUCCESS                0
#define EPERM                   1
#define ENOENT                  2
#define ESRCH                   3
#define EINTR                   4
#define EIO                     5
#define ENXIO                   6
#define E2BIG                   7
#define ENOEXEC                 8
#define EBADF                   9
#define ECHILD                  10
#define EAGAIN                  11
#define ENOMEM                  12
#define EACCES                  13
#define EFAULT                  14
#define ENOTBLK                 15
#define EBUSY                   16
#define EEXIST                  17
#define EXDEV                   18
#define ENODEV                  19
#define ENOTDIR                 20
#define EISDIR                  21
#define EINVAL                  22
#define ENFILE                  23
#define EMFILE                  24
#define ENOTTY                  25
#define ETXTBSY                 26
#define EFBIG                   27
#define ENOSPC                  28
#define ESPIPE                  29
#define EROFS                   30
#define EMLINK                  31
#define EPIPE                   32
#define EDOM                    33
#define ERANGE                  34

#endif // _HNX_ABI_ERRORS_H
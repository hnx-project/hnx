# 错误处理 (Error Handling)

HNX 内核的 `error` 模块 (`kernel/src/error`) 负责定义和管理内核操作中可能出现的错误。一个健壮的错误处理机制对于操作系统的稳定性和可靠性至关重要。该模块旨在提供统一的错误类型和处理流程，使得内核代码能够清晰、安全地处理异常情况。

## 设计原则

-   **清晰的错误语义:** 错误类型应准确地描述问题的性质，便于开发者理解和调试。
-   **避免恐慌 (Panic):** 尽可能地避免在内核中发生不可恢复的 `panic`，除非系统处于无法继续运行的状态。大多数错误应通过返回 `Result` 类型进行处理。
-   **错误传播:** 错误应能够有效地在函数调用链中传播，直到被合适的代码块捕获和处理。
-   **资源清理:** 在错误发生时，应确保所有已分配的资源能够被正确释放，避免资源泄漏。

## 核心组件

`error` 模块通常会定义一个或多个枚举类型来表示不同类别的错误，例如：

-   **`SysError`:** 表示系统调用级别的错误，用户空间可能通过系统调用接口感知到这些错误。
-   **`KernelError`:** 涵盖内核内部操作可能遇到的所有错误，例如内存分配失败、设备访问错误、IPC 错误等。

这些错误类型通常会包含一个错误代码或更详细的描述信息，以便于定位问题。例如：

```rust
pub enum SysError {
    /// 操作成功完成 (Success)
    Ok = 0,
    /// 权限不足 (Permission Denied)
    PermissionDenied = 1,
    /// 内存不足 (Out of Memory)
    OutOfMemory = 2,
    /// 无效参数 (Invalid Argument)
    InvalidArgument = 3,
    /// 设备或资源忙 (Device or Resource Busy)
    ResourceBusy = 4,
    /// 未找到文件或目录 (No Such File or Directory)
    NoSuchEntry = 5,
    /// 无法实现此功能 (Operation Not Supported)
    NotSupported = 6,
    /// IPC 错误 (Inter-Process Communication Error)
    IpcError = 7,
    /// 超时 (Timeout)
    Timeout = 8,
    /// 缓冲区太小 (Buffer Too Small)
    BufferTooSmall = 9,
    /// 未知错误 (Unknown Error)
    Unknown = 0xFF,
}

// 通常会有从内部错误类型到 SysError 的转换实现
impl From<KernelError> for SysError {
    fn from(err: KernelError) -> Self {
        match err {
            // ... 匹配 KernelError 到 SysError ...
            _ => SysError::Unknown,
        }
    }
}

// KernelError 可能更加细致
pub enum KernelError {
    MemoryAllocationFailed,
    PageTableError,
    IpcEndpointNotFound,
    // ... 更多内核内部错误 ...
}
```

## 错误日志与调试

除了返回错误码，内核还会利用日志系统 (`log` crate) 记录详细的错误信息，这对于内核的调试和问题追踪至关重要。错误日志通常会包含错误发生的模块、函数、行号以及相关的变量值。

-   **`error!` 宏:** 用于记录严重错误，通常表示系统功能受到影响。
-   **`warn!` 宏:** 用于记录警告信息，表示可能存在问题但系统仍能继续运行。

## 未来展望

未来的错误处理可能会引入更复杂的错误上下文传递机制，例如使用 `anyhow` 或 `thiserror` 等库的思想，但这需要根据 HNX 的特定需求和 `no_std` 环境的限制进行权衡和适配。目标是提供一个既能满足内核严格要求，又能提高开发效率的错误处理系统。
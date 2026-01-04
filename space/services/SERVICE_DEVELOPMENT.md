# HNX 服务开发指南

## 概述

HNX 采用微内核架构，核心功能通过用户空间服务实现。本文档描述如何为 HNX 开发统一结构的服务。

## 服务架构

### 核心概念

- **微内核**: 仅提供基本功能（IPC、内存管理、调度）
- **用户空间服务**: 所有系统功能作为独立服务运行
- **基于能力的 IPC**: 通过端点（Endpoint）进行安全的进程间通信
- **服务注册与发现**: 通过 IPC 路由器服务进行服务发现

### 服务层次结构

```
┌─────────────────┐
│   Init (PID 1)  │ 初始化进程，启动核心服务
└────────┬────────┘
         │
┌────────▼────────┐
│  IPC Router     │ 服务注册和消息路由
└────────┬────────┘
         │
┌────────▼────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Process Manager │    │  VFS Service    │    │  Echo Service   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 统一的服务开发结构

### 服务模板

所有服务应遵循以下统一结构：

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::ipc::{IpcError, ServiceFramework};

/// 服务主函数
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 创建服务框架
    let framework = match ServiceFramework::new("service-name") {
        Ok(f) => f,
        Err(_) => {
            // 框架创建失败，简单循环
            loop {
                hnxlib::syscall::yield_cpu();
            }
        }
    };

    // 运行服务
    framework.run(handle_message);
}

/// 处理IPC消息
fn handle_message(op: u16, request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    match op {
        // 操作码1: 功能1
        1 => handle_function1(request, response),
        // 操作码2: 功能2
        2 => handle_function2(request, response),
        // 未知操作码
        _ => Err(IpcError::InvalidArgs),
    }
}

/// 具体功能处理函数
fn handle_function1(request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 处理请求并填充响应
    let response_data = b"Response";
    let len = response_data.len().min(response.len());
    response[..len].copy_from_slice(&response_data[..len]);
    Ok(len)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        hnxlib::syscall::yield_cpu();
    }
}
```

### 服务框架特性

`ServiceFramework` 提供以下功能：

1. **自动服务注册**: 启动时向 IPC 路由器注册服务
2. **消息处理循环**: 自动处理消息接收和发送
3. **错误处理**: 统一的错误响应机制
4. **资源管理**: 自动管理端点资源

### 操作码约定

- `100-199`: 测试和诊断操作
- `200-299`: 进程管理操作
- `300-399`: 文件系统操作  
- `400-499`: 设备驱动操作
- `500-599`: 网络操作
- `600-699`: 系统管理操作

## IPC 通信机制

### 基本概念

- **端点 (Endpoint)**: 通信的基本单元，每个端点有唯一 ID
- **句柄 (Handle)**: 对端点的引用，通过能力系统控制权限
- **消息 (Message)**: 包含操作码和数据的通信单元

### 消息格式

```
+----------------+----------------+----------------+
|   操作码 (2B)  |   数据长度 (2B)|     数据       |
+----------------+----------------+----------------+
```

### 通信模式

#### 1. 请求-响应模式

```rust
use hnxlib::ipc::{Endpoint, IpcError};

fn request_response(endpoint: &Endpoint) -> Result<Vec<u8>, IpcError> {
    let request = b"Hello";
    let mut response = [0u8; 256];
    
    endpoint.send(100, request)?;
    let (op, len) = endpoint.recv(&mut response)?;
    
    Ok(response[..len].to_vec())
}
```

#### 2. 发布-订阅模式

通过 IPC 路由器实现服务发现：

```rust
use hnxlib::ipc::IpcRouterClient;

// 连接到路由器
let router = IpcRouterClient::connect()?;

// 注册服务
router.register_service("my-service", &endpoint)?;

// 查找服务
let target_endpoint = router.find_service("target-service")?;
```

### 错误处理

IPC 操作返回 `Result<T, IpcError>`，错误类型包括：

- `InvalidArgs`: 无效参数
- `PermissionDenied`: 权限不足
- `EndpointNotFound`: 端点不存在
- `MessageTooLarge`: 消息过大
- `Timeout`: 超时
- `SyscallFailed`: 系统调用失败

## 服务开发步骤

### 1. 创建服务目录

```bash
mkdir -p space/services/my-service/src
```

### 2. 创建 Cargo.toml

```toml
[package]
name = "my-service"
version = "0.1.0"
edition = "2021"

[dependencies]
hnx-abi = { path = "../../../abi-bindings" }
libc = { path = "../../libc" }
hnxlib = { path = "../../hnxlib" }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
opt-level = "z"
lto = true
```

### 3. 实现服务逻辑

参考上面的服务模板实现 `_start` 和消息处理函数。

### 4. 添加到工作空间

编辑 `space/Cargo.toml`，在 `members` 中添加：

```toml
members = [
    # ...
    "services/my-service",
]
```

### 5. 构建和测试

```bash
make space           # 构建所有空间组件
make image           # 创建完整系统镜像
make run             # 在 QEMU 中运行测试
```

## 现有服务示例

### Echo Service

简单的回显服务，演示基本 IPC 通信：

```rust
fn handle_message(op: u16, request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    match op {
        // Echo操作: 原样返回接收到的数据
        100 => {
            let len = request.len().min(response.len());
            response[..len].copy_from_slice(&request[..len]);
            Ok(len)
        }
        
        // Ping操作: 返回"Pong"
        101 => {
            const PONG: &[u8] = b"Pong";
            let len = PONG.len().min(response.len());
            response[..len].copy_from_slice(&PONG[..len]);
            Ok(len)
        }
        
        _ => Err(IpcError::InvalidArgs),
    }
}
```

### IPC Router Service

服务注册和发现的核心服务，管理服务注册表。

### Process Manager Service

进程生命周期管理和服务健康监控。

## 最佳实践

### 1. 资源管理

- 使用 `Endpoint` 类型安全地管理端点句柄
- 及时释放不再需要的资源
- 避免在消息处理中进行长时间操作

### 2. 错误处理

- 为所有 IPC 操作提供有意义的错误响应
- 使用标准的错误操作码（原操作码 + 1 表示错误）
- 记录关键错误以便调试

### 3. 安全性

- 验证所有输入参数
- 实施最小权限原则
- 使用能力系统控制访问权限

### 4. 性能

- 使用固定大小的缓冲区避免动态分配
- 减少消息拷贝次数
- 合理使用 `yield_cpu()` 避免饥饿

## 调试技巧

### 1. 日志输出

```rust
use hnxlib::println;

println!("[my-service] Starting initialization");
```

### 2. 系统调用跟踪

内核日志显示系统调用入口和退出信息，可通过 QEMU 输出查看。

### 3. 内存调试

- 检查堆栈使用情况
- 验证指针有效性
- 使用 `user_range_ok` 验证用户内存访问

## 常见问题

### Q1: 服务无法启动

**可能原因**:
- initrd 中缺少服务二进制文件
- 系统调用返回值传递问题
- 内存分配失败

**解决方案**:
- 检查 `make image` 输出确认服务已包含
- 验证系统调用 ABI 一致性
- 增加堆大小或优化内存使用

### Q2: IPC 消息丢失

**可能原因**:
- 端点句柄无效
- 权限不足
- 缓冲区大小不足

**解决方案**:
- 验证端点句柄有效性
- 检查能力权限
- 增加接收缓冲区大小

### Q3: 服务注册失败

**可能原因**:
- IPC 路由器服务未运行
- 服务名称冲突
- 注册表已满

**解决方案**:
- 确保 IPC 路由器服务已启动
- 使用唯一服务名称
- 增加注册表容量或清理旧条目

## 下一步计划

### 短期目标

1. **完善 IPC 机制**: 实现完整的消息路由和连接管理
2. **服务健康监控**: 实现服务心跳和自动重启
3. **能力系统集成**: 完善基于能力的访问控制

### 长期目标

1. **动态服务加载**: 支持运行时加载和卸载服务
2. **服务版本管理**: 支持服务版本控制和兼容性
3. **分布式服务**: 支持跨节点的服务通信

---

## 附录

### A. 系统调用参考

| 系统调用 | 编号 | 描述 |
|---------|------|------|
| `HNX_SYS_SPAWN_SERVICE` | 0x0103 | 从 initrd 加载服务 |
| `HNX_SYS_EP_CREATE` | 0x012F | 创建 IPC 端点 |
| `HNX_SYS_EP_SEND` | 0x0130 | 发送 IPC 消息 |
| `HNX_SYS_EP_RECV` | 0x0131 | 接收 IPC 消息 |
| `HNX_SYS_IPC_WAIT` | 0x012D | 等待 IPC 事件 |
| `HNX_SYS_IPC_WAKE` | 0x012E | 唤醒等待的进程 |

### B. 示例项目结构

```
my-service/
├── Cargo.toml
└── src/
    └── main.rs
```

### C. 参考实现

- `space/services/echo-service/`: 简单回显服务
- `space/services/ipcrouter-service/`: IPC 路由器服务
- `space/services/procmgr-service/`: 进程管理服务

### D. 相关文档

- `QWEN.md`: 项目总体架构和开发状态
- `README.MD`: 项目构建和运行指南
- `include/hnx/abi/syscalls.h`: 系统调用 ABI 定义
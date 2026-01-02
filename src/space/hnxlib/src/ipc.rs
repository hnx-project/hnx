//! IPC（进程间通信）高级抽象
//!
//! 提供基于端点的消息传递抽象，简化服务间通信。
//! 
//! ## 核心概念
//! - **端点 (Endpoint)**: 通信的基本单元，每个端点有一个唯一ID
//! - **句柄 (Handle)**: 对端点的引用，通过能力系统进行权限控制
//! - **消息 (Message)**: 包含操作码和数据的通信单元
//! - **服务注册 (Service Registry)**: 服务通过IPC路由器注册其端点

use core::fmt;

/// IPC错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// 无效参数
    InvalidArgs,
    /// 权限不足
    PermissionDenied,
    /// 端点不存在
    EndpointNotFound,
    /// 消息过大
    MessageTooLarge,
    /// 超时
    Timeout,
    /// 系统调用失败
    SyscallFailed(isize),
    /// 其他错误
    Other,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::InvalidArgs => write!(f, "Invalid arguments"),
            IpcError::PermissionDenied => write!(f, "Permission denied"),
            IpcError::EndpointNotFound => write!(f, "Endpoint not found"),
            IpcError::MessageTooLarge => write!(f, "Message too large"),
            IpcError::Timeout => write!(f, "Timeout"),
            IpcError::SyscallFailed(code) => write!(f, "System call failed with code {}", code),
            IpcError::Other => write!(f, "Other IPC error"),
        }
    }
}

/// IPC端点句柄
///
/// 封装端点句柄，提供类型安全的操作。
/// 当`Endpoint`被drop时，句柄不会自动关闭（需要显式管理）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    handle: usize,
}

impl Endpoint {
    /// 创建新的IPC端点
    pub fn create() -> Result<Self, IpcError> {
        let handle = crate::safe_syscall::ep_create();
        if handle < 0 {
            Err(IpcError::SyscallFailed(handle))
        } else {
            Ok(Self { handle: handle as usize })
        }
    }

    /// 从现有句柄创建端点（不安全）
    ///
    /// # 安全性
    /// 调用者必须确保句柄是有效的端点句柄。
    pub unsafe fn from_handle(handle: usize) -> Self {
        Self { handle }
    }

    /// 获取端点句柄
    pub fn handle(&self) -> usize {
        self.handle
    }

    /// 发送消息到端点
    pub fn send(&self, op: u16, data: &[u8]) -> Result<usize, IpcError> {
        let result = crate::safe_syscall::ep_send(self.handle, op, data);
        if result < 0 {
            Err(IpcError::SyscallFailed(result))
        } else {
            Ok(result as usize)
        }
    }

    /// 从端点接收消息
    pub fn recv(&self, buffer: &mut [u8]) -> Result<(u16, usize), IpcError> {
        // 注意：内核的ep_recv只返回数据长度，不包含操作码
        // 操作码在消息内部，需要解析完整消息
        // 暂时简化：只返回接收到的数据长度
        let result = crate::safe_syscall::ep_recv(self.handle, buffer);
        if result < 0 {
            Err(IpcError::SyscallFailed(result))
        } else {
            // TODO: 解析消息获取操作码
            Ok((0, result as usize))
        }
    }

    /// 发送消息并等待响应（请求-回复模式）
    pub fn call(&self, op: u16, request: &[u8], response: &mut [u8]) -> Result<(u16, usize), IpcError> {
        self.send(op, request)?;
        self.recv(response)
    }
}

/// IPC消息
///
/// 完整的IPC消息结构，包含操作码和数据。
pub struct IpcMessage {
    /// 操作码（16位）
    pub op: u16,
    /// 消息数据
    pub data: [u8; 256],
    /// 实际数据长度
    pub len: usize,
}

impl IpcMessage {
    /// 创建新消息
    pub fn new(op: u16, data: &[u8]) -> Self {
        let mut msg = Self {
            op,
            data: [0; 256],
            len: data.len().min(256),
        };
        msg.data[..msg.len].copy_from_slice(&data[..msg.len]);
        msg
    }

    /// 获取消息数据切片
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// 获取操作码
    pub fn op(&self) -> u16 {
        self.op
    }
}

/// IPC路由器客户端
///
/// 用于与服务注册和发现系统交互。
pub struct IpcRouterClient {
    /// 路由器服务的端点
    router_endpoint: Endpoint,
}

impl IpcRouterClient {
    /// 连接到IPC路由器服务
    ///
    /// # 注意
    /// 目前假设路由器服务使用固定的端点ID 1。
    /// 未来需要通过服务发现机制获取路由器端点。
    pub fn connect() -> Result<Self, IpcError> {
        // TODO: 实现服务发现机制
        // 目前硬编码路由器端点句柄为1
        Ok(Self {
            router_endpoint: unsafe { Endpoint::from_handle(1) },
        })
    }

    /// 注册服务
    ///
    /// 向IPC路由器注册服务名称和端点。
    pub fn register_service(&self, service_name: &str, endpoint: &Endpoint) -> Result<(), IpcError> {
        let mut payload = [0u8; 256];
        let name_bytes = service_name.as_bytes();
        let payload_len = name_bytes.len().min(128);
        
        // 格式：[端点句柄(usize)][服务名称长度(usize)][服务名称...]
        let handle = endpoint.handle();
        payload[0..8].copy_from_slice(&handle.to_ne_bytes());
        payload[8..16].copy_from_slice(&(payload_len as usize).to_ne_bytes());
        payload[16..16+payload_len].copy_from_slice(&name_bytes[..payload_len]);
        
        // 使用操作码1表示服务注册
        self.router_endpoint.send(1, &payload[..16+payload_len])?;
        Ok(())
    }

    /// 查找服务
    ///
    /// 通过服务名称查找对应的端点。
    pub fn find_service(&self, service_name: &str) -> Result<Endpoint, IpcError> {
        let mut payload = [0u8; 256];
        let name_bytes = service_name.as_bytes();
        let payload_len = name_bytes.len().min(128);
        
        // 格式：[服务名称长度(usize)][服务名称...]
        payload[0..8].copy_from_slice(&(payload_len as usize).to_ne_bytes());
        payload[8..8+payload_len].copy_from_slice(&name_bytes[..payload_len]);
        
        // 使用操作码2表示服务查找
        self.router_endpoint.send(2, &payload[..8+payload_len])?;
        
        // 等待响应
        let mut response = [0u8; 16];
        let (_, len) = self.router_endpoint.recv(&mut response)?;
        
        if len >= 8 {
            let handle_bytes = &response[0..8];
            let handle = usize::from_ne_bytes([
                handle_bytes[0], handle_bytes[1], handle_bytes[2], handle_bytes[3],
                handle_bytes[4], handle_bytes[5], handle_bytes[6], handle_bytes[7],
            ]);
            Ok(unsafe { Endpoint::from_handle(handle) })
        } else {
            Err(IpcError::EndpointNotFound)
        }
    }
}

/// 服务框架
///
/// 简化服务开发的框架，提供标准化的服务结构。
pub struct ServiceFramework {
    /// 服务端点
    endpoint: Endpoint,
    /// IPC路由器客户端
    router: IpcRouterClient,
    /// 服务名称
    name: &'static str,
}

impl ServiceFramework {
    /// 创建新服务
    pub fn new(name: &'static str) -> Result<Self, IpcError> {
        let endpoint = Endpoint::create()?;
        let router = IpcRouterClient::connect()?;
        
        Ok(Self {
            endpoint,
            router,
            name,
        })
    }

    /// 启动服务
    ///
    /// 注册服务并进入消息处理循环。
    pub fn run<F>(mut self, handler: F) -> !
    where
        F: Fn(u16, &[u8], &mut [u8]) -> Result<usize, IpcError>,
    {
        // 注册服务
        if let Err(_e) = self.router.register_service(self.name, &self.endpoint) {
            // 注册失败，仍然可以运行但不支持服务发现
            let _ = crate::safe_syscall::debug_print("[Service] Failed to register service\n");
        } else {
            let _ = crate::safe_syscall::debug_print("[Service] Service registered successfully\n");
        }

        // 消息处理循环
        let mut buffer = [0u8; 256];
        let mut response_buf = [0u8; 256];
        loop {
            match self.endpoint.recv(&mut buffer) {
                Ok((op, len)) => {
                    let request = &buffer[..len];
                    match handler(op, request, &mut response_buf) {
                        Ok(response_len) => {
                            // 发送响应
                            let _ = self.endpoint.send(op, &response_buf[..response_len]);
                        }
                        Err(e) => {
                            // 发送错误响应（操作码加1表示错误）
                            let error_msg = b"Error";
                            let _ = self.endpoint.send(op + 1, error_msg);
                        }
                    }
                }
                Err(e) => {
                    // 接收错误，继续循环
                    match e {
                        IpcError::SyscallFailed(code) => {
                            let _ = crate::safe_syscall::debug_print("[Service] Receive error - syscall failed: ");
                            // 简单打印错误代码
                            if code == -1 {
                                let _ = crate::safe_syscall::debug_print("-1\n");
                            } else if code == -2 {
                                let _ = crate::safe_syscall::debug_print("-2\n");
                            } else {
                                let _ = crate::safe_syscall::debug_print("unknown\n");
                            }
                        }
                        _ => {
                            let _ = crate::safe_syscall::debug_print("[Service] Receive error\n");
                        }
                    }
                }
            }

            // 让出CPU
            crate::safe_syscall::yield_cpu();
        }
    }

    /// 获取服务端点
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}
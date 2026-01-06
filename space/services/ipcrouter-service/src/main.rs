#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::ipc::{Endpoint, IpcError, ServiceFramework};

/// 最大服务注册数量
const MAX_SERVICES: usize = 32;

/// 服务注册条目
struct ServiceEntry {
    name: [u8; 64],
    name_len: usize,
    endpoint: Endpoint,
}

/// 服务注册表
struct ServiceRegistry {
    services: [Option<ServiceEntry>; MAX_SERVICES],
}

impl ServiceRegistry {
    const fn new() -> Self {
        Self {
            services: [const { None }; MAX_SERVICES],
        }
    }

    /// 注册服务
    fn register(&mut self, name: &str, endpoint: Endpoint) -> Result<(), IpcError> {
        // 检查是否已注册
        for entry in &self.services {
            if let Some(entry) = entry {
                if entry.name_len == name.len() && &entry.name[..entry.name_len] == name.as_bytes() {
                    return Err(IpcError::Other); // 已存在
                }
            }
        }

        // 查找空闲槽位
        for slot in &mut self.services {
            if slot.is_none() {
                let mut name_buf = [0u8; 64];
                let len = name.len().min(64);
                name_buf[..len].copy_from_slice(&name.as_bytes()[..len]);
                
                *slot = Some(ServiceEntry {
                    name: name_buf,
                    name_len: len,
                    endpoint,
                });
                return Ok(());
            }
        }

        Err(IpcError::Other) // 注册表已满
    }

    /// 查找服务
    fn find(&self, name: &str) -> Option<Endpoint> {
        for entry in &self.services {
            if let Some(entry) = entry {
                if entry.name_len == name.len() && &entry.name[..entry.name_len] == name.as_bytes() {
                    return Some(entry.endpoint);
                }
            }
        }
        None
    }
}

/// 全局服务注册表
static mut SERVICE_REGISTRY: ServiceRegistry = ServiceRegistry::new();

/// IPC路由器服务主函数
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 创建服务框架
    let framework = match ServiceFramework::new("ipcrouter") {
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
        // 操作码1: 注册服务
        1 => handle_register(request, response),
        // 操作码2: 查找服务
        2 => handle_find(request, response),
        // 操作码3: 列出服务
        3 => handle_list(response),
        _ => Err(IpcError::InvalidArgs),
    }
}

/// 处理服务注册
fn handle_register(request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 请求格式: [端点句柄(8字节)][服务名称长度(8字节)][服务名称...]
    if request.len() < 16 {
        return Err(IpcError::InvalidArgs);
    }

    // 解析端点句柄
    let handle_bytes = &request[0..8];
    let handle = usize::from_ne_bytes([
        handle_bytes[0], handle_bytes[1], handle_bytes[2], handle_bytes[3],
        handle_bytes[4], handle_bytes[5], handle_bytes[6], handle_bytes[7],
    ]);

    // 解析服务名称长度
    let name_len_bytes = &request[8..16];
    let name_len = usize::from_ne_bytes([
        name_len_bytes[0], name_len_bytes[1], name_len_bytes[2], name_len_bytes[3],
        name_len_bytes[4], name_len_bytes[5], name_len_bytes[6], name_len_bytes[7],
    ]);

    if request.len() < 16 + name_len {
        return Err(IpcError::InvalidArgs);
    }

    // 解析服务名称
    let name_data = &request[16..16 + name_len];
    let name = match core::str::from_utf8(name_data) {
        Ok(s) => s,
        Err(_) => return Err(IpcError::InvalidArgs),
    };

    // 创建端点对象
    let endpoint = unsafe { Endpoint::from_handle(handle) };

    // 注册服务
    unsafe {
        match SERVICE_REGISTRY.register(name, endpoint) {
            Ok(()) => {
                response[0] = 1; // 成功
                Ok(1)
            }
            Err(e) => Err(e),
        }
    }
}

/// 处理服务查找
fn handle_find(request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 请求格式: [服务名称长度(8字节)][服务名称...]
    if request.len() < 8 {
        return Err(IpcError::InvalidArgs);
    }

    // 解析服务名称长度
    let name_len_bytes = &request[0..8];
    let name_len = usize::from_ne_bytes([
        name_len_bytes[0], name_len_bytes[1], name_len_bytes[2], name_len_bytes[3],
        name_len_bytes[4], name_len_bytes[5], name_len_bytes[6], name_len_bytes[7],
    ]);

    if request.len() < 8 + name_len {
        return Err(IpcError::InvalidArgs);
    }

    // 解析服务名称
    let name_data = &request[8..8 + name_len];
    let name = match core::str::from_utf8(name_data) {
        Ok(s) => s,
        Err(_) => return Err(IpcError::InvalidArgs),
    };

    // 查找服务
    unsafe {
        match SERVICE_REGISTRY.find(name) {
            Some(endpoint) => {
                let handle = endpoint.handle();
                response[0..8].copy_from_slice(&handle.to_ne_bytes());
                response[8] = 1; // 成功标记
                Ok(9) // 8字节句柄 + 1字节成功标记
            }
            None => {
                response[0] = 0; // 失败标记
                Ok(1)
            }
        }
    }
}

/// 处理服务列表
fn handle_list(response: &mut [u8]) -> Result<usize, IpcError> {
    // 简化实现：返回固定响应
    let list_msg = b"IPC Router Service (v0.1)";
    let len = list_msg.len().min(response.len());
    response[..len].copy_from_slice(&list_msg[..len]);
    Ok(len)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        hnxlib::syscall::yield_cpu();
    }
}
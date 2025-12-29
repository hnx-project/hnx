#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::ipc::{IpcError, ServiceFramework};

/// Echo服务主函数
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 创建服务框架
    let framework = match ServiceFramework::new("echo") {
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
///
/// 操作码定义:
/// - 100: Echo请求 - 回显接收到的数据
/// - 101: Ping请求 - 返回固定的Pong响应
/// - 102: 状态查询 - 返回服务状态
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
        
        // 状态查询: 返回服务信息
        102 => {
            const STATUS: &[u8] = b"Echo Service v0.1.0 - Ready";
            let len = STATUS.len().min(response.len());
            response[..len].copy_from_slice(&STATUS[..len]);
            Ok(len)
        }
        
        // 未知操作码
        _ => Err(IpcError::InvalidArgs),
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        hnxlib::syscall::yield_cpu();
    }
}
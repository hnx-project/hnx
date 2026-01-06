#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::ipc::{IpcError, ServiceFramework};

/// 进程管理服务主函数
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 创建服务框架
    let framework = match ServiceFramework::new("procmgr") {
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
/// - 200: 进程状态查询
/// - 201: 启动服务
/// - 202: 停止服务
/// - 203: 服务健康检查
fn handle_message(op: u16, request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    match op {
        // 进程状态查询
        200 => {
            const STATUS: &[u8] = b"Process Manager Service v0.1.0 - Ready";
            let len = STATUS.len().min(response.len());
            response[..len].copy_from_slice(&STATUS[..len]);
            Ok(len)
        }
        
        // 启动服务（占位符）
        201 => {
            const MSG: &[u8] = b"Service start requested (not implemented)";
            let len = MSG.len().min(response.len());
            response[..len].copy_from_slice(&MSG[..len]);
            Ok(len)
        }
        
        // 停止服务（占位符）
        202 => {
            const MSG: &[u8] = b"Service stop requested (not implemented)";
            let len = MSG.len().min(response.len());
            response[..len].copy_from_slice(&MSG[..len]);
            Ok(len)
        }
        
        // 服务健康检查（占位符）
        203 => {
            const MSG: &[u8] = b"All services healthy (monitoring not implemented)";
            let len = MSG.len().min(response.len());
            response[..len].copy_from_slice(&MSG[..len]);
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
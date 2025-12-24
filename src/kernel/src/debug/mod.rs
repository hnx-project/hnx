//! 内核调试接口模块
//! 
//! 提供内核调试功能，包括：
//! - 调试命令解析
//! - 内存状态监控
//! - 进程状态监控
//! - 安全的外部调试接口

#![no_std]

#[allow(unused_imports)]
use crate::memory;
#[allow(unused_imports)]
use crate::process;
#[allow(unused_imports)]
use crate::console;

// 导入必要的类型
use heapless::String;
use heapless::Vec;

pub mod debugger;
pub mod command_parser;
pub mod memory_monitor;
pub mod process_monitor;
#[cfg(test)]
pub mod test;

/// 调试接口错误类型
#[derive(Debug)]
pub enum DebugError {
    /// 命令解析错误
    ParseError(heapless::String<128>),
    /// 权限不足
    PermissionDenied,
    /// 资源未找到
    NotFound,
    /// 内部错误
    InternalError(heapless::String<128>),
}

/// 调试命令结果
pub type DebugResult<T> = Result<T, DebugError>;

/// 初始化调试接口
pub fn init() -> DebugResult<()> {
    // 初始化各个子模块
    debugger::init()?;
    Ok(())
}

/// 处理调试命令
pub fn handle_debug_command(command: &str) -> DebugResult<heapless::String<512>> {
    let parsed_command = command_parser::parse_command(command)?;
    match parsed_command {
        command_parser::Command::MemoryStatus => {
            let status = memory_monitor::get_memory_status()?;
            let mut result = String::<512>::new();
            if result.push_str(&status).is_err() {
                let mut err_msg = String::<128>::new();
                err_msg.push_str("Failed to format result").unwrap_or(());
                return Err(DebugError::InternalError(err_msg));
            }
            Ok(result)
        }
        command_parser::Command::ProcessList => {
            let list = process_monitor::get_process_list()?;
            let mut result = String::<512>::new();
            if result.push_str(&list).is_err() {
                let mut err_msg = String::<128>::new();
                err_msg.push_str("Failed to format result").unwrap_or(());
                return Err(DebugError::InternalError(err_msg));
            }
            Ok(result)
        }
        command_parser::Command::ProcessInfo(pid) => {
            let info = process_monitor::get_process_info(pid)?;
            let mut result = String::<512>::new();
            if result.push_str(&info).is_err() {
                let mut err_msg = String::<128>::new();
                err_msg.push_str("Failed to format result").unwrap_or(());
                return Err(DebugError::InternalError(err_msg));
            }
            Ok(result)
        }
        command_parser::Command::Help => {
            Ok(get_help_text())
        }
    }
}

/// 获取帮助文本
fn get_help_text() -> String<512> {
    let help = "HNX Kernel Debugger Commands:
  memstat          - 显示内存状态
  ps               - 显示进程列表
  ps <pid>         - 显示指定进程信息
  help             - 显示此帮助信息
";
    let mut result = String::<512>::new();
    result.push_str(help).unwrap_or(());
    result
}
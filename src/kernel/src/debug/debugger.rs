//! 内核调试器核心实现

#![no_std]

use super::{DebugResult, DebugError};
use crate::console;

/// 调试会话状态
#[derive(Debug, Clone)]
pub struct DebugSession {
    /// 是否启用调试
    pub enabled: bool,
    /// 调试级别
    pub level: u32,
}

/// 全局调试会话实例
static mut DEBUG_SESSION: DebugSession = DebugSession {
    enabled: false,
    level: 0,
};

/// 初始化调试器
pub fn init() -> DebugResult<()> {
    unsafe {
        DEBUG_SESSION.enabled = true;
        DEBUG_SESSION.level = 1;
    }
    
    // 注意：在no_std环境中，我们不能使用println宏
    // console::println("Debugger initialized");
    Ok(())
}

/// 检查调试权限
pub fn check_debug_permission() -> DebugResult<bool> {
    // 在实际实现中，这里应该检查调用者的权限
    // 目前简化实现，允许所有调试访问
    Ok(true)
}

/// 记录调试信息
pub fn log_debug(message: &str, level: u32) -> DebugResult<()> {
    if !check_debug_permission()? {
        return Err(DebugError::PermissionDenied);
    }
    
    unsafe {
        if DEBUG_SESSION.enabled && level <= DEBUG_SESSION.level {
            // 在no_std环境中，我们不能使用println宏
            // console::println(&format!("[DEBUG {}] {}", level, message));
        }
    }
    
    Ok(())
}

/// 获取调试会话信息
pub fn get_session_info() -> DebugResult<DebugSession> {
    if !check_debug_permission()? {
        return Err(DebugError::PermissionDenied);
    }
    
    unsafe {
        Ok(DEBUG_SESSION.clone())
    }
}
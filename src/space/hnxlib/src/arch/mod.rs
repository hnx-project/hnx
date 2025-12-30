//! 用户空间架构抽象层
//!
//! 提供跨架构的用户空间操作抽象，包括：
//! - 系统调用指令封装
//! - TLS（线程局部存储）操作
//! - 用户空间上下文切换
//! - 原子操作
//!
//! ## 设计原则
//! 1. 应用程序代码应完全架构无关
//! 2. 架构相关代码集中在 `arch/` 目录
//! 3. 通过条件编译选择正确的实现

pub mod common;

// 架构特定实现
#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

// 当前架构的类型别名
#[cfg(target_arch = "aarch64")]
pub use aarch64::AArch64 as CurrentArch;

#[cfg(target_arch = "x86_64")]
pub use x86_64::x86_64 as CurrentArch;

#[cfg(target_arch = "riscv64")]
pub use riscv64::RiscV64 as CurrentArch;

// 为不支持的目标提供占位符
#[cfg(not(any(
    target_arch = "aarch64",
    target_arch = "x86_64",
    target_arch = "riscv64"
)))]
pub use common::UnimplementedArch as CurrentArch;

// 重新导出常用功能
pub use common::{UserArch, UserSyscall, UserTls, UserContext, UserAtomic};
pub use common::{Arch, Syscall, SyscallResult, SyscallError};

/// 架构特定的系统调用实现
///
/// 这是系统调用的低级接口，应用程序应该使用 `Syscall` 或高级包装器。
pub mod syscall {
    use super::*;
    
    /// 执行0参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall0(num: usize) -> usize {
        Arch::syscall0(num)
    }
    
    /// 执行1参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall1(num: usize, arg1: usize) -> usize {
        Arch::syscall1(num, arg1)
    }
    
    /// 执行2参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall2(num: usize, arg1: usize, arg2: usize) -> usize {
        Arch::syscall2(num, arg1, arg2)
    }
    
    /// 执行3参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
        Arch::syscall3(num, arg1, arg2, arg3)
    }
    
    /// 执行4参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> usize {
        Arch::syscall4(num, arg1, arg2, arg3, arg4)
    }
    
    /// 执行5参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall5(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
        Arch::syscall5(num, arg1, arg2, arg3, arg4, arg5)
    }
    
    /// 执行6参数系统调用（低级接口）
    #[inline(always)]
    pub fn syscall6(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> usize {
        Arch::syscall6(num, arg1, arg2, arg3, arg4, arg5, arg6)
    }
}

/// 架构特定的TLS操作
pub mod tls {
    use super::*;
    
    /// 获取当前线程的TLS指针
    #[inline(always)]
    pub fn get_tls<T>() -> *mut T {
        CurrentArch::get_tls() as *mut T
    }
    
    /// 设置当前线程的TLS指针
    #[inline(always)]
    pub fn set_tls<T>(ptr: *mut T) {
        CurrentArch::set_tls(ptr as *mut _)
    }
    
    /// 获取线程指针（原始值）
    #[inline(always)]
    pub fn get_thread_pointer() -> usize {
        CurrentArch::get_thread_pointer()
    }
}

/// 架构特定的原子操作
pub mod atomic {
    use super::*;
    
    /// 比较并交换
    #[inline(always)]
    pub fn compare_exchange(ptr: *mut usize, expected: usize, desired: usize) -> Result<usize, usize> {
        CurrentArch::compare_exchange(ptr, expected, desired)
    }
    
    /// 原子加法
    #[inline(always)]
    pub fn fetch_add(ptr: *mut usize, val: usize) -> usize {
        CurrentArch::fetch_add(ptr, val)
    }
    
    /// 获取加载（带acquire语义）
    #[inline(always)]
    pub fn load_acquire(ptr: *const usize) -> usize {
        CurrentArch::load_acquire(ptr)
    }
    
    /// 释放存储（带release语义）
    #[inline(always)]
    pub fn store_release(ptr: *mut usize, val: usize) {
        CurrentArch::store_release(ptr, val)
    }
}

/// 预导入模块
///
/// 应用程序可以 `use hnxlib::prelude::*` 来导入常用功能
pub mod prelude {
    pub use super::{
        UserArch, UserSyscall, UserTls, UserContext, UserAtomic,
        Arch, Syscall, SyscallResult, SyscallError,
    };
    
    pub use super::syscall::*;
    pub use super::tls::*;
    pub use super::atomic::*;
    
    /// 当前架构常量
    pub const ARCH_NAME: &str = super::CurrentArch::ARCH_NAME;
    pub const STACK_ALIGNMENT: usize = super::CurrentArch::STACK_ALIGNMENT;
    pub const TLS_ALIGNMENT: usize = super::CurrentArch::TLS_ALIGNMENT;
    pub const SYSCALL_INSTRUCTION: &str = super::CurrentArch::SYSCALL_INSTRUCTION;
}
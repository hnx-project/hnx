//! 用户空间架构通用定义
//!
//! 包含跨架构共享的类型定义和辅助函数。

pub mod traits;

// 重新导出主要trait
pub use traits::{
    UserSyscall, 
    UserTls, 
    UserContext, 
    UserAtomic, 
    UserArch,
    UnimplementedArch,
};

/// 系统调用错误处理
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    /// 无效参数
    InvalidArgument,
    /// 内存访问错误
    MemoryFault,
    /// 资源不足
    NoResources,
    /// 权限不足
    PermissionDenied,
    /// 不支持的操作
    NotSupported,
    /// 其他错误
    Other(isize),
}

impl SyscallError {
    /// 从系统调用返回值创建错误
    pub fn from_return_value(ret: isize) -> Option<Self> {
        if ret >= 0 {
            None
        } else {
            match ret {
                -1 => Some(SyscallError::InvalidArgument),
                -2 => Some(SyscallError::MemoryFault),
                -3 => Some(SyscallError::NoResources),
                -4 => Some(SyscallError::PermissionDenied),
                -5 => Some(SyscallError::NotSupported),
                _ => Some(SyscallError::Other(ret)),
            }
        }
    }
}

/// 系统调用结果类型
pub type SyscallResult<T> = Result<T, SyscallError>;

/// 通用系统调用包装器
///
/// 提供类型安全的系统调用接口
pub struct Syscall;

impl Syscall {
    /// 执行系统调用并检查错误
    pub fn call0(num: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall0(num))
    }
    
    /// 执行1参数系统调用
    pub fn call1(num: usize, arg1: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall1(num, arg1))
    }
    
    /// 执行2参数系统调用
    pub fn call2(num: usize, arg1: usize, arg2: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall2(num, arg1, arg2))
    }
    
    /// 执行3参数系统调用
    pub fn call3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall3(num, arg1, arg2, arg3))
    }
    
    /// 执行4参数系统调用
    pub fn call4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall4(num, arg1, arg2, arg3, arg4))
    }
    
    /// 执行5参数系统调用
    pub fn call5(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall5(num, arg1, arg2, arg3, arg4, arg5))
    }
    
    /// 执行6参数系统调用
    pub fn call6(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> SyscallResult<usize> {
        Self::check_result(Arch::syscall6(num, arg1, arg2, arg3, arg4, arg5, arg6))
    }
    
    /// 检查系统调用返回值
    fn check_result(ret: usize) -> SyscallResult<usize> {
        match SyscallError::from_return_value(ret as isize) {
            Some(err) => Err(err),
            None => Ok(ret),
        }
    }
}

/// 当前架构的动态分发
///
/// 通过条件编译选择正确的架构实现
pub struct Arch;

impl Arch {
    /// 获取当前架构的系统调用实现
    pub fn syscall0(num: usize) -> usize {
        ArchImpl::syscall0(num)
    }
    
    pub fn syscall1(num: usize, arg1: usize) -> usize {
        ArchImpl::syscall1(num, arg1)
    }
    
    pub fn syscall2(num: usize, arg1: usize, arg2: usize) -> usize {
        ArchImpl::syscall2(num, arg1, arg2)
    }
    
    pub fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
        ArchImpl::syscall3(num, arg1, arg2, arg3)
    }
    
    pub fn syscall4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> usize {
        ArchImpl::syscall4(num, arg1, arg2, arg3, arg4)
    }
    
    pub fn syscall5(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
        ArchImpl::syscall5(num, arg1, arg2, arg3, arg4, arg5)
    }
    
    pub fn syscall6(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> usize {
        ArchImpl::syscall6(num, arg1, arg2, arg3, arg4, arg5, arg6)
    }
}

/// 条件编译选择的架构实现类型
#[cfg(target_arch = "aarch64")]
type ArchImpl = crate::arch::aarch64::AArch64;

#[cfg(target_arch = "x86_64")]
type ArchImpl = crate::arch::x86_64::x86_64;

#[cfg(target_arch = "riscv64")]
type ArchImpl = crate::arch::riscv64::RiscV64;

#[cfg(not(any(
    target_arch = "aarch64",
    target_arch = "x86_64",
    target_arch = "riscv64"
)))]
type ArchImpl = UnimplementedArch;
//! 内核架构通用定义
//!
//! 包含跨架构共享的类型定义和抽象接口。

pub mod traits;

// 重新导出主要trait
pub use traits::{
    Arch as ArchTrait, Cpu, Mmu, InterruptController, Timer, Platform,
    UnimplementedArch,
};

/// 架构特定的错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchError {
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
    /// 超时
    Timeout,
    /// 其他错误
    Other(isize),
}

impl ArchError {
    /// 转换为系统错误代码
    pub fn to_sys_error(self) -> crate::object::traits::ObjectError {
        use crate::object::traits::ObjectError;
        match self {
            ArchError::InvalidArgument => ObjectError::InvalidArgs,
            ArchError::MemoryFault => ObjectError::NoMemory,
            ArchError::NoResources => ObjectError::NoMemory,
            ArchError::PermissionDenied => ObjectError::PermissionDenied,
            ArchError::NotSupported => ObjectError::NotSupported,
            ArchError::Timeout => ObjectError::TimedOut,
            ArchError::Other(_) => ObjectError::InvalidArgs,
        }
    }
}

/// 架构结果类型
pub type ArchResult<T> = Result<T, ArchError>;

/// 当前架构的动态分发
///
/// 通过条件编译选择正确的架构实现
pub struct Arch;

impl Arch {
    /// 获取当前CPU ID
    pub fn cpu_id() -> u32 {
        ArchImpl::cpu_id()
    }
    
    /// 暂停CPU直到中断发生
    pub fn cpu_wait_for_interrupt() {
        ArchImpl::cpu_wait_for_interrupt()
    }
    
    /// 使能中断
    pub fn cpu_enable_interrupts() {
        ArchImpl::cpu_enable_interrupts()
    }
    
    /// 禁用中断
    pub fn cpu_disable_interrupts() {
        ArchImpl::cpu_disable_interrupts()
    }
    
    /// 判断中断是否使能
    pub fn cpu_interrupts_enabled() -> bool {
        ArchImpl::cpu_interrupts_enabled()
    }
    
    /// 初始化MMU
    pub fn mmu_init() {
        ArchImpl::mmu_init()
    }
    
    /// 启用MMU
    pub fn mmu_enable() {
        ArchImpl::mmu_enable()
    }
    
    /// 禁用MMU
    pub fn mmu_disable() {
        ArchImpl::mmu_disable()
    }
    
    /// 创建内核页表
    pub fn mmu_create_kernel_page_table() -> crate::memory::PageTable {
        ArchImpl::mmu_create_kernel_page_table()
    }
    
    /// 初始化中断控制器
    pub fn interrupt_init() {
        ArchImpl::interrupt_init()
    }
    
    /// 使能特定中断
    pub fn interrupt_enable(irq: u32) {
        ArchImpl::interrupt_enable(irq)
    }
    
    /// 禁用特定中断
    pub fn interrupt_disable(irq: u32) {
        ArchImpl::interrupt_disable(irq)
    }
    
    /// 处理中断
    pub fn interrupt_handle(irq: u32) {
        ArchImpl::interrupt_handle(irq)
    }
    
    /// 初始化定时器
    pub fn timer_init() {
        ArchImpl::timer_init()
    }
    
    /// 获取当前时间（纳秒）
    pub fn timer_now() -> u64 {
        ArchImpl::timer_now()
    }
    
    /// 设置定时器中断（相对时间，纳秒）
    pub fn timer_set_interval(ns: u64) {
        ArchImpl::timer_set_interval(ns)
    }
    
    /// 初始化平台
    pub fn platform_init() {
        ArchImpl::platform_init()
    }
    
    /// 获取平台名称
    pub fn platform_name() -> &'static str {
        ArchImpl::platform_name()
    }
    
    /// 关机系统
    pub fn platform_shutdown() -> ! {
        ArchImpl::platform_shutdown()
    }
    
    /// 重启系统
    pub fn platform_reboot() -> ! {
        ArchImpl::platform_reboot()
    }
}

/// 条件编译选择的架构实现类型
#[cfg(target_arch = "aarch64")]
type ArchImpl = crate::arch::aarch64::AArch64;

#[cfg(target_arch = "x86_64")]
type ArchImpl = crate::arch::x86_64::X86_64;

#[cfg(target_arch = "riscv64")]
type ArchImpl = crate::arch::riscv64::RiscV64;

#[cfg(not(any(
    target_arch = "aarch64",
    target_arch = "x86_64",
    target_arch = "riscv64"
)))]
type ArchImpl = UnimplementedArch;
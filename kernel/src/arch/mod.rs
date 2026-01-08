//! 内核架构抽象层
//!
//! 提供跨架构的内核操作抽象，包括：
//! - CPU 控制和异常处理
//! - 内存管理单元（MMU）和页表
//! - 中断控制器（GIC）管理
//! - 定时器和时钟源
//! - 多核启动（SMP）
//! - 平台特定功能（PSCI、SMC调用等）
//!
//! ## 设计原则
//! 1. 内核核心代码应尽量架构无关
//! 2. 架构相关代码集中在 `arch/` 目录下
//! 3. 通过条件编译选择正确的实现
//! 4. 使用trait提供通用接口
//! 5. 允许架构特定的优化和扩展

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
pub use x86_64::X86_64 as CurrentArch;

#[cfg(target_arch = "riscv64")]
pub use riscv64::RiscV64 as CurrentArch;

// 重新导出常用功能
pub use common::{Arch, Cpu, Mmu, InterruptController, Timer, Platform};

/// 架构特定的CPU操作
pub mod cpu {
    use super::*;
    
    /// 获取当前CPU ID
    #[inline(always)]
    pub fn id() -> u32 {
        CurrentArch::cpu_id()
    }
    
    /// 暂停CPU直到中断发生
    #[inline(always)]
    pub fn wait_for_interrupt() {
        CurrentArch::cpu_wait_for_interrupt();
    }
    
    /// 使能中断
    #[inline(always)]
    pub fn enable_interrupts() {
        CurrentArch::cpu_enable_interrupts();
    }
    
    /// 禁用中断
    #[inline(always)]
    pub fn disable_interrupts() {
        CurrentArch::cpu_disable_interrupts();
    }
    
    /// 判断中断是否使能
    #[inline(always)]
    pub fn interrupts_enabled() -> bool {
        CurrentArch::cpu_interrupts_enabled()
    }
}

/// 架构特定的内存管理操作
pub mod mmu {
    use super::*;
    
    /// 初始化MMU
    pub fn init() {
        CurrentArch::mmu_init();
    }
    
    /// 启用MMU
    pub fn enable() {
        CurrentArch::mmu_enable();
    }
    
    /// 禁用MMU
    pub fn disable() {
        CurrentArch::mmu_disable();
    }
    
    /// 创建内核页表
    pub fn create_kernel_page_table() -> crate::memory::PageTable {
        CurrentArch::mmu_create_kernel_page_table()
    }
}

/// 架构特定的中断操作
pub mod interrupt {
    use super::*;
    
    /// 初始化中断控制器
    pub fn init() {
        CurrentArch::interrupt_init();
    }
    
    /// 使能特定中断
    pub fn enable(irq: u32) {
        CurrentArch::interrupt_enable(irq);
    }
    
    /// 禁用特定中断
    pub fn disable(irq: u32) {
        CurrentArch::interrupt_disable(irq);
    }
    
    /// 处理中断
    pub fn handle(irq: u32) {
        CurrentArch::interrupt_handle(irq);
    }
}

/// 架构特定的定时器操作
pub mod timer {
    use super::*;
    
    /// 初始化定时器
    pub fn init() {
        CurrentArch::timer_init();
    }
    
    /// 获取当前时间（纳秒）
    pub fn now() -> u64 {
        CurrentArch::timer_now()
    }
    
    /// 设置定时器中断（相对时间，纳秒）
    pub fn set_interval(ns: u64) {
        CurrentArch::timer_set_interval(ns);
    }
}

/// 架构特定的平台操作
pub mod platform {
    use super::*;
    
    /// 初始化平台
    pub fn init() {
        CurrentArch::platform_init();
    }
    
    /// 获取平台名称
    pub fn name() -> &'static str {
        CurrentArch::platform_name()
    }
    
    /// 关机系统
    pub fn shutdown() -> ! {
        CurrentArch::platform_shutdown()
    }
    
    /// 重启系统
    pub fn reboot() -> ! {
        CurrentArch::platform_reboot()
    }
}

/// 预导入模块
///
/// 内核代码可以 `use crate::arch::prelude::*` 来导入常用功能
pub mod prelude {
    pub use super::{
        Arch, Cpu, Mmu, InterruptController, Timer, Platform,
    };

    pub use super::cpu::*;
    pub use super::mmu::*;
    pub use super::interrupt::*;
    pub use super::timer::*;
    pub use super::platform::*;

    /// 当前架构常量
    pub const ARCH_NAME: &str = <super::CurrentArch as super::common::traits::Arch>::ARCH_NAME;
    pub const PAGE_SIZE: usize = <super::CurrentArch as super::common::traits::Arch>::PAGE_SIZE;
    pub const PAGE_SIZE_BITS: usize = <super::CurrentArch as super::common::traits::Arch>::PAGE_SIZE_BITS;
}

// 重新导出常用常量和类型
pub use prelude::{ARCH_NAME, PAGE_SIZE, PAGE_SIZE_BITS};
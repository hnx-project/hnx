//! 内核架构抽象接口定义
//!
//! 这些trait定义了内核需要处理的架构差异：
//! 1. CPU控制和状态管理
//! 2. 内存管理单元（MMU）和页表
//! 3. 中断控制器（GIC）管理
//! 4. 定时器和时钟源
//! 5. 平台特定功能（PSCI、SMC调用等）

use crate::arch::common::ArchError;

/// CPU控制和状态管理
pub trait Cpu {
    /// 获取当前CPU ID
    fn cpu_id() -> u32;
    
    /// 暂停CPU直到中断发生
    fn cpu_wait_for_interrupt();
    
    /// 使能中断
    fn cpu_enable_interrupts();
    
    /// 禁用中断
    fn cpu_disable_interrupts();
    
    /// 判断中断是否使能
    fn cpu_interrupts_enabled() -> bool;
    
    /// 内存屏障（确保所有内存访问完成）
    fn memory_barrier();
    
    /// 指令同步屏障（确保所有指令完成）
    fn instruction_barrier();
    
    /// 数据同步屏障（确保所有数据访问完成）
    fn data_barrier();
}

/// 内存管理单元（MMU）和页表
pub trait Mmu {
    /// 初始化MMU
    fn mmu_init();
    
    /// 启用MMU
    fn mmu_enable();
    
    /// 禁用MMU
    fn mmu_disable();
    
    /// 创建内核页表
    fn mmu_create_kernel_page_table() -> crate::memory::PageTable;
    
    /// 映射虚拟地址到物理地址
    fn mmu_map(
        page_table: &mut crate::memory::PageTable,
        vaddr: usize,
        paddr: usize,
        size: usize,
        flags: crate::memory::MapFlags,
    ) -> Result<(), ArchError>;
    
    /// 取消虚拟地址映射
    fn mmu_unmap(
        page_table: &mut crate::memory::PageTable,
        vaddr: usize,
        size: usize,
    ) -> Result<(), ArchError>;
}

/// 中断控制器管理
pub trait InterruptController {
    /// 初始化中断控制器
    fn interrupt_init();
    
    /// 使能特定中断
    fn interrupt_enable(irq: u32);
    
    /// 禁用特定中断
    fn interrupt_disable(irq: u32);
    
    /// 处理中断
    fn interrupt_handle(irq: u32);
    
    /// 发送EOI（中断结束）
    fn interrupt_send_eoi(irq: u32);
    
    /// 设置中断优先级
    fn interrupt_set_priority(irq: u32, priority: u8);
    
    /// 设置中断目标CPU
    fn interrupt_set_target(irq: u32, cpu_mask: u8);
}

/// 定时器和时钟源
pub trait Timer {
    /// 初始化定时器
    fn timer_init();
    
    /// 获取当前时间（纳秒）
    fn timer_now() -> u64;
    
    /// 设置定时器中断（相对时间，纳秒）
    fn timer_set_interval(ns: u64);
    
    /// 使能定时器中断
    fn timer_enable();
    
    /// 禁用定时器中断
    fn timer_disable();
    
    /// 获取定时器频率（Hz）
    fn timer_frequency() -> u64;
}

/// 平台特定功能
pub trait Platform {
    /// 初始化平台
    fn platform_init();
    
    /// 获取平台名称
    fn platform_name() -> &'static str;
    
    /// 关机系统
    fn platform_shutdown() -> !;
    
    /// 重启系统
    fn platform_reboot() -> !;
    
    /// 获取内存布局
    fn platform_memory_layout() -> &'static [crate::memory::MemoryRegion];
    
    /// 获取设备树地址（如果可用）
    fn platform_device_tree() -> Option<usize>;
}

/// 聚合所有架构相关操作
pub trait Arch: Cpu + Mmu + InterruptController + Timer + Platform {
    /// 架构名称
    const ARCH_NAME: &'static str;
    
    /// 页大小（字节）
    const PAGE_SIZE: usize;
    
    /// 页大小位宽（log2(PAGE_SIZE)）
    const PAGE_SIZE_BITS: usize;
    
    /// 虚拟地址位宽
    const VADDR_BITS: usize;
    
    /// 物理地址位宽
    const PADDR_BITS: usize;
    
    /// 最大支持CPU数量
    const MAX_CPUS: u32;
}

/// 默认实现占位符
///
/// 用于在没有特定架构实现时提供编译时错误
pub struct UnimplementedArch;

impl Cpu for UnimplementedArch {
    fn cpu_id() -> u32 {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn cpu_wait_for_interrupt() {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn cpu_enable_interrupts() {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn cpu_disable_interrupts() {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn cpu_interrupts_enabled() -> bool {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn memory_barrier() {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn instruction_barrier() {
        unimplemented!("Cpu not implemented for this architecture")
    }
    
    fn data_barrier() {
        unimplemented!("Cpu not implemented for this architecture")
    }
}

impl Mmu for UnimplementedArch {
    fn mmu_init() {
        unimplemented!("Mmu not implemented for this architecture")
    }
    
    fn mmu_enable() {
        unimplemented!("Mmu not implemented for this architecture")
    }
    
    fn mmu_disable() {
        unimplemented!("Mmu not implemented for this architecture")
    }
    
    fn mmu_create_kernel_page_table() -> crate::memory::PageTable {
        unimplemented!("Mmu not implemented for this architecture")
    }
    
    fn mmu_map(
        _page_table: &mut crate::memory::PageTable,
        _vaddr: usize,
        _paddr: usize,
        _size: usize,
        _flags: crate::memory::MapFlags,
    ) -> Result<(), ArchError> {
        unimplemented!("Mmu not implemented for this architecture")
    }
    
    fn mmu_unmap(
        _page_table: &mut crate::memory::PageTable,
        _vaddr: usize,
        _size: usize,
    ) -> Result<(), ArchError> {
        unimplemented!("Mmu not implemented for this architecture")
    }
}

impl InterruptController for UnimplementedArch {
    fn interrupt_init() {
        unimplemented!("InterruptController not implemented for this architecture")
    }
    
    fn interrupt_enable(_irq: u32) {
        unimplemented!("InterruptController not implemented for this architecture")
    }
    
    fn interrupt_disable(_irq: u32) {
        unimplemented!("InterruptController not implemented for this architecture")
    }
    
    fn interrupt_handle(_irq: u32) {
        unimplemented!("InterruptController not implemented for this architecture")
    }
    
    fn interrupt_send_eoi(_irq: u32) {
        unimplemented!("InterruptController not implemented for this architecture")
    }
    
    fn interrupt_set_priority(_irq: u32, _priority: u8) {
        unimplemented!("InterruptController not implemented for this architecture")
    }
    
    fn interrupt_set_target(_irq: u32, _cpu_mask: u8) {
        unimplemented!("InterruptController not implemented for this architecture")
    }
}

impl Timer for UnimplementedArch {
    fn timer_init() {
        unimplemented!("Timer not implemented for this architecture")
    }
    
    fn timer_now() -> u64 {
        unimplemented!("Timer not implemented for this architecture")
    }
    
    fn timer_set_interval(_ns: u64) {
        unimplemented!("Timer not implemented for this architecture")
    }
    
    fn timer_enable() {
        unimplemented!("Timer not implemented for this architecture")
    }
    
    fn timer_disable() {
        unimplemented!("Timer not implemented for this architecture")
    }
    
    fn timer_frequency() -> u64 {
        unimplemented!("Timer not implemented for this architecture")
    }
}

impl Platform for UnimplementedArch {
    fn platform_init() {
        unimplemented!("Platform not implemented for this architecture")
    }
    
    fn platform_name() -> &'static str {
        unimplemented!("Platform not implemented for this architecture")
    }
    
    fn platform_shutdown() -> ! {
        unimplemented!("Platform not implemented for this architecture")
    }
    
    fn platform_reboot() -> ! {
        unimplemented!("Platform not implemented for this architecture")
    }
    
    fn platform_memory_layout() -> &'static [crate::memory::MemoryRegion] {
        unimplemented!("Platform not implemented for this architecture")
    }
    
    fn platform_device_tree() -> Option<usize> {
        unimplemented!("Platform not implemented for this architecture")
    }
}

impl Arch for UnimplementedArch {
    const ARCH_NAME: &'static str = "unimplemented";
    const PAGE_SIZE: usize = 4096;
    const PAGE_SIZE_BITS: usize = 12;
    const VADDR_BITS: usize = 48;
    const PADDR_BITS: usize = 48;
    const MAX_CPUS: u32 = 1;
}
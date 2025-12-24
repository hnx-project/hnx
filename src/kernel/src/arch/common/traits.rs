// 架构必须实现的 trait
use crate::arch::common::mmu::MmuFlags;
pub type Result<T> = core::result::Result<T, ()>;
pub trait Arch {
    /// 架构名称
    const NAME: &'static str;

    /// 初始化架构特定部分
    fn init();

    /// 获取当前 CPU ID
    fn cpu_id() -> u32;

    /// 停止 CPU
    fn halt() -> !;
}

// MMU trait
pub trait Mmu {
    /// 初始化页表
    fn init_page_table();

    /// 映射虚拟地址到物理地址
    fn map(vaddr: usize, paddr: usize, flags: MmuFlags) -> Result<()>;

    /// 取消映射
    fn unmap(vaddr: usize) -> Result<()>;

    /// 获取物理地址
    fn virt_to_phys(vaddr: usize) -> Option<usize>;
}

// 中断控制器 trait
pub trait InterruptController {
    fn init();
    fn enable_irq(irq: u32);
    fn disable_irq(irq: u32);
    fn ack_irq(irq: u32);
}

// 定时器 trait
pub trait Timer {
    fn init(frequency: u64);
    fn set_timeout(ms: u64, callback: fn());
    fn get_ticks() -> u64;
}

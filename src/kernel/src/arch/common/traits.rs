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

// 内存操作 trait (屏障、TLB、缓存)
pub trait Memory {
    /// 数据同步屏障 (Data Synchronization Barrier)
    fn data_sync_barrier();

    /// 指令同步屏障 (Instruction Synchronization Barrier)
    fn instruction_barrier();

    /// 数据内存屏障 (Data Memory Barrier)
    fn data_memory_barrier();

    /// 刷新整个 TLB (所有条目)
    fn tlb_flush_all();

    /// 按虚拟地址使 TLB 条目无效 (可选的 ASID)
    fn tlb_invalidate(vaddr: usize, asid: Option<u16>);

    /// 获取当前 ASID (地址空间标识符)
    fn get_current_asid() -> u16;

    /// 清理数据缓存范围
    fn clean_dcache_range(addr: usize, size: usize);

    /// 刷新指令缓存全部
    fn flush_icache_all();

    /// 让出 CPU 执行权 (用于自旋等待)
    fn yield_cpu();

    /// 获取当前页表基址 (TTBR0)
    fn get_current_page_table_base() -> usize;

    /// 设置当前页表基址 (TTBR0)
    ///
    /// # 参数
    /// - `base`: 页表物理基址
    /// - `asid`: 可选的地址空间标识符 (ASID)
    fn set_current_page_table_base(base: usize, asid: Option<u16>);
}

// CPU 操作 trait
pub trait Cpu {
    /// 获取当前 CPU 核心 ID
    fn id() -> u32;

    /// 让出 CPU 执行权 (用于自旋等待)
    fn yield_cpu();

    /// 内存屏障 (全功能屏障)
    fn barrier();

    /// 读内存屏障 (Load-Load 和 Load-Store)
    fn read_barrier();

    /// 写内存屏障 (Store-Store)
    fn write_barrier();
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

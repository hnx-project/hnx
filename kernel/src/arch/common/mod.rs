// 架构通用工具
//
// 提供所有架构共享的通用功能。

pub mod registers;

// 通用类型定义
pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;
pub type PageNumber = usize;
pub type FrameNumber = usize;

// 页面大小常量
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

/// 对齐到页面边界
pub const fn align_to_page(addr: usize) -> usize {
    (addr + PAGE_MASK) & !PAGE_MASK
}

/// 是否页面对齐
pub const fn is_page_aligned(addr: usize) -> bool {
    addr & PAGE_MASK == 0
}

/// 转换为页号
pub const fn addr_to_page(addr: usize) -> PageNumber {
    addr >> PAGE_SHIFT
}

/// 转换为地址
pub const fn page_to_addr(page: PageNumber) -> usize {
    page << PAGE_SHIFT
}

/// 内存屏障类型
pub enum MemoryBarrier {
    /// 读屏障
    Read,
    /// 写屏障
    Write,
    /// 全屏障
    Full,
}

/// 缓存操作类型
pub enum CacheOperation {
    /// 无效化
    Invalidate,
    /// 清理
    Clean,
    /// 清理并无效化
    CleanAndInvalidate,
}

/// 缓存范围
pub struct CacheRange {
    pub start: usize,
    pub end: usize,
}

impl CacheRange {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "Invalid cache range");
        Self { start, end }
    }

    pub fn size(&self) -> usize {
        self.end - self.start
    }
}

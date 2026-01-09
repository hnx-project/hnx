// MMU/内存管理单元抽象特征
//
// 要求实现：
// - 页表管理
// - 地址空间管理
// - TLB管理
// - 内存属性配置
// - 大页支持
// - ASID管理

use super::super::ArchResult;

/// MMU架构抽象特征
pub trait MmuArch: Send + Sync {
    /// 页表对象类型
    type PageTable: PageTable;
    
    /// 页表项对象类型
    type PageTableEntry: PageTableEntry;
    
    /// 地址空间描述符
    type AddressSpace: AddressSpace;
    
    /// 物理内存区域
    type MemoryRegion: MemoryRegion;
    
    /// 初始化MMU
    fn init() -> ArchResult<()>;
    
    /// 创建新页表
    fn create_page_table() -> ArchResult<Self::PageTable>;
    
    /// 获取当前页表
    fn current_page_table() -> Self::PageTable;
    
    /// 切换到指定页表
    fn switch_page_table(table: &Self::PageTable);
    
    /// 映射虚拟地址到物理地址
    fn map(
        table: &mut Self::PageTable,
        vaddr: usize,
        paddr: usize,
        size: usize,
        flags: MappingFlags,
    ) -> ArchResult<()>;
    
    /// 取消映射
    fn unmap(
        table: &mut Self::PageTable,
        vaddr: usize,
        size: usize,
    ) -> ArchResult<()>;
    
    /// 查询映射
    fn query(
        table: &Self::PageTable,
        vaddr: usize,
    ) -> ArchResult<MappingInfo>;
    
    /// 刷新TLB
    fn tlb_flush(vaddr: Option<usize>, asid: Option<u16>);
    
    /// 无效化缓存
    fn cache_invalidate(vaddr: usize, size: usize);
    
    /// 获取页面大小
    fn page_size() -> usize;
    
    /// 支持的大页大小列表
    fn supported_huge_page_sizes() -> &'static [usize];
    
    /// 分配物理页面
    fn allocate_physical_page() -> ArchResult<usize>;
    
    /// 释放物理页面
    fn free_physical_page(paddr: usize) -> ArchResult<()>;
}

/// 页表特征
pub trait PageTable: Send + Sync + Clone {
    /// 获取根地址
    fn root_address(&self) -> usize;
    
    /// 获取ASID
    fn asid(&self) -> u16;
    
    /// 设置ASID
    fn set_asid(&mut self, asid: u16);
    
    /// 复制页表
    fn clone_with_asid(&self, asid: u16) -> ArchResult<Self>;
    
    /// 统计信息
    fn stats(&self) -> PageTableStats;
}

/// 页表项特征
pub trait PageTableEntry: Send + Sync {
    /// 创建无效项
    fn invalid() -> Self;
    
    /// 创建页表项
    fn new(paddr: usize, flags: EntryFlags) -> Self;
    
    /// 是否有效
    fn is_valid(&self) -> bool;
    
    /// 获取物理地址
    fn physical_address(&self) -> usize;
    
    /// 获取标志位
    fn flags(&self) -> EntryFlags;
    
    /// 设置访问位
    fn set_accessed(&mut self, accessed: bool);
    
    /// 设置脏位
    fn set_dirty(&mut self, dirty: bool);
}

/// 地址空间特征
pub trait AddressSpace: Send + Sync {
    /// 创建新页表
    fn new() -> ArchResult<Self> where Self: Sized;
    
    /// 映射内存区域
    fn map_region(
        &mut self,
        vaddr: usize,
        region: &dyn MemoryRegion,
        flags: MappingFlags,
    ) -> ArchResult<()>;
    
    /// 分配虚拟内存
    fn allocate(
        &mut self,
        size: usize,
        alignment: usize,
        flags: AllocationFlags,
    ) -> ArchResult<VirtualMemory>;
    
    /// 释放虚拟内存
    fn free(&mut self, vmem: VirtualMemory) -> ArchResult<()>;
}

/// 内存区域特征
pub trait MemoryRegion: Send + Sync {
    /// 起始地址
    fn base(&self) -> usize;
    
    /// 大小
    fn size(&self) -> usize;
    
    /// 类型
    fn memory_type(&self) -> MemoryType;
    
    /// 是否可缓存
    fn is_cacheable(&self) -> bool;
    
    /// 是否可执行
    fn is_executable(&self) -> bool;
    
    /// 是否可写
    fn is_writable(&self) -> bool;
    
    /// 是否设备内存
    fn is_device(&self) -> bool;
}

/// 内存类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// 普通内存（可缓存）
    Normal,
    /// 设备内存（不可缓存）
    Device,
    /// 预留内存
    Reserved,
    /// ACPI内存
    Acpi,
    /// 不可用内存
    Unusable,
}

/// 映射标志位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingFlags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub user: bool,
    pub global: bool,
    pub no_cache: bool,
    pub write_through: bool,
    pub write_combine: bool,
}

/// 条目标志位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryFlags {
    pub present: bool,
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub global: bool,
    pub accessed: bool,
    pub dirty: bool,
    pub huge_page: bool,
}

/// 映射信息
#[derive(Debug, Clone, Copy)]
pub struct MappingInfo {
    pub paddr: usize,
    pub size: usize,
    pub flags: MappingFlags,
}

/// 虚拟内存对象
#[derive(Debug, Clone)]
pub struct VirtualMemory {
    pub vaddr: usize,
    pub size: usize,
    pub flags: AllocationFlags,
}

/// 分配标志位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub guard_page: bool,
}

/// 页表统计信息
#[derive(Debug, Clone, Copy)]
pub struct PageTableStats {
    pub total_pages: usize,
    pub used_pages: usize,
    pub huge_pages: usize,
}

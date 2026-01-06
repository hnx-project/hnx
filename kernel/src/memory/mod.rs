//! Memory Management Subsystem
//!
//! This module provides the core memory management functionality for the HNX microkernel:
//! - **Physical Memory**: Page frame allocation using buddy allocator
//! - **Virtual Memory**: Address space management and page table operations
//! - **Memory Protection**: Guard pages, stack canaries, ASLR
//! - **Allocators**: Buddy allocator (large blocks), Slab allocator (small objects)
//! - **DMA**: Direct Memory Access support
//! - **Memory Mapping**: User space memory mapping management

#![allow(dead_code)]

pub mod physical;
pub mod virt;
pub mod protection;
pub mod allocator;

// Import types needed for memory management
use core::ops::Range;
use shared::sync::mutex::Mutex;
use crate::memory::allocator::dma::DmaAllocator;
use crate::memory::allocator::buddy::BuddyAllocator;
use crate::memory::allocator::slab::SlabAllocator;

// Re-export commonly used types and functions
pub use physical::{PhysAddr, alloc_pages, free_pages, stats as physical_stats, check_invariants};
pub use virt::{VirtAddr, KERNEL_BASE, map, unmap, create_user_l1, map_in_pt};
pub use protection::{
    add_guard_page, 
    initialize_stack_canary, 
    verify_stack_canary,
    randomize_address_space,
    validate_memory_access,
};
pub use allocator::buddy::ALLOCATOR as BUDDY_ALLOCATOR;
pub use allocator::slab::SLAB_ALLOCATOR;

/// 内存映射类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapType {
    /// 可读写的RAM内存
    Ram,
    /// 只读的ROM内存
    Rom,
    /// 设备内存映射
    Device,
    /// 匿名映射
    Anonymous,
    /// 文件映射
    File,
    /// 保留区域
    Reserved,
}

/// 内存映射区域标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMapFlags {
    /// 可读
    pub readable: bool,
    /// 可写
    pub writable: bool,
    /// 可执行
    pub executable: bool,
    /// 用户可访问
    pub user_accessible: bool,
    /// 缓存策略
    pub cacheable: bool,
}

impl MemoryMapFlags {
    /// 创建新的标志
    pub const fn new(readable: bool, writable: bool, executable: bool, user_accessible: bool, cacheable: bool) -> Self {
        Self {
            readable,
            writable,
            executable,
            user_accessible,
            cacheable,
        }
    }

    /// 默认RAM权限（可读写，用户不可访问）
    pub const fn ram_default() -> Self {
        Self::new(true, true, false, false, true)
    }

    /// 默认ROM权限（只读，用户可访问）
    pub const fn rom_default() -> Self {
        Self::new(true, false, false, true, true)
    }

    /// 默认设备权限（可读写，不缓存）
    pub const fn device_default() -> Self {
        Self::new(true, true, false, false, false)
    }
}

/// 内存映射区域
#[derive(Debug, Clone)]
pub struct MemoryMapEntry {
    /// 虚拟地址范围
    pub range: Range<usize>,
    /// 内存映射类型
    pub map_type: MemoryMapType,
    /// 内存映射标志
    pub flags: MemoryMapFlags,
    /// 相关数据（如文件描述符等）
    pub data: Option<u64>,
}

impl MemoryMapEntry {
    /// 创建新的内存映射区域
    pub fn new(start: usize, size: usize, map_type: MemoryMapType, flags: MemoryMapFlags) -> Self {
        Self {
            range: Range { start, end: start + size },
            map_type,
            flags,
            data: None,
        }
    }

    /// 获取映射区域大小
    pub fn size(&self) -> usize {
        self.range.end - self.range.start
    }

    /// 检查是否与另一个区域重叠
    pub fn overlaps(&self, other: &Self) -> bool {
        self.range.start < other.range.end && other.range.start < self.range.end
    }

    /// 检查是否包含指定地址
    pub fn contains(&self, addr: usize) -> bool {
        self.range.contains(&addr)
    }
}

/// 内存管理器
pub struct MemoryManager {
    /// 内存映射区域列表
    entries: heapless::Vec<MemoryMapEntry, 64>, // 最多支持64个映射区域
    /// DMA 分配器
    pub dma_allocator: DmaAllocator,
    /// 伙伴分配器
    pub buddy_allocator: BuddyAllocator,
    /// slab 分配器
    pub slab_allocator: SlabAllocator,
}

impl MemoryManager {
    /// 创建新的内存映射管理器
    pub(crate) const fn new() -> Self {
        Self {
            entries: heapless::Vec::new(),
            dma_allocator: DmaAllocator::new(),
            buddy_allocator: BuddyAllocator::new(),
            slab_allocator: SlabAllocator::new(),
        }
    }

    /// 初始化内存管理器
    /// 1. 获取物理内存布局
    /// 2. 初始化物理页帧管理结构
    /// 3. 初始化 Buddy 分配器
    /// 4. 建立内核虚拟内存映射（若未提前建立）
    /// 5. 初始化 slab 分配器（依赖 Buddy）
    /// 6. 初始化 DMA 内存管理（依赖物理内存布局）
    /// 7. 初始化完整虚拟内存管理器（依赖 slab 和 Buddy）
    /// 8. 启动内存回收线程等后台服务
    /// 9. 内存管理器自检并输出状态
    pub fn init(&mut self) {
        crate::info!("Initializing memory manager subsystem...");

        // 阶段 1: 物理内存初始化
        crate::info!("[MM] Phase 1: Initializing physical memory...");
        let boot_info = crate::arch::boot::get_boot_info();
        physical::init(boot_info);
        let phys_stats = physical::stats();
        crate::info!("[MM] Physical memory map: {} pages available.", phys_stats.total_free_pages);

        // 阶段 2: Buddy 分配器初始化
        crate::info!("[MM] Phase 2: Initializing Buddy allocator...");
        unsafe { self.buddy_allocator.init(boot_info.phys_mem_start as usize, boot_info.phys_mem_size as usize) };

        // 阶段 3: 内核虚拟内存映射初始化
        crate::info!("[MM] Phase 3: Initializing kernel virtual memory...");
        virt::init(); // 这将注册内核的 VMA

        // 阶段 4: Slab 分配器初始化
        crate::info!("[MM] Phase 4: Initializing Slab allocator...");
        self.slab_allocator.init();

        // 阶段 5: DMA 分配器初始化
        crate::info!("[MM] Phase 5: Initializing DMA allocator...");
        // DmaAllocator is implicitly initialized via new().

        // 阶段 6: 启动后台服务和自检
        crate::info!("[MM] Phase 6: Starting background services and self-check...");
        // TODO: 启动内存回收线程 (kswapd)

        crate::info!("Memory Manager Ready.");
    }


    /// 添加新的内存映射区域
    ///
    /// # 参数
    /// * `entry` - 要添加的内存映射区域
    ///
    /// # 返回值
    /// 成功返回Ok(()), 如果与现有区域冲突则返回Err(())
    pub fn add_entry(&mut self, entry: MemoryMapEntry) -> Result<(), ()> {
        // 检查是否与现有区域冲突
        for existing in &self.entries {
            if entry.overlaps(existing) {
                return Err(());
            }
        }

        // 检查地址范围是否有效
        if entry.range.start >= entry.range.end {
            return Err(());
        }

        // 添加到列表
        self.entries.push(entry).map_err(|_| ())
    }

    /// 删除指定范围的内存映射区域
    ///
    /// # 参数
    /// * `range` - 要删除的地址范围
    ///
    /// # 返回值
    /// 返回被删除的区域数量
    pub fn remove_entries_in_range(&mut self, range: Range<usize>) -> usize {
        // Validate range
        if range.start >= range.end {
            return 0;
        }

        let mut removed_count = 0;
        self.entries.retain(|entry| {
            let should_remove = entry.range.start < range.end && range.start < entry.range.end;
            if should_remove {
                removed_count += 1;
            }
            !should_remove
        });
        removed_count
    }

    /// 更新指定范围内的内存映射区域的标志，处理部分重叠情况
    ///
    /// # 参数
    /// * `range` - 要更新的地址范围
    /// * `new_flags` - 新的标志
    ///
    /// # 返回值
    /// 成功返回Ok(()), 失败返回Err(())
    pub fn update_flags_with_partial_overlap(&mut self, range: Range<usize>, new_flags: MemoryMapFlags) -> Result<(), ()> {
        // Validate range
        if range.start >= range.end {
            return Ok(());
        }
        
        // Create a vector to hold new entries
        let mut new_entries = heapless::Vec::<MemoryMapEntry, 64>::new();
        let mut entries_to_remove = heapless::Vec::<Range<usize>, 64>::new();
        
        // Process each entry that overlaps with the range
        for entry in &self.entries {
            if entry.range.start < range.end && range.start < entry.range.end {
                // This entry overlaps with our range
                
                // Add the part before our range if it exists
                if entry.range.start < range.start {
                    let mut before_entry = entry.clone();
                    before_entry.range.end = range.start;
                    new_entries.push(before_entry).map_err(|_| ())?;
                }
                
                // Add the part after our range if it exists
                if entry.range.end > range.end {
                    let mut after_entry = entry.clone();
                    after_entry.range.start = range.end;
                    new_entries.push(after_entry).map_err(|_| ())?;
                }
                
                // Add the overlapping part with new flags
                let mut overlapping_entry = entry.clone();
                overlapping_entry.range.start = core::cmp::max(entry.range.start, range.start);
                overlapping_entry.range.end = core::cmp::min(entry.range.end, range.end);
                overlapping_entry.flags = new_flags;
                new_entries.push(overlapping_entry).map_err(|_| ())?;
                
                // Mark this entry for removal
                entries_to_remove.push(entry.range.clone()).map_err(|_| ())?;
            }
        }
        
        // Remove old entries
        for range_to_remove in &entries_to_remove {
            self.entries.retain(|entry| entry.range != *range_to_remove);
        }
        
        // Add new entries
        for entry in new_entries {
            self.add_entry(entry)?;
        }
        
        Ok(())
    }
    
    /// 查找与指定范围重叠的所有内存映射区域
    ///
    /// # 参数
    /// * `range` - 要查找的地址范围
    ///
    /// # 返回值
    /// 返回所有重叠的区域
    pub fn find_overlapping_entries(&self, range: Range<usize>) -> heapless::Vec<&MemoryMapEntry, 16> {
        let mut result = heapless::Vec::new();
        
        // Validate range
        if range.start >= range.end {
            return result;
        }
        
        for entry in &self.entries {
            if entry.range.start < range.end && range.start < entry.range.end {
                // 忽略错误，因为我们在编译时知道容量足够
                let _ = result.push(entry);
            }
        }
        result
    }
    
    /// 查找包含指定地址的内存映射区域
    ///
    /// # 参数
    /// * `addr` - 要查找的地址
    ///
    /// # 返回值
    /// 如果找到返回对应的区域，否则返回None
    pub fn find_entry(&self, addr: usize) -> Option<&MemoryMapEntry> {
        self.entries.iter().find(|entry: &&MemoryMapEntry| entry.contains(addr))
    }
    
    /// 获取所有内存映射区域
    pub fn entries(&self) -> &[MemoryMapEntry] {
        &self.entries
    }
    
    /// 清空所有内存映射区域
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 分配 DMA 缓冲区
    ///
    /// # 参数
    /// * `size`: 缓冲区大小（字节）
    /// * `alignment`: 对齐要求（字节）
    ///
    /// # 返回值
    /// * `Ok((物理地址, 能力))`: 分配成功
    /// * `Err(DriverError)`: 分配失败
    pub fn allocate_dma_buffer(
        &mut self,
        size: usize,
        alignment: usize
    ) -> Result<(u64, crate::security::capability::Capability), crate::drivers::ipc_protocol::DriverError> {
        self.dma_allocator.allocate_dma_buffer(size, alignment)
    }

    /// 释放 DMA 缓冲区
    ///
    /// # 参数
    /// * `phys_addr`: 要释放的缓冲区的物理地址
    ///
    /// # 返回值
    /// * `Ok(())`: 释放成功
    /// * `Err(DriverError)`: 释放失败
    pub fn deallocate_dma_buffer(
        &mut self,
        phys_addr: u64
    ) -> Result<(), crate::drivers::ipc_protocol::DriverError> {
        self.dma_allocator.deallocate_dma_buffer(phys_addr)
    }
}

/// 全局内存管理器实例
///
/// # 安全性
///
/// `static mut` 是不安全的，但我们在内存初始化时只对其进行一次写操作，
/// 并且之后的所有访问都通过安全的 `get_memory_manager()` 函数进行，因此这种用法是可控的。
#[used]
static mut MEMORY_MANAGER: Option<Mutex<MemoryManager>> = None;

/// 初始化全局内存管理器实例
pub fn init_manager() {
    crate::info!("memory: initializing global memory manager");
    let manager = Mutex::new(MemoryManager::new());
    unsafe {
        MEMORY_MANAGER = Some(manager);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
    crate::info!("memory: global memory manager initialized");
}

/// 获取对全局内存管理器实例的安全引用
pub fn get_memory_manager() -> &'static Mutex<MemoryManager> {
    unsafe {
        MEMORY_MANAGER.as_ref().expect("Memory manager has not been initialized")
    }
}

/// 初始化内存管理子系统
///
/// 这个函数初始化所有内存管理组件，正确的顺序为：
/// 1. 物理内存分配器（伙伴分配器）
/// 2. 虚拟内存管理
/// 3. 用于小对象分配的 slab 分配器
/// 4. 用户空间的内存映射管理器
///
/// 注意：在当前架构中，内存初始化是通过 MemoryManager 的 init 方法执行的。
/// 这个函数获取全局内存管理器实例并初始化它。
pub fn init() {
    crate::info!("memory: initializing subsystem via global memory manager");

    // 确保全局管理器已初始化
    unsafe {
        if MEMORY_MANAGER.is_none() {
            init_manager();
        }
    }

    let mut manager = get_memory_manager().lock();
    manager.init();

    crate::info!("memory: initialization complete");
}

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
    dma_allocator: DmaAllocator,
    /// 伙伴分配器
    buddy_allocator: BuddyAllocator,
    /// slab 分配器
    slab_allocator: SlabAllocator,
}

impl MemoryManager {
    /// 创建新的内存映射管理器
    pub const fn new() -> Self {
        Self {
            entries: heapless::Vec::new(),
            dma_allocator: DmaAllocator::new(),
            buddy_allocator: BuddyAllocator::new(),
            slab_allocator: SlabAllocator::new(),
        }
    }

    /// 初始化内存管理器
    pub fn init(&mut self) {
        let boot_info = crate::arch::boot::get_boot_info();

        crate::info!("memory: initializing subsystem");

        // Initialize physical memory allocator
        crate::info!("memory: initializing physical memory allocator");
        physical::init(boot_info);

        // Print physical memory statistics
        let s = physical::stats();
        crate::info!(
            "memory: phys stats: free_pages={} alloc_calls={} free_calls={} coalesce={} frag={:.3}",
            s.total_free_pages,
            s.alloc_calls,
            s.free_calls,
            s.coalesce_events,
            s.fragmentation_index,
        );

        // Verify physical allocator invariants
        let inv = physical::check_invariants();
        crate::info!("memory: phys invariants {}", if inv { "ok" } else { "bad" });

        // Initialize virtual memory management
        crate::info!("memory: initializing virtual memory");
        virt::init();

        // Initialize slab allocator for small objects
        crate::info!("memory: initializing slab allocator");
        SLAB_ALLOCATOR.init();

        // Initialize buddy allocator for kernel heap
        crate::info!("memory: initializing buddy allocator");
        let heap_start = 0x40000000; // TODO: Get from device tree
        let heap_size = 0x10000000;  // TODO: Get from device tree
        unsafe {
            BUDDY_ALLOCATOR.init(heap_start, heap_size);
        }

        crate::info!("memory: memory manager initialization complete");
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
}

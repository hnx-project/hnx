//! 内存映射区域管理器
//!
//! 该模块负责管理虚拟内存区域(VMA)，支持动态添加、删除和查询内存映射区域，
//! 并支持不同类型的内存映射（如RAM、ROM、设备内存等）。

use core::ops::Range;

use shared::sync::mutex::Mutex;
use super::allocator::dma::DmaAllocator;
use super::allocator::buddy::BuddyAllocator;
use super::allocator::slab::SlabAllocator;

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
        super::physical::init(boot_info);

        // Print physical memory statistics
        let s = super::physical::stats();
        crate::info!(
            "memory: phys stats: free_pages={} alloc_calls={} free_calls={} coalesce={} frag={:.3}",
            s.total_free_pages,
            s.alloc_calls,
            s.free_calls,
            s.coalesce_events,
            s.fragmentation_index,
        );

        // Verify physical allocator invariants
        let inv = super::physical::check_invariants();
        crate::info!("memory: phys invariants {}", if inv { "ok" } else { "bad" });

        // Initialize virtual memory management
        crate::info!("memory: initializing virtual memory");
        super::virt::init();

        // Initialize slab allocator for small objects
        crate::info!("memory: initializing slab allocator");
        super::allocator::slab::SLAB_ALLOCATOR.init();

        // Initialize buddy allocator for kernel heap
        crate::info!("memory: initializing buddy allocator");
        let heap_start = 0x40000000; // TODO: Get from device tree
        let heap_size = 0x10000000;  // TODO: Get from device tree
        unsafe {
            super::allocator::buddy::ALLOCATOR.init(heap_start, heap_size);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_map_entry_creation() {
        let entry = MemoryMapEntry::new(
            0x1000,
            0x1000,
            MemoryMapType::Ram,
            MemoryMapFlags::ram_default(),
        );
        
        assert_eq!(entry.range.start, 0x1000);
        assert_eq!(entry.range.end, 0x2000);
        assert_eq!(entry.size(), 0x1000);
        assert_eq!(entry.map_type, MemoryMapType::Ram);
        assert_eq!(entry.flags, MemoryMapFlags::ram_default());
    }
    
    #[test]
    fn test_memory_map_entry_overlap() {
        let entry1 = MemoryMapEntry::new(
            0x1000,
            0x1000,
            MemoryMapType::Ram,
            MemoryMapFlags::ram_default(),
        );
        
        let entry2 = MemoryMapEntry::new(
            0x1800,
            0x1000,
            MemoryMapType::Rom,
            MemoryMapFlags::rom_default(),
        );
        
        let entry3 = MemoryMapEntry::new(
            0x3000,
            0x1000,
            MemoryMapType::Device,
            MemoryMapFlags::device_default(),
        );
        
        assert!(entry1.overlaps(&entry2));
        assert!(!entry1.overlaps(&entry3));
        assert!(!entry2.overlaps(&entry3));
    }
    
    #[test]
    fn test_memory_map_manager_add_entry() {
        let mut manager = MemoryManager::new();

        let entry1 = MemoryMapEntry::new(
            0x1000,
            0x1000,
            MemoryMapType::Ram,
            MemoryMapFlags::ram_default(),
        );

        let entry2 = MemoryMapEntry::new(
            0x3000,
            0x1000,
            MemoryMapType::Rom,
            MemoryMapFlags::rom_default(),
        );

        assert!(manager.add_entry(entry1.clone()).is_ok());
        assert!(manager.add_entry(entry2.clone()).is_ok());
        assert_eq!(manager.entries().len(), 2);
    }
    
    #[test]
    fn test_memory_map_manager_add_conflicting_entry() {
        let mut manager = MemoryManager::new();

        let entry1 = MemoryMapEntry::new(
            0x1000,
            0x1000,
            MemoryMapType::Ram,
            MemoryMapFlags::ram_default(),
        );

        let entry2 = MemoryMapEntry::new(
            0x1800,
            0x1000,
            MemoryMapType::Rom,
            MemoryMapFlags::rom_default(),
        );

        assert!(manager.add_entry(entry1.clone()).is_ok());
        assert!(manager.add_entry(entry2.clone()).is_err());
        assert_eq!(manager.entries().len(), 1);
    }
    
    #[test]
    fn test_memory_map_manager_find_entry() {
        let mut manager = MemoryManager::new();

        let entry = MemoryMapEntry::new(
            0x1000,
            0x1000,
            MemoryMapType::Ram,
            MemoryMapFlags::ram_default(),
        );

        assert!(manager.add_entry(entry.clone()).is_ok());

        assert!(manager.find_entry(0x1500).is_some());
        assert!(manager.find_entry(0x500).is_none());
        assert!(manager.find_entry(0x2500).is_none());
    }
    
    #[test]
    fn test_memory_map_manager_remove_entries() {
        let mut manager = MemoryManager::new();

        let entry1 = MemoryMapEntry::new(
            0x1000,
            0x1000,
            MemoryMapType::Ram,
            MemoryMapFlags::ram_default(),
        );

        let entry2 = MemoryMapEntry::new(
            0x3000,
            0x1000,
            MemoryMapType::Rom,
            MemoryMapFlags::rom_default(),
        );

        assert!(manager.add_entry(entry1.clone()).is_ok());
        assert!(manager.add_entry(entry2.clone()).is_ok());

        let removed = manager.remove_entries_in_range(0x500..0x2000);
        assert_eq!(removed, 1);
        assert_eq!(manager.entries().len(), 1);
    }
}
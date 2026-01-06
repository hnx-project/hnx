//! DMA Allocator
//!
//! This module implements DMA-capable memory allocation for device drivers.

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::drivers::ipc_protocol::DriverError;
use crate::memory::physical::alloc_pages;
use crate::security::capability::Capability;
use shared::sync::mutex::Mutex;

/// Information about a DMA region
#[derive(Debug, Clone)]
pub struct DmaRegion {
    pub physical_address: u64,
    pub virtual_address: usize,
    pub size: usize,
    pub allocated: bool,
}

/// DMA allocator
pub struct DmaAllocator {
    allocated_regions: BTreeMap<u64, DmaRegion>,
    free_regions: Vec<DmaRegion>,
}

impl DmaAllocator {
    /// Create a new DMA allocator
    pub const fn new() -> Self {
        Self {
            allocated_regions: BTreeMap::new(),
            free_regions: Vec::new(),
        }
    }

    /// Initialize the DMA allocator
    /// This method prepares the allocator for use, setting up any necessary structures.
    pub fn init(&mut self) {
        // Currently nothing to initialize, but kept for consistency
        // with other manager singletons.
    }

    /// Allocate a DMA buffer
    pub fn allocate_dma_buffer(&mut self, size: usize, alignment: usize) -> Result<(u64, Capability), DriverError> {
        // For now, we'll just allocate a new region
        // In a real implementation, we would look for free regions first
        
        let page_size = 4096;
        let pages_needed = (size + page_size - 1) / page_size;
        
        // Allocate physical pages
        let physical_address = match alloc_pages(pages_needed) {
            Some(addr) => addr as u64,
            None => return Err(DriverError::OutOfMemory),
        };
        
        // For simplicity, we'll use the same value for virtual address
        // In a real implementation, we would map this to kernel virtual address space
        let virtual_address = physical_address as usize;
        
        let region = DmaRegion {
            physical_address,
            virtual_address,
            size: pages_needed * page_size,
            allocated: true,
        };
        
        // Create a DMA buffer capability
        let capability = Capability::new_dma_buffer(
            region.physical_address,
            region.virtual_address,
            region.size
        );
        
        self.allocated_regions.insert(region.physical_address, region);
        
        Ok((physical_address, capability))
    }

    /// Deallocate a DMA buffer
    pub fn deallocate_dma_buffer(&mut self, phys_addr: u64) -> Result<(), DriverError> {
        if let Some(mut region) = self.allocated_regions.remove(&phys_addr) {
            region.allocated = false;
            self.free_regions.push(region);
            Ok(())
        } else {
            Err(DriverError::InvalidArgument)
        }
    }
}

/// 全局DMA分配器单例实例
///
/// # 安全性
///
/// `static mut` 是不安全的，但我们在初始化时只对其进行一次写操作，
/// 并且之后的所有访问都通过安全的 `get_dma_allocator()` 函数进行，因此这种用法是可控的。
#[used]
static mut DMA_ALLOCATOR: Option<Mutex<DmaAllocator>> = None;

/// 初始化全局DMA分配器单例实例
pub fn init_dma_allocator() {
    crate::info!("dma: initializing global DMA allocator singleton");
    let allocator = Mutex::new(DmaAllocator::new());
    unsafe {
        DMA_ALLOCATOR = Some(allocator);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
    crate::info!("dma: global DMA allocator singleton initialized");
}

/// 获取对全局DMA分配器单例实例的安全引用
pub fn get_dma_allocator() -> &'static Mutex<DmaAllocator> {
    unsafe {
        DMA_ALLOCATOR.as_ref().expect("DMA allocator has not been initialized")
    }
}

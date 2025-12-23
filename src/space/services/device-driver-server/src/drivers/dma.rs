//! DMA Allocator for user-space device driver server
//!
//! This module implements DMA-capable memory allocation for device drivers
//! that communicate with the kernel through IPC.

extern crate alloc;

use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// DMA allocator instance
pub static DMA_ALLOCATOR: Mutex<DmaAllocator> = Mutex::new(DmaAllocator::new());

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

/// DMA error types
#[derive(Debug, Clone)]
pub enum DmaError {
    OutOfMemory,
    InvalidArgument,
    IoError,
}

/// Information about a DMA capability
#[derive(Debug, Clone)]
pub struct DmaCapability {
    pub id: u64,
    pub physical_address: u64,
    pub virtual_address: usize,
    pub size: usize,
}

impl DmaAllocator {
    /// Create a new DMA allocator
    pub const fn new() -> Self {
        Self {
            allocated_regions: BTreeMap::new(),
            free_regions: Vec::new(),
        }
    }

    /// Allocate a DMA buffer
    pub fn allocate_dma_buffer(&mut self, size: usize, alignment: usize) -> Result<(u64, DmaCapability), DmaError> {
        // For now, we'll just allocate a new region
        // In a real implementation, we would look for free regions first
        
        // Calculate number of pages needed
        let page_size = 4096;
        let pages_needed = (size + page_size - 1) / page_size;
        
        // In a user-space implementation, we would request DMA memory from the kernel
        // through IPC. For now, we'll simulate allocation with dummy values.
        let physical_address = (pages_needed * page_size) as u64;
        let virtual_address = physical_address as usize;
        
        // Create region
        let region = DmaRegion {
            physical_address,
            virtual_address,
            size: pages_needed * page_size,
            allocated: true,
        };
        
        // Create capability
        let capability = DmaCapability {
            id: self.generate_capability_id(),
            physical_address: region.physical_address,
            virtual_address: region.virtual_address,
            size: region.size,
        };
        
        // Store the allocated region
        self.allocated_regions.insert(region.physical_address, region);
        
        Ok((physical_address, capability))
    }

    /// Deallocate a DMA buffer
    pub fn deallocate_dma_buffer(&mut self, phys_addr: u64) -> Result<(), DmaError> {
        if let Some(mut region) = self.allocated_regions.remove(&phys_addr) {
            region.allocated = false;
            self.free_regions.push(region);
            Ok(())
        } else {
            Err(DmaError::InvalidArgument)
        }
    }
    
    /// Find a free region that meets the requirements
    fn find_free_region(&mut self, size: usize, alignment: usize) -> Result<DmaRegion, DmaError> {
        // For now, we'll just allocate a new region
        // In a real implementation, we would search free_regions first
        
        let page_size = 4096;
        let pages_needed = (size + page_size - 1) / page_size;
        
        // In a user-space implementation, we would request DMA memory from the kernel
        // through IPC. For now, we'll simulate allocation with dummy values.
        let physical_address = (pages_needed * page_size) as u64;
        let virtual_address = physical_address as usize;
        
        Ok(DmaRegion {
            physical_address,
            virtual_address,
            size: pages_needed * page_size,
            allocated: false,
        })
    }
    
    /// Generate a unique capability ID
    fn generate_capability_id(&self) -> u64 {
        // In a real implementation, this would be a proper ID generator
        // For now, we'll use a simple counter
        static mut NEXT_ID: u64 = 1;
        unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        }
    }
}

impl DmaCapability {
    /// Create a new DMA capability
    pub fn new(physical_address: u64, virtual_address: usize, size: usize) -> Self {
        static mut NEXT_ID: u64 = 1;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        
        Self {
            id,
            physical_address,
            virtual_address,
            size,
        }
    }
    
    /// Get the capability ID
    pub fn id(&self) -> u64 {
        self.id
    }
}
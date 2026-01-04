//! DMA Allocator
//!
//! This module implements DMA-capable memory allocation for device drivers.

#![no_std]

extern crate alloc;

use spin::Mutex;
use alloc::collections::BTreeMap;
use crate::memory::physical::alloc_pages;
use crate::drivers::ipc_protocol::DriverError;

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
    free_regions: alloc::vec::Vec<DmaRegion>,
}

impl DmaAllocator {
    /// Create a new DMA allocator
    pub const fn new() -> Self {
        Self {
            allocated_regions: BTreeMap::new(),
            free_regions: alloc::vec::Vec::new(),
        }
    }

    /// Allocate a DMA buffer
    pub fn allocate_dma_buffer(&mut self, size: usize, alignment: usize) -> Result<(u64, Capability), DriverError> {
        // For now, we'll just allocate a new region
        // In a real implementation, we would look for free regions first
        
        // Calculate number of pages needed
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
        
        // Create region
        let region = DmaRegion {
            physical_address,
            virtual_address,
            size: pages_needed * page_size,
            allocated: true,
        };
        
        // Create capability
        let capability = Capability::new_dma_buffer(
            region.physical_address,
            region.virtual_address,
            region.size
        );
        
        // Store the allocated region
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
    
    /// Find a free region that meets the requirements
    fn find_free_region(&mut self, size: usize, alignment: usize) -> Result<DmaRegion, DriverError> {
        // For now, we'll just allocate a new region
        // In a real implementation, we would search free_regions first
        
        let page_size = 4096;
        let pages_needed = (size + page_size - 1) / page_size;
        
        // Allocate physical pages
        let physical_address = match alloc_pages(pages_needed) {
            Some(addr) => addr as u64,
            None => return Err(DriverError::OutOfMemory),
        };
        
        // For simplicity, we'll use the same value for virtual address
        let virtual_address = physical_address as usize;
        
        Ok(DmaRegion {
            physical_address,
            virtual_address,
            size: pages_needed * page_size,
            allocated: false,
        })
    }
}

/// Capability for accessing hardware resources
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: u64,
    pub cap_type: CapabilityType,
}

/// Types of capabilities
#[derive(Debug, Clone)]
pub enum CapabilityType {
    Mmio {
        physical_address: u64,
        size: usize,
    },
    DmaBuffer {
        physical_address: u64,
        virtual_address: usize,  // Kernel virtual address
        size: usize,
    },
}

impl Capability {
    /// Create a new MMIO capability
    pub fn new_mmio(physical_address: u64, size: usize) -> Self {
        Self {
            id: generate_capability_id(),
            cap_type: CapabilityType::Mmio { physical_address, size },
        }
    }

    /// Create a new DMA buffer capability
    pub fn new_dma_buffer(physical_address: u64, virtual_address: usize, size: usize) -> Self {
        Self {
            id: generate_capability_id(),
            cap_type: CapabilityType::DmaBuffer { physical_address, virtual_address, size },
        }
    }

    /// Create an invalid capability
    pub fn invalid() -> Self {
        Self {
            id: 0,
            cap_type: CapabilityType::Mmio { physical_address: 0, size: 0 },
        }
    }
    
    /// Get the capability ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Global capability ID generator
static NEXT_CAPABILITY_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);

/// Generate a unique capability ID
fn generate_capability_id() -> u64 {
    NEXT_CAPABILITY_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}
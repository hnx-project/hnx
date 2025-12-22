//! Zero-copy packet passing for network operations
//!
//! This module implements zero-copy packet buffers for efficient packet
//! passing between kernel and user-space.

extern crate alloc;

use crate::network::NetworkError;
use core::sync::atomic::{AtomicU32, Ordering};

/// Packet buffer for zero-copy operations
pub struct PacketBuffer {
    pub physical_address: u64,
    pub virtual_address: usize,
    pub size: usize,
    pub capacity: usize,
    pub ref_count: AtomicU32,
}

impl PacketBuffer {
    /// Create a new packet buffer of specified size
    pub fn new(size: usize) -> Result<Self, NetworkError> {
        // Allocate DMA-capable memory
        let (phys_addr, virt_addr) = dma_allocate(size)?;
        
        Ok(Self {
            physical_address: phys_addr,
            virtual_address: virt_addr,
            size: 0,
            capacity: size,
            ref_count: AtomicU32::new(1),
        })
    }
    
    /// Clone the packet buffer (increases reference count)
    pub fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            physical_address: self.physical_address,
            virtual_address: self.virtual_address,
            size: self.size,
            capacity: self.capacity,
            ref_count: AtomicU32::new(self.ref_count.load(Ordering::SeqCst)),
        }
    }
    
    /// Get immutable slice of the buffer data
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.virtual_address as *const u8, self.size)
        }
    }
    
    /// Get mutable slice of the buffer data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.virtual_address as *mut u8, self.capacity)
        }
    }
    
    /// Set the size of valid data in the buffer
    pub fn set_size(&mut self, size: usize) {
        self.size = core::cmp::min(size, self.capacity);
    }
}

impl Drop for PacketBuffer {
    fn drop(&mut self) {
        if self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            // Last reference, deallocate
            dma_deallocate(self.physical_address, self.virtual_address)
                .unwrap_or_else(|e| crate::error!("Failed to deallocate packet buffer: {:?}", e));
        }
    }
}

/// Stub function for DMA allocation
fn dma_allocate(size: usize) -> Result<(u64, usize), NetworkError> {
    // In a real implementation, this would allocate DMA-capable memory
    // For now, we'll just simulate it with regular allocation
    let layout = core::alloc::Layout::from_size_align(size, 8)
        .map_err(|_| NetworkError::OutOfMemory)?;
    
    unsafe {
        let ptr = alloc::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            Err(NetworkError::OutOfMemory)
        } else {
            let virt_addr = ptr as usize;
            let phys_addr = virt_addr as u64; // Simplified for simulation
            Ok((phys_addr, virt_addr))
        }
    }
}

/// Stub function for DMA deallocation
fn dma_deallocate(physical_address: u64, virtual_address: usize) -> Result<(), NetworkError> {
    // In a real implementation, this would deallocate DMA-capable memory
    // For now, we'll just simulate it
    let size = 4096; // Assume standard page size for simulation
    let layout = core::alloc::Layout::from_size_align(size, 8)
        .map_err(|_| NetworkError::InvalidParameter)?;
    
    unsafe {
        let ptr = virtual_address as *mut u8;
        alloc::alloc::dealloc(ptr, layout);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_buffer_creation() {
        let result = PacketBuffer::new(1500);
        assert!(result.is_ok());
        
        let buffer = result.unwrap();
        assert_eq!(buffer.size, 0);
        assert_eq!(buffer.capacity, 1500);
        assert_eq!(buffer.ref_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_packet_buffer_clone() {
        let buffer = PacketBuffer::new(1024).unwrap();
        let cloned = buffer.clone();
        
        assert_eq!(buffer.physical_address, cloned.physical_address);
        assert_eq!(buffer.virtual_address, cloned.virtual_address);
        assert_eq!(buffer.ref_count.load(Ordering::SeqCst), 2);
        assert_eq!(cloned.ref_count.load(Ordering::SeqCst), 2);
    }
}
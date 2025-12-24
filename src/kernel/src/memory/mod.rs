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
pub mod virtual_;
pub mod protection;
pub mod buddy_allocator;
pub mod dma;
pub mod slab_allocator;
pub mod mmap_manager;

use crate::arch::common::boot::BootInfo;

// Re-export commonly used types and functions
pub use physical::{PhysAddr, alloc_pages, free_pages, stats as physical_stats, check_invariants};
pub use virtual_::{VirtAddr, KERNEL_BASE, map, unmap, create_user_l1, map_in_pt};
pub use protection::{
    add_guard_page, 
    initialize_stack_canary, 
    verify_stack_canary,
    randomize_address_space,
    validate_memory_access,
};
pub use buddy_allocator::ALLOCATOR as BUDDY_ALLOCATOR;
pub use slab_allocator::SLAB_ALLOCATOR;

/// Initialize the memory management subsystem
///
/// This function initializes all memory management components in the correct order:
/// 1. Physical memory allocator (buddy allocator)
/// 2. Virtual memory management
/// 3. Slab allocator for small object allocation
/// 4. Memory mapping manager for user space
pub fn init(boot_info: BootInfo) {
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
    virtual_::init();
    
    // Initialize slab allocator for small objects
    crate::info!("memory: initializing slab allocator");
    slab_allocator::SLAB_ALLOCATOR.init();
    
    // Initialize memory mapping manager
    crate::info!("memory: initializing memory map manager");
    mmap_manager::init();
    
    crate::info!("memory: initialization complete");
}

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
pub mod manager;

// Import types needed for memory management
use core::ops::Range;
use shared::sync::mutex::Mutex;

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

// Re-export memory manager types
pub use manager::{MemoryMapType, MemoryMapFlags, MemoryMapEntry, MemoryManager, init_memory_manager, get_memory_manager};



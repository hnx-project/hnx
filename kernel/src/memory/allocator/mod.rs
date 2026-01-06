//! Memory allocators for the HNX kernel
//!
//! This module provides various memory allocators for different use cases:
//! - **Buddy allocator**: For page-sized allocations
//! - **Slab allocator**: For small object allocations
//! - **DMA allocator**: For device DMA buffers

pub mod buddy;
pub mod slab;
pub mod dma;

// Re-export commonly used types and functions
pub use buddy::{BuddyAllocator, ALLOCATOR as BUDDY_ALLOCATOR, get_buddy_allocator};
pub use slab::{SlabAllocator, SLAB_ALLOCATOR, get_slab_allocator};
pub use dma::{DmaAllocator, DmaRegion, init_dma_allocator, get_dma_allocator};
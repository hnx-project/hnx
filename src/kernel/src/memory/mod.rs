#![allow(dead_code)]

pub mod physical;
pub mod virtual_;
pub mod protection;
pub mod buddy_allocator;
pub mod dma;
pub mod slab_allocator;
pub mod mmap_manager;

use crate::arch::common::boot::BootInfo;

pub fn init(boot_info: BootInfo) {
    crate::info!("init");
    crate::info!("initializing physical memory allocator");
    physical::init(boot_info);
    let s = physical::stats();
    crate::info!(
        "phys stats: free_pages={} alloc_calls={} free_calls={} coalesce={} frag={:.3}",
        s.total_free_pages,
        s.alloc_calls,
        s.free_calls,
        s.coalesce_events,
        s.fragmentation_index,
    );
    let inv = physical::check_invariants();
    crate::info!("phys invariants {}", if inv { "ok" } else { "bad" });
    crate::info!("initializing virtual memory");
    virtual_::init();
    crate::info!("initializing slab allocator");
    slab_allocator::SLAB_ALLOCATOR.init();
    crate::info!("initializing memory map manager");
    mmap_manager::init();
    crate::info!("memory initialization complete");
}

#[cfg(test)]
mod buddy_allocator_test;

#[cfg(test)]
mod slab_allocator_test;

#[cfg(test)]
mod mmap_manager_test;
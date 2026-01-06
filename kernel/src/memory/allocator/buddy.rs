//! Advanced memory allocator for the HNX kernel
//!
//! This module implements a buddy system memory allocator that supports
//! allocation and deallocation of memory blocks.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use crate::arch::memory;

/// Maximum order of the buddy system (2^MAX_ORDER bytes)
const MAX_ORDER: usize = 32;

/// Minimum block size (typically a page size)
const MIN_BLOCK_SIZE: usize = 4096;

/// A buddy system memory allocator
pub struct BuddyAllocator {
    /// Start address of the heap
    heap_start: AtomicUsize,
    /// End address of the heap
    heap_end: AtomicUsize,
    /// Free lists for each order
    free_lists: [UnsafeCell<*mut FreeBlock>; MAX_ORDER],
    /// Lock for synchronization
    locked: AtomicBool,
}

/// A free block in the buddy system
#[repr(C)]
struct FreeBlock {
    /// Pointer to the next free block
    next: *mut FreeBlock,
    /// Order of this block (size = MIN_BLOCK_SIZE * 2^order)
    order: usize,
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    pub const fn new() -> Self {
        // Initialize free lists
        const NULL_BLOCK: UnsafeCell<*mut FreeBlock> = UnsafeCell::new(ptr::null_mut());
        Self {
            heap_start: AtomicUsize::new(0),
            heap_end: AtomicUsize::new(0),
            free_lists: [NULL_BLOCK; MAX_ORDER],
            locked: AtomicBool::new(false),
        }
    }
    
    /// Initialize the allocator with a heap
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        crate::info!("buddy allocator init: heap_start=0x{:016X}, heap_size={}", heap_start, heap_size);
        self.heap_start.store(heap_start, Ordering::Relaxed);
        self.heap_end.store(heap_start + heap_size, Ordering::Relaxed);
        
        // Initialize free lists to null
        for i in 0..MAX_ORDER {
            *self.free_lists[i].get() = ptr::null_mut();
        }
        
        crate::info!("buddy allocator: initialized free lists");
        // Add the entire heap as one big free block
        self.add_free_block(heap_start, heap_size);
        crate::info!("buddy allocator: added free block");
    }
    
    /// Add a free block to the appropriate free list
    unsafe fn add_free_block(&self, addr: usize, size: usize) {
        crate::info!("buddy allocator: add_free_block addr=0x{:016X}, size={}", addr, size);
        // Find the largest power of 2 that fits in the size
        let mut order = 0;
        let mut block_size = MIN_BLOCK_SIZE;
        while block_size * 2 <= size && order < MAX_ORDER - 1 {
            block_size *= 2;
            order += 1;
        }
        
        crate::info!("buddy allocator: calculated order={}, block_size={}", order, block_size);
        
        // If the block is larger than what we can handle, split it
        if block_size > size {
            block_size /= 2;
            order -= 1;
        }
        
        crate::info!("buddy allocator: adjusted order={}, block_size={}", order, block_size);
        
        // Create a free block
        let block = addr as *mut FreeBlock;
        (*block).next = *self.free_lists[order].get();
        (*block).order = order;
        *self.free_lists[order].get() = block;
        
        crate::info!("buddy allocator: created free block at addr=0x{:016X}, order={}", addr, order);
        
        // If there's remaining space, recursively add it
        let remaining = size - block_size;
        if remaining >= MIN_BLOCK_SIZE {
            crate::info!("buddy allocator: adding remaining space addr=0x{:016X}, size={}", addr + block_size, remaining);
            self.add_free_block(addr + block_size, remaining);
        }
        crate::info!("buddy allocator: add_free_block complete for addr=0x{:016X}", addr);
    }
    
    /// Find a free block of the given order
    unsafe fn find_free_block(&self, order: usize) -> Option<*mut FreeBlock> {
        // First try to find a block of exactly the right size
        let block = *self.free_lists[order].get();
        if !block.is_null() {
            *self.free_lists[order].get() = (*block).next;
            return Some(block);
        }
        
        // If no block of exact size, try to split a larger block
        for i in (order + 1)..MAX_ORDER {
            let larger_block = *self.free_lists[i].get();
            if !larger_block.is_null() {
                // Remove the larger block from its list
                *self.free_lists[i].get() = (*larger_block).next;
                
                // Split the block down to the desired size
                let mut current_order = i;
                let mut current_addr = larger_block as usize;
                while current_order > order {
                    current_order -= 1;
                    let buddy_addr = current_addr + (MIN_BLOCK_SIZE << current_order);
                    
                    // Add the buddy to the free list
                    let buddy_block = buddy_addr as *mut FreeBlock;
                    (*buddy_block).next = *self.free_lists[current_order].get();
                    (*buddy_block).order = current_order;
                    *self.free_lists[current_order].get() = buddy_block;
                }
                
                return Some(current_addr as *mut FreeBlock);
            }
        }
        
        None
    }
    
    /// Return a block to the free lists
    unsafe fn free_block(&self, block: *mut FreeBlock, order: usize) {
        let mut current_block = block;
        let mut current_order = order;
        let block_addr = block as usize;
        
        // Try to merge with buddies
        loop {
            // Prevent overflow when shifting or accessing array out of bounds
            if current_order >= MAX_ORDER - 1 {
                // Add to free list and exit loop
                (*current_block).next = *self.free_lists[MAX_ORDER - 1].get();
                (*current_block).order = MAX_ORDER - 1;
                *self.free_lists[MAX_ORDER - 1].get() = current_block;
                break;
            }
            
            let block_size = MIN_BLOCK_SIZE << current_order;
            let buddy_addr = block_addr ^ block_size;
            
            // Check if the buddy is free
            let mut found_buddy = false;
            let mut prev_ptr: *mut *mut FreeBlock = self.free_lists[current_order].get();
            let mut current_ptr = *prev_ptr;
            
            while !current_ptr.is_null() {
                if current_ptr as usize == buddy_addr {
                    // Found the buddy, remove it from the free list
                    *prev_ptr = (*current_ptr).next;
                    found_buddy = true;
                    break;
                }
                prev_ptr = &mut (*current_ptr).next;
                current_ptr = *prev_ptr;
            }
            
            if found_buddy {
                // Merge with the buddy
                let merged_addr = block_addr & !(block_size * 2 - 1);
                current_block = merged_addr as *mut FreeBlock;
                current_order += 1;
            } else {
                // No buddy to merge with, add to free list
                (*current_block).next = *self.free_lists[current_order].get();
                (*current_block).order = current_order;
                *self.free_lists[current_order].get() = current_block;
                break;
            }
        }
    }
    
    /// Acquire the lock
    fn lock(&self) -> bool {
        self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
    
    /// Release the lock
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

unsafe impl GlobalAlloc for BuddyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Acquire lock
        if !self.lock() {
            // Spin until we can acquire the lock
            while !self.lock() {
                memory::yield_cpu();
            }
        }
        
        // Calculate the required block size
        let mut size = layout.size().max(layout.align());
        size = size.next_power_of_two().max(MIN_BLOCK_SIZE);
        
        // Find the appropriate order
        let mut order = 0;
        let mut block_size = MIN_BLOCK_SIZE;
        while block_size < size && order < MAX_ORDER - 1 {
            block_size *= 2;
            order += 1;
        }
        
        // Find a free block
        if let Some(block) = self.find_free_block(order) {
            self.unlock();
            block as *mut u8
        } else {
            self.unlock();
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // Acquire lock
        if !self.lock() {
            // Spin until we can acquire the lock
            while !self.lock() {
                memory::yield_cpu();
            }
        }
        
        // Get the block and its order
        let block = ptr as *mut FreeBlock;
        let order = (*block).order;
        
        // Return the block to the free lists
        self.free_block(block, order);
        
        // Release lock
        self.unlock();
    }
}

// SAFETY: Our allocator is safe to share between threads because we use atomic operations
// and locks to synchronize access
unsafe impl Sync for BuddyAllocator {}

#[global_allocator]
pub static ALLOCATOR: BuddyAllocator = BuddyAllocator::new();

/// 获取全局伙伴分配器单例实例的引用
pub fn get_buddy_allocator() -> &'static BuddyAllocator {
    &ALLOCATOR
}
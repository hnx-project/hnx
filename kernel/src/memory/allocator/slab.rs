//! Slab allocator for kernel objects
//!
//! This module implements a slab allocator that efficiently manages fixed-size
//! allocations for kernel objects. The slab allocator reduces internal fragmentation
//! compared to the buddy allocator for small objects and provides faster allocation
//! and deallocation times.

use core::alloc::Layout;
use core::ptr;
use core::mem;
use core::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use shared::sync::mutex::Mutex;
use crate::memory::physical;

/// Statistics for cache monitoring (only in debug builds)
#[cfg(debug_assertions)]
#[derive(Debug)]
struct CacheStats {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    slab_creations: AtomicUsize,
}

/// Snapshot of cache statistics
#[derive(Debug)]
pub struct CacheStatsSnapshot {
    pub allocations: usize,
    pub deallocations: usize,
    pub slab_creations: usize,
}

/// Statistics for the entire slab allocator
#[derive(Debug)]
pub struct SlabAllocatorStats {
    pub cache_16: CacheStatsSnapshot,
    pub cache_32: CacheStatsSnapshot,
    pub cache_64: CacheStatsSnapshot,
    pub cache_128: CacheStatsSnapshot,
    pub cache_256: CacheStatsSnapshot,
    pub cache_512: CacheStatsSnapshot,
    pub cache_1024: CacheStatsSnapshot,
    pub cache_2048: CacheStatsSnapshot,
}

/// Maximum number of caches
const MAX_CACHES: usize = 32;

/// Size of a slab in pages
const SLAB_SIZE_PAGES: usize = 4; // 16KB slabs
const SLAB_SIZE_BYTES: usize = SLAB_SIZE_PAGES * 4096;

/// A slab contains multiple objects of the same size
#[repr(C)]
struct Slab {
    /// Pointer to the next slab in the list
    next: AtomicPtr<Slab>,
    /// Number of free objects in this slab
    free_count: AtomicUsize,
    /// Bitmap of free objects (bit set = object is free)
    free_bitmap: [AtomicUsize; (SLAB_SIZE_BYTES / mem::align_of::<usize>() + 63) / 64],
    /// Base address of the slab memory
    base_addr: usize,
    /// Size of each object in this slab
    object_size: usize,
    /// Number of objects that fit in this slab
    objects_per_slab: usize,
}

impl Slab {
    /// Create a new slab
    unsafe fn new(base_addr: usize, object_size: usize) -> *mut Slab {
        let slab_ptr = base_addr as *mut Slab;
        
        // Initialize the slab header
        ptr::write_volatile(&mut (*slab_ptr).next, AtomicPtr::new(ptr::null_mut()));
        ptr::write_volatile(&mut (*slab_ptr).free_count, AtomicUsize::new(0));
        ptr::write_volatile(&mut (*slab_ptr).base_addr, base_addr);
        ptr::write_volatile(&mut (*slab_ptr).object_size, object_size);
        
        // Calculate how many objects fit in the slab
        let header_size = mem::size_of::<Slab>();
        // Align header size to object alignment
        let aligned_header_size = (header_size + object_size - 1) & !(object_size - 1);
        let usable_size = SLAB_SIZE_BYTES - aligned_header_size;
        let objects_per_slab = usable_size / object_size;
        ptr::write_volatile(&mut (*slab_ptr).objects_per_slab, objects_per_slab);
        
        // Initialize free bitmap
        for i in 0..((SLAB_SIZE_BYTES / mem::align_of::<usize>() + 63) / 64) {
            ptr::write_volatile(&mut (*slab_ptr).free_bitmap[i], AtomicUsize::new(0));
        }
        
        // Mark all objects as free initially
        let full_words = objects_per_slab / 64;
        let remainder = objects_per_slab % 64;
        
        for i in 0..full_words {
            (*slab_ptr).free_bitmap[i].store(usize::MAX, Ordering::Relaxed);
        }
        
        if remainder > 0 {
            let mask = (1usize << remainder) - 1;
            if full_words < ((SLAB_SIZE_BYTES / mem::align_of::<usize>() + 63) / 64) {
                (*slab_ptr).free_bitmap[full_words].store(mask, Ordering::Relaxed);
            }
        }
        
        (*slab_ptr).free_count.store(objects_per_slab, Ordering::Relaxed);
        
        slab_ptr
    }
    
    /// Allocate an object from this slab
    fn allocate(&self) -> Option<*mut u8> {
        let free_count = self.free_count.load(Ordering::Relaxed);
        if free_count == 0 {
            return None;
        }
        
        // Find a free object using the bitmap
        let objects_per_slab = self.objects_per_slab;
        let bitmap_size = (objects_per_slab + 63) / 64;
        
        for i in 0..bitmap_size.min((SLAB_SIZE_BYTES / mem::align_of::<usize>() + 63) / 64) {
            let mut word = self.free_bitmap[i].load(Ordering::Relaxed);
            if word != 0 {
                // Find the first free bit in this word
                let bit_pos = word.trailing_zeros() as usize;
                let mask = 1usize << bit_pos;
                
                // Atomically clear the bit
                while let Err(old) = self.free_bitmap[i].compare_exchange_weak(
                    word, 
                    word & !mask, 
                    Ordering::Relaxed, 
                    Ordering::Relaxed
                ) {
                    word = old;
                    if word & mask == 0 {
                        // Bit was already cleared, try next word
                        break;
                    }
                }
                
                if word & mask != 0 {
                    // Successfully allocated
                    self.free_count.fetch_sub(1, Ordering::Relaxed);
                    // Calculate object address with proper alignment
                    let header_size = mem::size_of::<Slab>();
                    let aligned_header_size = (header_size + self.object_size - 1) & !(self.object_size - 1);
                    let obj_index = i * 64 + bit_pos;
                    let obj_addr = self.base_addr + aligned_header_size + obj_index * self.object_size;
                    return Some(obj_addr as *mut u8);
                }
            }
        }
        
        None
    }
    
    /// Deallocate an object back to this slab
    fn deallocate(&self, ptr: *mut u8) -> bool {
        let header_size = mem::size_of::<Slab>();
        let aligned_header_size = (header_size + self.object_size - 1) & !(self.object_size - 1);
        
        let obj_addr = ptr as usize;
        if obj_addr < self.base_addr + aligned_header_size || 
           obj_addr >= self.base_addr + SLAB_SIZE_BYTES {
            return false;
        }
        
        let obj_offset = obj_addr - (self.base_addr + aligned_header_size);
        if obj_offset % self.object_size != 0 {
            return false;
        }
        
        let obj_index = obj_offset / self.object_size;
        if obj_index >= self.objects_per_slab {
            return false;
        }
        
        let word_index = obj_index / 64;
        let bit_index = obj_index % 64;
        let mask = 1usize << bit_index;
        
        // Check bounds for bitmap access
        if word_index >= (SLAB_SIZE_BYTES / mem::align_of::<usize>() + 63) / 64 {
            return false;
        }
        
        // Atomically set the bit
        let mut word = self.free_bitmap[word_index].load(Ordering::Relaxed);
        while let Err(old) = self.free_bitmap[word_index].compare_exchange_weak(
            word, 
            word | mask, 
            Ordering::Relaxed, 
            Ordering::Relaxed
        ) {
            word = old;
            if word & mask != 0 {
                // Bit was already set, double free?
                return false;
            }
        }
        
        self.free_count.fetch_add(1, Ordering::Relaxed);
        true
    }
    
    /// Check if this slab contains the given pointer
    fn contains(&self, ptr: *mut u8) -> bool {
        let header_size = mem::size_of::<Slab>();
        let aligned_header_size = (header_size + self.object_size - 1) & !(self.object_size - 1);
        
        let addr = ptr as usize;
        addr >= self.base_addr + aligned_header_size && 
        addr < self.base_addr + SLAB_SIZE_BYTES &&
        (addr - (self.base_addr + aligned_header_size)) % self.object_size == 0
    }
}

/// A cache for objects of a specific size
struct SlabCache {
    /// Object size for this cache
    object_size: usize,
    /// List of partially full slabs
    partial_slabs: AtomicPtr<Slab>,
    /// List of empty slabs
    empty_slabs: AtomicPtr<Slab>,
    /// List of full slabs
    full_slabs: AtomicPtr<Slab>,
    /// Lock for synchronizing access to this cache
    lock: Mutex<()>,
    /// Statistics for debugging and monitoring
    #[cfg(debug_assertions)]
    stats: CacheStats,
}

impl SlabCache {
    /// Create a new slab cache
    const fn new(object_size: usize) -> Self {
        Self {
            object_size,
            partial_slabs: AtomicPtr::new(ptr::null_mut()),
            empty_slabs: AtomicPtr::new(ptr::null_mut()),
            full_slabs: AtomicPtr::new(ptr::null_mut()),
            lock: Mutex::new(()),
            #[cfg(debug_assertions)]
            stats: CacheStats {
                allocations: AtomicUsize::new(0),
                deallocations: AtomicUsize::new(0),
                slab_creations: AtomicUsize::new(0),
            },
        }
    }
    
    /// Initialize the cache with a new slab
    fn init_with_slab(&self) -> bool {
        crate::info!("initializing slab for cache size {}", self.object_size);
        
        // Acquire lock to ensure thread safety during initialization
        let _guard = self.lock.lock();
        
        // Allocate physical pages for a new slab
        if let Some(page_addr) = physical::alloc_pages(SLAB_SIZE_PAGES) {
            crate::info!("allocated pages for slab of size {} at address {:#x}", self.object_size, page_addr);
            
            // Validate that we have enough space for at least one object
            let header_size = mem::size_of::<Slab>();
            // Align header size to object alignment
            let aligned_header_size = (header_size + self.object_size - 1) & !(self.object_size - 1);
            let usable_size = SLAB_SIZE_BYTES - aligned_header_size;
            let min_objects_needed = 1;
            
            if usable_size < self.object_size * min_objects_needed {
                crate::error!("insufficient space in slab for objects of size {}", self.object_size);
                // Free the allocated pages since we can't use them
                physical::free_pages(page_addr, SLAB_SIZE_PAGES);
                return false;
            }
            
            unsafe {
                let slab_ptr = Slab::new(page_addr, self.object_size);
                
                // Verify slab was created successfully
                if slab_ptr.is_null() {
                    crate::error!("failed to create slab structure for cache size {}", self.object_size);
                    // Free the allocated pages since slab creation failed
                    physical::free_pages(page_addr, SLAB_SIZE_PAGES);
                    return false;
                }
                
                // Verify that the slab has free objects
                let free_count = (*slab_ptr).free_count.load(Ordering::Relaxed);
                if free_count == 0 {
                    crate::error!("newly created slab has no free objects for cache size {}", self.object_size);
                    // Free the allocated pages since slab has no free objects
                    physical::free_pages(page_addr, SLAB_SIZE_PAGES);
                    return false;
                }
                
                crate::info!("created slab with {} free objects of size {}", free_count, self.object_size);
                
                // Update statistics
                #[cfg(debug_assertions)]
                {
                    self.stats.slab_creations.fetch_add(1, Ordering::Relaxed);
                }
                
                // Atomically add to empty slabs list using compare-and-swap
                let mut retries = 0;
                const MAX_RETRIES: usize = 10;
                
                loop {
                    let old_head = self.empty_slabs.load(Ordering::Acquire);
                    (*slab_ptr).next.store(old_head, Ordering::Release);
                    
                    match self.empty_slabs.compare_exchange_weak(
                        old_head,
                        slab_ptr,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            crate::info!("slab initialized successfully for cache size {}", self.object_size);
                            return true;
                        }
                        Err(_) => {
                            retries += 1;
                            if retries >= MAX_RETRIES {
                                crate::error!("failed to add slab to empty list after {} retries for cache size {}", MAX_RETRIES, self.object_size);
                                // Free the allocated pages since we couldn't add the slab to the list
                                physical::free_pages(page_addr, SLAB_SIZE_PAGES);
                                return false;
                            }
                            // Brief pause before retry
                            core::hint::spin_loop();
                        }
                    }
                }
            }
        } else {
            crate::warn!("failed to allocate pages for slab of size {}", self.object_size);
            false
        }
    }
    
    /// Allocate an object from this cache
    fn allocate(&self) -> Option<*mut u8> {
        let _guard = self.lock.lock();
        
        // First try to allocate from a partial slab
        if let Some(ptr) = self.allocate_from_list(&self.partial_slabs) {
            #[cfg(debug_assertions)]
            {
                self.stats.allocations.fetch_add(1, Ordering::Relaxed);
            }
            return Some(ptr);
        }
        
        // Then try to allocate from an empty slab
        if let Some(ptr) = self.allocate_from_list(&self.empty_slabs) {
            #[cfg(debug_assertions)]
            {
                self.stats.allocations.fetch_add(1, Ordering::Relaxed);
            }
            return Some(ptr);
        }
        
        // If we couldn't allocate from existing slabs, try to create a new one
        if self.init_with_slab() {
            // Now try again to allocate from the newly created empty slab
            if let Some(ptr) = self.allocate_from_list(&self.empty_slabs) {
                #[cfg(debug_assertions)]
                {
                    self.stats.allocations.fetch_add(1, Ordering::Relaxed);
                }
                return Some(ptr);
            }
        }
        
        None
    }
    
    /// Helper function to allocate from a slab list
    fn allocate_from_list(&self, list_head: &AtomicPtr<Slab>) -> Option<*mut u8> {
        let mut current_ptr = list_head.load(Ordering::Acquire);
        
        while !current_ptr.is_null() {
            let slab = unsafe { &*current_ptr };
            if let Some(ptr) = slab.allocate() {
                // Check if the slab is now full
                let free_count = slab.free_count.load(Ordering::Acquire);
                
                // Validate free count is within expected bounds
                if free_count > slab.objects_per_slab {
                    crate::error!("invalid free count {} for slab with {} objects", 
                                 free_count, slab.objects_per_slab);
                    return Some(ptr); // Still return the allocated object despite inconsistency
                }
                
                if free_count == 0 {
                    // Move slab from current list to full list
                    self.move_slab(list_head, &self.full_slabs, current_ptr);
                } else if free_count == slab.objects_per_slab - 1 {
                    // Slab was empty and now is partial, move it
                    self.move_slab(list_head, &self.partial_slabs, current_ptr);
                }
                // For other cases, the slab stays in its current list
                
                crate::debug!("allocated object from slab {:p}, {} free objects remaining", 
                             current_ptr, free_count);
                return Some(ptr);
            }
            current_ptr = slab.next.load(Ordering::Acquire);
        }
        
        None
    }
    
    /// Deallocate an object back to this cache
    fn deallocate(&self, ptr: *mut u8) -> bool {
        let _guard = self.lock.lock();
        
        // Validate input pointer
        if ptr.is_null() {
            crate::warn!("attempted to deallocate null pointer");
            return false;
        }
        
        // Search lists for the containing slab
        let lists: [(&AtomicPtr<Slab>, SlabListType); 3] = [
            (&self.partial_slabs, SlabListType::Partial),
            (&self.empty_slabs, SlabListType::Empty),
            (&self.full_slabs, SlabListType::Full),
        ];
        
        for (list_head, list_type) in &lists {
            let mut current_ptr = list_head.load(Ordering::Acquire);
            
            while !current_ptr.is_null() {
                let slab = unsafe { &*current_ptr };
                if slab.contains(ptr) {
                    if !slab.deallocate(ptr) {
                        crate::warn!("failed to deallocate object at {:p} from slab {:p}", ptr, current_ptr);
                        return false;
                    }
                    
                    // Update statistics
                    #[cfg(debug_assertions)]
                    {
                        self.stats.deallocations.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    // Move slab between lists based on new free count
                    let free_count = slab.free_count.load(Ordering::Acquire);
                    
                    // Validate free count is within expected bounds
                    if free_count > slab.objects_per_slab {
                        crate::error!("invalid free count {} for slab with {} objects after deallocation", 
                                     free_count, slab.objects_per_slab);
                        return true; // Still consider it successful despite inconsistency
                    }
                    
                    if free_count == slab.objects_per_slab {
                        // Slab is now empty
                        if *list_type != SlabListType::Empty {
                            crate::debug!("moving slab {:p} to empty list ({} free objects)", 
                                         current_ptr, free_count);
                            self.move_slab(*list_head, &self.empty_slabs, current_ptr);
                        }
                    } else if free_count == 0 {
                        // Slab is now full
                        if *list_type != SlabListType::Full {
                            crate::debug!("moving slab {:p} to full list ({} free objects)", 
                                         current_ptr, free_count);
                            self.move_slab(*list_head, &self.full_slabs, current_ptr);
                        }
                    } else {
                        // Slab is now partial
                        let was_empty = *list_type == SlabListType::Empty;
                        let was_full = *list_type == SlabListType::Full;
                        
                        if was_empty || was_full {
                            crate::debug!("moving slab {:p} to partial list ({} free objects)", 
                                         current_ptr, free_count);
                            self.move_slab(*list_head, &self.partial_slabs, current_ptr);
                        }
                    }
                    
                    crate::debug!("successfully deallocated object at {:p} from slab {:p}", ptr, current_ptr);
                    return true;
                }
                current_ptr = slab.next.load(Ordering::Acquire);
            }
        }
        
        crate::warn!("failed to find containing slab for deallocated object at {:p}", ptr);
        false
    }
    
    /// Move a slab from one list to another
    fn move_slab(&self, from_list: &AtomicPtr<Slab>, to_list: &AtomicPtr<Slab>, slab_ptr: *mut Slab) {
        // Acquire lock to ensure atomicity of the move operation
        let _guard = self.lock.lock();
        
        // Remove from the from_list using compare-and-swap for thread safety
        let mut retries = 0;
        const MAX_RETRIES: usize = 10;
        
        loop {
            let mut head = from_list.load(Ordering::Acquire);
            let mut prev_ptr: *mut AtomicPtr<Slab> = from_list as *const _ as *mut AtomicPtr<Slab>;
            let mut found = false;
            
            // Traverse the list to find and remove the slab
            while !head.is_null() {
                if ptr::eq(head, slab_ptr) {
                    // Found the slab to remove
                    let next = unsafe { (*head).next.load(Ordering::Acquire) };
                    unsafe { 
                        (*prev_ptr).store(next, Ordering::Release);
                    }
                    found = true;
                    break;
                }
                prev_ptr = unsafe { &mut (*head).next };
                head = unsafe { (*head).next.load(Ordering::Acquire) };
            }
            
            if found {
                break; // Successfully removed from from_list
            }
            
            retries += 1;
            if retries >= MAX_RETRIES {
                crate::warn!("failed to find slab in from_list after {} retries", MAX_RETRIES);
                return; // Failed to find the slab, give up
            }
            
            // Brief pause before retry
            core::hint::spin_loop();
        }
        
        // Add to the to_list using compare-and-swap for thread safety
        retries = 0;
        loop {
            let old_head = to_list.load(Ordering::Acquire);
            unsafe {
                (*slab_ptr).next.store(old_head, Ordering::Release);
            }
            
            match to_list.compare_exchange_weak(
                old_head,
                slab_ptr,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break, // Successfully added to to_list
                Err(_) => {
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        crate::error!("failed to add slab to to_list after {} retries", MAX_RETRIES);
                        return; // Failed to add to the list, give up
                    }
                    // Brief pause before retry
                    core::hint::spin_loop();
                }
            }
        }
        
        crate::debug!("moved slab {:p} between lists", slab_ptr);
    }
    
    /// Get statistics for this cache (only available in debug builds)
    #[cfg(debug_assertions)]
    fn stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            allocations: self.stats.allocations.load(Ordering::Relaxed),
            deallocations: self.stats.deallocations.load(Ordering::Relaxed),
            slab_creations: self.stats.slab_creations.load(Ordering::Relaxed),
        }
    }
    
    /// Get statistics for this cache (returns dummy values in release builds)
    #[cfg(not(debug_assertions))]
    fn stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            allocations: 0,
            deallocations: 0,
            slab_creations: 0,
        }
    }
}

/// Type of slab list for tracking purposes
#[derive(PartialEq)]
enum SlabListType {
    Partial,
    Empty,
    Full,
}

/// Fixed-size caches for common object sizes
struct StaticSlabCaches {
    cache_16: SlabCache,
    cache_32: SlabCache,
    cache_64: SlabCache,
    cache_128: SlabCache,
    cache_256: SlabCache,
    cache_512: SlabCache,
    cache_1024: SlabCache,
    cache_2048: SlabCache,
}

/// The global slab allocator
pub struct SlabAllocator {
    /// Static caches for common sizes
    static_caches: StaticSlabCaches,
    /// Flag to track if allocator is initialized
    initialized: AtomicUsize,
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub const fn new() -> Self {
        Self {
            static_caches: StaticSlabCaches {
                cache_16: SlabCache::new(16),
                cache_32: SlabCache::new(32),
                cache_64: SlabCache::new(64),
                cache_128: SlabCache::new(128),
                cache_256: SlabCache::new(256),
                cache_512: SlabCache::new(512),
                cache_1024: SlabCache::new(1024),
                cache_2048: SlabCache::new(2048),
            },
            initialized: AtomicUsize::new(0),
        }
    }
    
    /// Initialize the slab allocator
    pub fn init(&self) {
        if self.initialized.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            crate::info!("initializing slab allocator");
            
            // Initialize all static caches with at least one slab each
            let cache_results = [
                ("16-byte", self.static_caches.cache_16.init_with_slab()),
                ("32-byte", self.static_caches.cache_32.init_with_slab()),
                ("64-byte", self.static_caches.cache_64.init_with_slab()),
                ("128-byte", self.static_caches.cache_128.init_with_slab()),
                ("256-byte", self.static_caches.cache_256.init_with_slab()),
                ("512-byte", self.static_caches.cache_512.init_with_slab()),
                ("1024-byte", self.static_caches.cache_1024.init_with_slab()),
                ("2048-byte", self.static_caches.cache_2048.init_with_slab()),
            ];
            
            // Count successful initializations
            let mut success_count = 0;
            for (cache_name, success) in &cache_results {
                if *success {
                    success_count += 1;
                } else {
                    crate::warn!("failed to initialize {} cache", cache_name);
                }
            }
            
            if success_count == cache_results.len() {
                crate::info!("slab allocator initialized successfully with {} caches", success_count);
            } else {
                crate::warn!("slab allocator initialized with only {}/{} caches", success_count, cache_results.len());
                // Log which caches failed to initialize
                for (cache_name, success) in &cache_results {
                    if !success {
                        crate::warn!("  - {} cache initialization failed", cache_name);
                    }
                }
                // Even if some caches failed to initialize, we can still proceed
                // The allocator will fall back to the buddy allocator for failed cache sizes
            }
        } else {
            crate::info!("slab allocator already initialized");
        }
    }
    
    /// Get the appropriate cache for a given layout
    fn get_cache_for_layout(&self, layout: Layout) -> Option<&SlabCache> {
        let size = layout.size().max(layout.align());
        
        // Select the appropriate cache based on size
        match size {
            0..=16 => Some(&self.static_caches.cache_16),
            17..=32 => Some(&self.static_caches.cache_32),
            33..=64 => Some(&self.static_caches.cache_64),
            65..=128 => Some(&self.static_caches.cache_128),
            129..=256 => Some(&self.static_caches.cache_256),
            257..=512 => Some(&self.static_caches.cache_512),
            513..=1024 => Some(&self.static_caches.cache_1024),
            1025..=2048 => Some(&self.static_caches.cache_2048),
            _ => None, // Too large for slab allocator
        }
    }
    
    /// Allocate memory using the slab allocator
    pub fn alloc(&self, layout: Layout) -> Option<*mut u8> {
        // For very large allocations, fall back to the buddy allocator
        if layout.size() > SLAB_SIZE_BYTES / 2 {
            return None;
        }
        
        if let Some(cache) = self.get_cache_for_layout(layout) {
            cache.allocate()
        } else {
            None
        }
    }
    
    /// Deallocate memory using the slab allocator
    pub fn dealloc(&self, ptr: *mut u8, layout: Layout) -> bool {
        if let Some(cache) = self.get_cache_for_layout(layout) {
            cache.deallocate(ptr)
        } else {
            false
        }
    }
    
    /// Get statistics for all caches
    pub fn stats(&self) -> SlabAllocatorStats {
        SlabAllocatorStats {
            cache_16: self.static_caches.cache_16.stats(),
            cache_32: self.static_caches.cache_32.stats(),
            cache_64: self.static_caches.cache_64.stats(),
            cache_128: self.static_caches.cache_128.stats(),
            cache_256: self.static_caches.cache_256.stats(),
            cache_512: self.static_caches.cache_512.stats(),
            cache_1024: self.static_caches.cache_1024.stats(),
            cache_2048: self.static_caches.cache_2048.stats(),
        }
    }
}
    
/// Global slab allocator instance
pub static SLAB_ALLOCATOR: SlabAllocator = SlabAllocator::new();

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_slab_creation() {
        // Tests would go here
    }
}
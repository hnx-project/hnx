//! Enhanced test for the slab allocator functionality

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnx_kernel::memory::slab_allocator::SLAB_ALLOCATOR;
use core::alloc::Layout;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn _start() -> ! {
    // Test basic functionality
    crate::println!("Testing slab allocator...");
    
    // This would normally be called during kernel initialization
    SLAB_ALLOCATOR.init();
    
    // Test allocation for different sizes
    let test_sizes = [16, 32, 64, 128, 256, 512, 1024, 2048];
    
    for &size in &test_sizes {
        let layout = Layout::from_size_align(size, 8).unwrap();
        
        // Try to allocate memory
        let ptr = SLAB_ALLOCATOR.alloc(layout);
        
        if let Some(p) = ptr {
            crate::println!("Allocation succeeded for size {}: {:p}", size, p);
            
            // Write to the allocated memory to ensure it's accessible
            unsafe {
                core::ptr::write_bytes(p, 0xAA, size);
            }
            
            // Deallocate the memory
            SLAB_ALLOCATOR.dealloc(p, layout);
            crate::println!("Deallocation succeeded for size {}", size);
        } else {
            crate::println!("Allocation failed for size {}", size);
        }
    }
    
    crate::println!("Slab allocator test completed!");
    
    // Exit the program
    loop {}
}
//! Test for the slab allocator

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
    let layout = Layout::from_size_align(32, 8).unwrap();
    
    // This would normally be called during kernel initialization
    // SLAB_ALLOCATOR.init();
    
    // Try to allocate memory
    let ptr = SLAB_ALLOCATOR.alloc(layout);
    
    if let Some(p) = ptr {
        // Allocation succeeded
        // In a real test, we would verify the memory is accessible
        
        // Deallocate the memory
        SLAB_ALLOCATOR.dealloc(p, layout);
    }
    
    // Exit the program
    loop {}
}
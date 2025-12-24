#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use core::alloc::{GlobalAlloc, Layout};
use hnx_abi::*;

mod ramfs;
mod ipc;

use ramfs::RamFs;
use ipc::VfsIpcHandler;

struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: core::sync::atomic::AtomicUsize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: core::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next.store(heap_start, core::sync::atomic::Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        
        loop {
            let current = self.next.load(core::sync::atomic::Ordering::Relaxed);
            let aligned = (current + align - 1) & !(align - 1);
            let new_next = aligned + size;
            
            if new_next > self.heap_end {
                return core::ptr::null_mut();
            }
            
            if self.next.compare_exchange(
                current,
                new_next,
                core::sync::atomic::Ordering::Relaxed,
                core::sync::atomic::Ordering::Relaxed
            ).is_ok() {
                return aligned as *mut u8;
            }
        }
    }
    
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
    }
}

#[global_allocator]
static mut ALLOCATOR: BumpAllocator = BumpAllocator::new();

static mut HEAP: [u8; 65536] = [0; 65536];

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }
    
    let mut fs = RamFs::new();
    
    fs.mkdir("/").ok();
    fs.mkdir("/bin").ok();
    fs.mkdir("/tmp").ok();
    
    let mut handler = VfsIpcHandler::new(fs);
    
    let vfs_epid: u32 = 1;
    
    loop {
        match handler.wait_and_handle(vfs_epid) {
            Ok(_) => {},
            Err(_) => {},
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

//! HNX Microkernel - Main Entry Point
#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(alloc)]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};

/// 简单的内核分配器（占位符）
struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        panic!("no allocator implemented")
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("no allocator implemented")
    }
}

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    panic!("allocation error")
}

mod debug;
mod arch;
mod memory;
mod object;
mod syscall;

use core::panic::PanicInfo;

/// HNX Microkernel Rust entry point (called from assembly boot code)
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // 早期调试输出初始化
    unsafe {
        debug::init();
    }

    // 输出启动信息
    println!("\n========================================");
    println!("       HNX Microkernel (AArch64)");
    println!("========================================\n");

    // 初始化架构相关功能
    println!("[1/5] Initializing architecture...");
    arch::platform::init();

    // 初始化内存管理
    println!("[2/5] Initializing memory management...");
    arch::mmu::init();

    // 初始化中断控制器
    println!("[3/5] Initializing interrupts...");
    arch::interrupt::init();

    // 初始化定时器
    println!("[4/5] Initializing timer...");
    arch::timer::init();

    // 初始化对象系统
    println!("[5/5] Initializing object system...");
    crate::object::init();

    println!("\nHNX Microkernel initialized successfully!");
    println!("Kernel is now entering main loop...\n");

    loop {
        arch::cpu::wait_for_interrupt();
    }
}

/// 内核panic处理函数
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n[PANIC] HNX Microkernel panic:");

    if let Some(location) = info.location() {
        println!("  at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    println!("  {}", info);

    println!("\nSystem halted.");

    // 进入无限循环
    loop {
        arch::cpu::wait_for_interrupt();
    }
}

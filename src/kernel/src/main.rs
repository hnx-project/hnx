#![no_std]
#![no_main]

mod panic;

// mod arch;
// mod core;

// use core::init::kernel_init;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // kernel_init();
    loop {}
}
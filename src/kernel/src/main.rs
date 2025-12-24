#![no_std]
#![no_main]

mod panic;

// mod arch;
// mod console;

// use core::init::kernel_init;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // kernel_init();
    loop {}
}
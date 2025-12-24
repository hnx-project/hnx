#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod elf;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnx_abi::*;

mod elf;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Simple debug output using system call
    let msg = "[loader-service] Started!\n";
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") HNX_SYS_WRITE,
            in("x0") 1usize,   // stdout
            in("x1") msg.as_ptr() as usize,
            in("x2") msg.len(),
            options(nostack)
        );
    }

    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
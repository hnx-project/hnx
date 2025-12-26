#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Use direct syscall for output to avoid complex formatting
    hnxlib::syscall::write(1, b"Init: PID 1 starting...\n");
    
    hnxlib::syscall::write(1, b"Init: System services ready to start\n");
    
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
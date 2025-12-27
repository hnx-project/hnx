#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Use libc crate's write function
use libc::write;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Try ONE simple write via direct assembly
    unsafe {
        // Use stack array instead of static reference
        let mut msg: [u8; 32] = [0; 32];
        let text = b"INIT: Hello from init process!\n";
        msg[..text.len()].copy_from_slice(text);

        let mut fd: usize = 1;
        let mut buf: usize = msg.as_ptr() as usize;
        let mut len: usize = text.len();
        let mut syscall_num: usize = 0x1001;  // HNX_SYS_WRITE
        let ret: isize;

        // Debug: try to ensure registers are set
        core::arch::asm!(
            "svc #0",
            inout("x8") syscall_num => _,
            inout("x0") fd => ret,
            inout("x1") buf => _,
            inout("x2") len => _,
            options(nostack, preserves_flags)
        );
    }

    // Simple infinite loop (no wfi)
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
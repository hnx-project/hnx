#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnx_abi::*;

const VFS_ENDPOINT: u32 = 1;
const LOADER_ENDPOINT: u32 = 3;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    debug_print(b"Init: PID 1 starting...\n");
    
    debug_print(b"Init: System services ready to start\n");
    
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

fn debug_print(msg: &[u8]) {
    unsafe {
        syscall3(HNX_SYS_WRITE, 1, msg.as_ptr() as usize, msg.len());
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

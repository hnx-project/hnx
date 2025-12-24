//! 内核恐慌处理

use core::panic::PanicInfo;

/// 恐慌处理器
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // TODO 输出恐慌信息
    
    // Halt the system
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
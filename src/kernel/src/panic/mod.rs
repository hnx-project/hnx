//! 内核恐慌处理

use core::panic::PanicInfo;

/// 恐慌处理器
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print panic information
    crate::console::write(format_args!("\nKERNEL PANIC at {}:{}:{}\n", 
        info.location().map(|l| l.file()).unwrap_or("<unknown>"),
        info.location().map(|l| l.line()).unwrap_or(0),
        info.location().map(|l| l.column()).unwrap_or(0)
    ));
    crate::console::write(format_args!("Message: {}\n", info.message()));
    
    // Dump CPU state if available
    crate::arch::dump_panic_state();
    
    // Halt the system
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
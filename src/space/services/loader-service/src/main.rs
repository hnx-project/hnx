#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 简单测试：输出信息并调用yield系统调用

    // 首先尝试输出一条消息
    let msg = b"loader-service started!\n";
    unsafe {
        // 使用内联汇编调用write系统调用
        // HNX_SYS_WRITE = 0x1001, fd=1 (stdout)
        core::arch::asm!(
            "svc #0",
            in("x8") 0x1001u64,  // syscall number
            in("x0") 1u64,       // fd (stdout)
            in("x1") msg.as_ptr(),
            in("x2") msg.len(),
        );
    }

    // 然后调用yield系统切换回init
    unsafe {
        // HNX_SYS_YIELD = 24 (0x18)
        core::arch::asm!(
            "svc #0",
            in("x8") 24u64,  // HNX_SYS_YIELD
        );
    }

    // 如果yield返回（不应该发生），继续循环
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
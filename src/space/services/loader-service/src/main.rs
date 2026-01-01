#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::println;
use hnx_abi::*;
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 简单测试：输出信息并调用yield系统调用

    // 首先尝试输出一条消息
    let msg = b"loader-service started!\n";
    println!("{}", core::str::from_utf8(msg).unwrap());
    println!("loader-service: calling yield");
    // 然后调用yield系统切换回init
    unsafe {
        // HNX_SYS_YIELD = 24 (0x18)
        // 明确清零所有参数寄存器
        let syscall_num: u64 = 24;
        core::arch::asm!(
            "svc #0",
            in("x8") syscall_num,
            in("x0") 0u64,
            in("x1") 0u64,
            in("x2") 0u64,
            in("x3") 0u64,
            in("x4") 0u64,
            in("x5") 0u64,
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
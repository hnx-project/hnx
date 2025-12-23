//! aarch64 启动代码

use crate::arch::common::boot::BootInfo;
use core::arch::global_asm;

// 汇编启动代码
global_asm!(include_str!("../boot.S"));
// 用户态自检片段 - ⚠️
// global_asm!(include_str!("../selftest.S"));

/// 清零 BSS 段
pub unsafe fn zero_bss() {
    extern "C" {
        static mut __bss_start: u64;
        static mut __bss_end: u64;
    }

    let bss_start = core::ptr::addr_of_mut!(__bss_start) as *mut u8;
    let bss_end = core::ptr::addr_of_mut!(__bss_end) as *mut u8;
    let bss_size = bss_end as usize - bss_start as usize;

    core::ptr::write_bytes(bss_start, 0, bss_size);
}

/// 收集启动信息
pub fn get_boot_info() -> BootInfo {
    // TODO 这是是不是应该针对不同机器？
    BootInfo {
        phys_mem_start: 0x40000000, // QEMU virt 机器的内存起始地址
        phys_mem_size: 0x40000000,  // 1GB 内存
        cmdline: b"console=ttyAMA0 log=trace\0".as_ptr(),
        initrd_start: 0x42000000,
        initrd_size: 0,
        dtb_ptr: 0x40000000, // 设备树地址
    }
}

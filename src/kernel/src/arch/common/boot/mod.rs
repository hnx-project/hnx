//! 通用启动信息

/// 启动信息结构
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// 物理内存起始地址
    pub phys_mem_start: u64,
    /// 物理内存大小
    pub phys_mem_size: u64,
    /// 内核命令行参数
    pub cmdline: *const u8,
    /// 初始化 RAM 磁盘起始地址
    pub initrd_start: u64,
    /// 初始化 RAM 磁盘大小
    pub initrd_size: u64,
    /// 设备树地址
    pub dtb_ptr: u64,
}

// 预留启动入口（占位以保持模块完整）
pub fn boot_main(_boot_info: BootInfo) -> ! {
    loop {}
}

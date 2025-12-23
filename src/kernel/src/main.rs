//! HNX 内核主入口
//! I dont like Tuesday
#![no_std]
#![no_main]
#![feature(format_args_nl)]

// 核心模块
mod arch;
mod console;
mod error;
mod sync;

// 重构后的核心模块
#[path = "core/mod.rs"]
mod kernel_core;
mod memory;
mod panic;
mod process;
mod drivers;
mod security;

const KERNEL_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARCH: &str = crate::arch::ARCH_NAME;

/// Rust 入口点（由汇编代码调用）
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // 初始化控制台（先启用，再进行 DTB 映射与日志）
    crate::console::init();
    
    // 设备树调度驱动与 MMIO 映射
    let bi0 = crate::arch::boot::get_boot_info();
    crate::drivers::init_from_dtb(&bi0);
    crate::console::driver_ready();
    
    println_raw!("======= HNX Kernel Booting =======");
    crate::info!("config platform {}", env!("HNX_PLATFORM_DEFAULT"));
    crate::info!("config uart {}", env!("HNX_UART_DEFAULT"));
    crate::info!("config irq {}", env!("HNX_IRQ_DEFAULT"));
    crate::info!("config timer_ms {}", env!("HNX_TIMER_DEFAULT_MS"));
    
    // 初始化架构
    crate::info!("about to initialize architecture");
    crate::arch::init();
    crate::info!("architecture initialized");
    
    // 解析启动参数（日志级别）
    {
        let bi = crate::arch::boot::get_boot_info();
        unsafe {
            let mut len = 0usize;
            while *bi.cmdline.add(len) != 0 {
                len += 1;
            }
            let s = core::str::from_utf8_unchecked(core::slice::from_raw_parts(bi.cmdline, len));
            let mut lvl = crate::console::loglvl::LogLevel::Info;
            if let Some(p) = s.find("log=") {
                let t = &s[p + 4..];
                if t.starts_with("trace") {
                    lvl = crate::console::loglvl::LogLevel::Trace;
                } else if t.starts_with("debug") {
                    lvl = crate::console::loglvl::LogLevel::Debug;
                } else if t.starts_with("info") {
                    lvl = crate::console::loglvl::LogLevel::Info;
                } else if t.starts_with("warn") {
                    lvl = crate::console::loglvl::LogLevel::Warn;
                } else if t.starts_with("error") {
                    lvl = crate::console::loglvl::LogLevel::Error;
                }
            }
            crate::console::loglvl::set_log_level(lvl);
        }
    }
    
    crate::info!("BOOT {} {} ({})", KERNEL_NAME, VERSION, ARCH);
    
    // 打印 VBAR_EL1 和 CurrentEL
    let mut vbar: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
    }
    crate::info!("arch VBAR_EL1=0x{:016X}", vbar);
    
    let mut cur_el: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cur_el);
    }
    crate::info!("arch CurrentEL=0x{:016X}", cur_el);
    
    // 初始化内存
    let boot_info = crate::arch::boot::get_boot_info();
    memory::init(boot_info);
    
    // 初始化全局分配器
    crate::info!("initializing buddy allocator");
    let heap_start = 0x40000000; // 示例堆起始地址
    let heap_size = 0x10000000;  // 示例堆大小 (256MB)
    unsafe {
        crate::memory::buddy_allocator::ALLOCATOR.init(heap_start, heap_size);
    }
    crate::info!("buddy allocator initialized");
    crate::info!("memory ok");
    
    // 注释掉文件系统初始化（移至用户空间）
    // fs::init();
    // crate::info!("fs ok");
    
    // 注释掉网络子系统初始化（移至用户空间）
    // network::init();
    // crate::info!("network ok");
    
    // 注释掉调试接口初始化（移至用户空间）
    // if let Err(e) = debug::init() {
    //     crate::error!("Failed to initialize debug interface: {:?}", e);
    // } else {
    //     crate::info!("debug interface initialized");
    // }
    
    // 初始化进程管理
    crate::info!("about to initialize process subsystem");
    process::init();
    crate::info!("process ok");
    
    crate::info!("kernel core ready");
    crate::info!("About to start scheduler");
    
    // 启动调度器
    crate::kernel_core::scheduler::run();
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    rust_main()
}
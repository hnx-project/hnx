//! HNX 微内核主入口
//! 微内核仅提供核心功能：
//! 1. 进程与线程管理
//! 2. 内存管理
//! 3. 进程间通信（IPC）
//! 4. 中断与异常处理
//! 5. 基本设备抽象
//! 6. 安全与能力
#![no_std]
#![no_main]
#![feature(format_args_nl)]

// 核心模块
mod arch;
mod console;
mod error;
mod sync;

// 核心内核模块（IPC、调度器）
mod core;
mod memory;
mod panic;
mod process;
mod drivers;
mod security;

// TODO: 添加以下模块（待实现）
// mod ipc;       // 进程间通信
// mod syscall;   // 系统调用处理
// mod capability; // 能力管理（可从security模块拆分）

const KERNEL_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARCH: &str = crate::arch::ARCH_NAME;

/// HNX 微内核 Rust 入口点（由汇编代码调用）
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // 阶段1：基础初始化
    init_phase1();
    
    // 阶段2：内存管理初始化
    // init_phase2();
    
    // // 阶段3：进程与IPC初始化
    // init_phase3();
    
    // // 阶段4：启动调度器
    // init_phase4();

    // 启动调度器（永不返回）
    crate::core::scheduler::run();
}

/// 阶段1：基础硬件初始化
fn init_phase1() {
    // 1. 初始化控制台（仅用于内核调试输出）
    crate::console::init();
    
    // 2. 设备树解析和基本设备映射
    // let boot_info = crate::arch::boot::get_boot_info();
    // crate::drivers::init_from_dtb(&boot_info);
    crate::console::driver_ready();
    
    println_raw!("======= HNX Microkernel Booting =======");
    crate::info!("Kernel: {} {} ({})", KERNEL_NAME, VERSION, ARCH);
    
    // 3. 初始化架构相关组件（中断、定时器、MMU）
    crate::info!("Initializing architecture...");
    crate::arch::init();
    crate::info!("Architecture initialized");
    
    // 4. 解析启动参数
    // parse_boot_params(&boot_info);
    
    // 5. 打印关键寄存器状态（调试用）
    // print_cpu_state();
}

/// 阶段2：内存管理初始化
fn init_phase2() {
    let boot_info = crate::arch::boot::get_boot_info();
    
    // 1. 初始化物理内存管理
    crate::info!("Initializing memory subsystem...");
    memory::init(boot_info);
    
    // 2. 初始化内核堆分配器
    crate::info!("Initializing buddy allocator...");
    let heap_start = 0x40000000; // TODO: 从设备树获取
    let heap_size = 0x10000000;  // TODO: 从设备树获取
    unsafe {
        crate::memory::buddy_allocator::ALLOCATOR.init(heap_start, heap_size);
    }
    crate::info!("Memory subsystem ready");
}

/// 阶段3：进程管理与IPC初始化
fn init_phase3() {
    // 1. 初始化进程管理
    crate::info!("Initializing process subsystem...");
    process::init();
    
    // 2. TODO: 初始化IPC子系统
    // ipc::init();
    
    // 3. TODO: 初始化能力安全系统
    // capability::init();
    
    // 4. TODO: 初始化系统调用处理
    // syscall::init();
    
    crate::info!("Process and IPC subsystems ready");
}

/// 阶段4：启动第一个进程并进入调度循环
fn init_phase4() {
    crate::info!("Kernel core ready");
    crate::info!("Starting scheduler...");
    
    // TODO: 创建第一个用户空间进程（init或procmgr）
    // 这个进程将负责启动用户空间服务
}

/// 解析启动参数
fn parse_boot_params(boot_info: &crate::arch::common::BootInfo) {
    unsafe {
        let mut len = 0usize;
        while *boot_info.cmdline.add(len) != 0 {
            len += 1;
        }
        let cmdline = ::core::str::from_utf8_unchecked(
            ::core::slice::from_raw_parts(boot_info.cmdline, len)
        );
        
        let mut log_level = crate::console::loglvl::LogLevel::Info;
        if let Some(pos) = cmdline.find("log=") {
            let level_str = &cmdline[pos + 4..];
            log_level = match level_str {
                s if s.starts_with("trace") => crate::console::loglvl::LogLevel::Trace,
                s if s.starts_with("debug") => crate::console::loglvl::LogLevel::Debug,
                s if s.starts_with("info") => crate::console::loglvl::LogLevel::Info,
                s if s.starts_with("warn") => crate::console::loglvl::LogLevel::Warn,
                s if s.starts_with("error") => crate::console::loglvl::LogLevel::Error,
                _ => crate::console::loglvl::LogLevel::Info,
            };
        }
        crate::console::loglvl::set_log_level(log_level);
        
        crate::info!("Boot command line: {}", cmdline);
        // crate::info!("Log level: {:?}", log_level);
    }
}

/// 打印CPU状态（调试用）
fn print_cpu_state() {
    // VBAR_EL1
    let mut vbar: u64;
    unsafe {
        ::core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
    }
    crate::info!("VBAR_EL1 = 0x{:016X}", vbar);
    
    // CurrentEL
    let mut cur_el: u64;
    unsafe {
        ::core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cur_el);
    }
    crate::info!("CurrentEL = 0x{:016X}", cur_el);
    
    // TODO: 添加更多架构相关的状态信息
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    rust_main()
}
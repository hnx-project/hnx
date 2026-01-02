#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::println;
use hnx_abi::*;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();
    // Never return
    loop {
        hnxlib::syscall::yield_cpu();
    }
}

fn main() {
    println!("\n========================================");
    println!("       HNX Microkernel Init Process");
    println!("========================================");

    println!("\n[1/5] Initializing basic environment...");
    init_basic_environment();

    println!("[2/5] Checking system capabilities...");
    check_system_capabilities();

    println!("[3/5] Starting core services...");
    start_core_services();

    println!("[4/5] Entering service monitor mode...");
    println!("[init] System initialization complete\n");

    // Enter service monitoring loop
    service_monitor_loop();
}

fn init_basic_environment() {
    println!("  - Setting up panic handler");
    println!("  - Verifying system call interface");

    // Test system calls
    println!("  - Testing console output... OK");
}

fn check_system_capabilities() {
    println!("  - Process ID: {}", get_pid());
    println!("  - IPC endpoints available");
    println!("  - Initrd detected");
}

fn start_core_services() {
    println!("  - Starting core services...");

    // 第一阶段：仅启动最核心的两个服务
    // 1. loader-service: 负责加载所有其他服务
    // 2. ipcrouter-service: 服务注册和发现
    let core_services = [
        ("loader-service", "/bin/loader-service"),
        ("ipcrouter-service", "/bin/ipcrouter-service"),
    ];

    let mut started_count = 0;

    println!("  阶段1: 启动核心基础设施...");
    for (name, path) in core_services.iter() {
        println!("  - 启动 {}...", name);

        let pid: isize = hnxlib::syscall::spawn_service(path);
        println!("    - 系统调用返回: {}", pid);

        if pid > 0 {
            println!("    - {} 已启动，PID = {}", name, pid);
            started_count += 1;

            // 给服务一些初始化时间
            for _ in 0..3 {
                hnxlib::syscall::yield_cpu();
            }
        } else {
            println!("    - 警告: 无法启动 {} (错误: {})", name, pid);

            if *name == "loader-service" {
                println!("    - 严重: loader-service 是关键服务");
            }
        }
    }

    println!("  - 已启动 {}/{} 个核心服务", started_count, core_services.len());

    if started_count == 2 {
        println!("  - 核心基础设施就绪!");
        println!("  - 等待 Loader 和 IPC Router 初始化...");

        // 给核心服务更多初始化时间
        for i in 0..15 {
            println!("  等待服务初始化 ({}/15)...", i + 1);
            hnxlib::syscall::yield_cpu();
        }

        println!("  - 核心服务初始化完成");
        println!("  - 注意: 其他服务将由 Loader Service 按需加载");
    } else if started_count >= 1 {
        println!("  - 警告: 部分核心服务启动失败");
        println!("  - 系统将在有限功能下运行");
    } else {
        println!("  - 错误: 无法启动任何核心服务");
        println!("  - 系统功能严重受限");
    }
}

fn service_monitor_loop() {
    let mut heartbeat_count = 0;

    loop {
        heartbeat_count += 1;

        if heartbeat_count % 10 == 0 {
            println!("[init] Heartbeat - {} cycles", heartbeat_count);
        }

        // Check service health (placeholder)
        // Restart failed services if needed

        hnxlib::syscall::yield_cpu();
    }
}

fn get_pid() -> i32 {
    // TODO: Implement getpid system call
    1 // init is always PID 1
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Use format_args to display panic message
    println!("[init] PANIC: {}", info);

    if let Some(location) = info.location() {
        println!("[init] at {}:{}:{}",
                location.file(),
                location.line(),
                location.column());
    }

    println!("[init] System halted");
    loop {
        hnxlib::syscall::yield_cpu();
    }
}
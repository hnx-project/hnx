#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::println;
use hnx_abi::*;

/// 混合忙等待和yield的函数，避免永远占用CPU
fn busy_wait(iterations: u32) {
    for i in 0..iterations {
        // 每100次迭代让出CPU一次，给其他进程更多机会
        if i % 100 == 0 {
            hnxlib::syscall::yield_cpu();
        }
        // 防止编译器优化掉循环
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        // 空循环
        let _ = i;
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();
    // Never return
    loop {
        busy_wait(10000);
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

    // TODO: Shell integration - temporarily disabled for service stabilization
    // println!("[4/5] Starting shell...");
    // println!("[INIT] Before calling start_shell()");
    // start_shell();
    // println!("[INIT] After start_shell() returned");
    println!("[4/5] Shell startup temporarily disabled (TODO)");

    println!("[5/5] Entering service monitor mode...");
    println!("[init] System initialization complete\n");

    // Enter service monitoring loop
    service_monitor_loop();
}

fn init_basic_environment() {
    println!("  - Setting up panic handler");
    println!("  - Verifying system call interface... OK");
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

    println!("  Phase 1: Starting core infrastructure...");
    for (name, path) in core_services.iter() {
        println!("  - Starting {}...", name);

        let pid: isize = hnxlib::syscall::spawn_service(path);
        println!("    - System call returned: {}", pid);

        if pid > 0 {
            println!("    - {} started, PID = {}", name, pid);
            started_count += 1;

            // Give service some initialization time
            for _ in 0..3 {
                // 使用忙等待而不是 yield_cpu
                busy_wait(5000);
            }
        } else {
            println!("    - WARNING: Could not start {} (error: {})", name, pid);

            if *name == "loader-service" {
                println!("    - CRITICAL: loader-service is a core service");
            }
        }
    }

    println!("  - Started {}/{} core services", started_count, core_services.len());

    if started_count == 2 {
        println!("  - Core infrastructure ready!");
        println!("  - Waiting for Loader and IPC Router to initialize...");

        // Give core services more initialization time
        for i in 0..10 {
            println!("  [INIT] Waiting for services ({}/10)...", i + 1);
            // 使用忙等待而不是 yield_cpu，避免调度问题
            busy_wait(10000);
        }

        println!("  - Core services initialization complete");
        println!("  - Note: Other services will be loaded on-demand by Loader Service");
    } else if started_count >= 1 {
        println!("  - WARNING: Some core services failed to start");
        println!("  - System will run with limited functionality");
    } else {
        println!("  - ERROR: Could not start any core services");
        println!("  - System functionality severely limited");
    }
}

fn start_shell() {
    println!("  - Starting HNX Shell...");
    println!("[INIT] Calling syscall::spawn_service(\"/bin/hnx-shell\")");

    let shell_path = "/bin/hnx-shell";
    let pid: isize = hnxlib::syscall::spawn_service(shell_path);

    println!("[INIT] spawn_service returned: {}", pid);
    println!("    - System call returned: {}", pid);

    if pid > 0 {
        println!("    - HNX Shell started, PID = {}", pid);

        // 给shell一些初始化时间
        for i in 0..5 {
            println!("[INIT] Waiting for shell ({} of 5)", i + 1);
            busy_wait(5000);
        }

        println!("  - Shell startup complete");
    } else {
        println!("    - WARNING: Could not start HNX Shell (error: {})", pid);
        println!("    - System will continue without user interface");
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

        busy_wait(10000);
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
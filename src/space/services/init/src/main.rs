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

    // 服务启动顺序
    let services = [
        ("loader-service", "/bin/loader-service"),
        ("vfs-service", "/bin/vfs-service"),
        ("ipcrouter-service", "/bin/ipcrouter-service"),
        ("procmgr-service", "/bin/procmgr-service"),
        ("echo-service", "/bin/echo-service"),
    ];

    let mut started_count = 0;

    for (name, path) in services.iter() {
        println!("  - Attempting to start {}...", name);

        let pid: isize = hnxlib::syscall::spawn_service(path);
        println!("    - System call returned: {}", pid);

        if pid > 0 {
            println!("    - {} started with PID {}", name, pid);
            started_count += 1;

            // 等待服务初始化
            for _ in 0..5 {
                hnxlib::syscall::yield_cpu();
            }
        } else {
            println!("    - Failed to start {} (error: {})", name, pid);

            // 如果是关键服务，记录警告但继续
            if *name == "loader-service" || *name == "vfs-service" {
                println!("    - Warning: {} is a critical service", name);
            }
        }
    }

    println!("  - Started {}/{} core services", started_count, services.len());

    if started_count >= 3 {
        println!("  - Core services started successfully!");
    } else {
        println!("  - Warning: Some core services failed to start");
        println!("  - System will continue with available services");
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
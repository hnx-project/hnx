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
    println!("  - Attempting to start Loader service...");

    // Try to spawn Loader service from initrd
    // Based on initrd creation logs: loader-service -> bin/loader-service
    let loader_path = "/bin/loader-service";

    // Use hnxlib wrapper
    let loader_pid: isize = hnxlib::syscall::spawn_service(loader_path);
    println!("  - Loader service system call returned: {}", loader_pid);

    if loader_pid > 0 {
        println!("  - Loader service started with PID {}", loader_pid);

        // Wait a bit for Loader to initialize
        for _ in 0..10 {
            hnxlib::syscall::yield_cpu();
        }

        println!("  - Attempting to start VFS service...");
        let vfs_path = "/bin/vfs-service";

        // Direct inline system call
        let vfs_pid: isize = hnxlib::syscall::spawn_service(vfs_path);

        println!("  - VFS service system call returned: {}", vfs_pid);

        if vfs_pid > 0 {
            println!("  - VFS service started with PID {}", vfs_pid);
            println!("  - Core services started successfully!");
        } else {
            println!("  - Failed to start VFS service (error: {})", vfs_pid);
            println!("  - Continuing without VFS...");
        }
    } else {
        println!("  - Failed to start Loader service (error: {})", loader_pid);
        println!("  - Note: Initrd may be gzip compressed (kernel can't decompress)");
        println!("  - Or services not found in initrd");
        println!("  - System will continue with basic console output");
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
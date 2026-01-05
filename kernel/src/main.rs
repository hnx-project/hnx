//! HNX Microkernel - Main Entry Point
//!
//! The HNX microkernel provides only the essential functionality:
//! 1. **Process & Thread Management**: Scheduling, context switching
//! 2. **Memory Management**: Virtual memory, physical page allocation
//! 3. **Inter-Process Communication (IPC)**: Message passing, shared memory
//! 4. **Interrupt & Exception Handling**: Hardware interrupt routing
//! 5. **Minimal Device Abstractions**: UART, GIC, timer
//! 6. **Security & Capabilities**: Fine-grained access control
//!
//! All other functionality (filesystem, networking, device drivers) runs in user space.

#![no_std]
#![no_main]
#![feature(format_args_nl)]

// ===== Core Microkernel Modules =====
mod arch;           // Architecture-specific code (aarch64, riscv64, x86_64)
mod console;        // Debug console output
mod core;           // IPC and Scheduler
mod drivers;        // Minimal drivers (UART, GIC, DTB)
mod error;          // Error types
mod ipc_services;   // IPC service delegation framework
mod memory;         // Memory management
mod panic;          // Panic handler
mod process;        // Process/thread management
mod security;       // Capability-based security

mod user;           // User space process support
mod loader;         // ELF/CPIO loader and service manager

mod kernel;         // HNX Kernel Object

// ===== Non-Core Modules (TODO: Move to User Space) =====
// These modules should eventually be moved to user space services:
// - fs/        → File System Service
// - network/   → Network Stack Service
// - debug/     → Debugging Service
// - testing/   → Test Framework

const KERNEL_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARCH: &str = crate::arch::ARCH_NAME;

/// HNX Microkernel Rust entry point (called from assembly boot code)
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // Initialize the global Kernel object. This must be the first step.
    kernel::init();

    init_phase1_hardware();
    init_phase2_memory();
    init_phase3_processes();
    init_phase4_scheduler();
}

/// Phase 1: Hardware Initialization
///
/// Initializes the minimal hardware required for kernel operation:
/// - Debug console (UART)
/// - Device Tree Blob (DTB) parsing
/// - Interrupt controller (GIC)
/// - Timer
fn init_phase1_hardware() {
    // Initialize debug console (UART)
    crate::console::init();
    
    // Parse Device Tree and initialize core drivers
    let boot_info = crate::arch::boot::get_boot_info();
    crate::drivers::init_from_dtb(&boot_info);
    crate::console::driver_ready();
    
    // Print kernel boot banner
    println!("======= HNX Microkernel Booting =======");
    println!("Kernel: {} {} ({})", KERNEL_NAME, VERSION, ARCH);
    
    // Initialize architecture-specific components (interrupts, timer, MMU)
    println!("Initializing architecture...");
    arch::init();
    println!("Architecture initialized");
}

/// Phase 2: Memory Management Initialization
///
/// Initializes the memory management subsystem:
/// - Physical memory allocator (buddy allocator)
/// - Virtual memory management (page tables)
/// - Slab allocator for small objects
/// - Memory mapping manager
fn init_phase2_memory() {
    let boot_info = crate::arch::boot::get_boot_info();
    
    println!("Initializing memory subsystem...");
    memory::init(boot_info);
    
    // Initialize kernel heap allocator
    println!("Initializing buddy allocator...");
    let heap_start = 0x40000000; // TODO: Get from device tree
    let heap_size = 0x10000000;  // TODO: Get from device tree
    unsafe {
        crate::memory::BUDDY_ALLOCATOR.init(heap_start, heap_size);
    }
    println!("Memory subsystem ready");
}

/// Phase 3: Process Management and IPC Initialization
///
/// Initializes process and IPC subsystems:
/// - Process Control Blocks (PCB)
/// - IPC endpoints and message queues
/// - Capability system (security)
/// - Well-known service endpoints for user space services
fn init_phase3_processes() {
    crate::info!("Initializing process subsystem...");
    process::init();
    
    println!("Initializing IPC service delegation...");
    if let Err(_) = ipc_services::delegate::init() {
        crate::warn!("Failed to initialize IPC service endpoints (services won't be available)");
    }
    
    let boot_info = crate::arch::boot::get_boot_info();
    println!("Initializing initrd accessor (dtb_ptr=0x{:X})...", boot_info.dtb_ptr);
    loader::init(boot_info.dtb_ptr as usize);
    
    println!("Process and IPC subsystems ready");
}

/// Phase 4: Start Scheduler
///
/// Enters the scheduler loop (never returns):
/// - Creates the idle task
/// - TODO: Launch the first user space process (init/procmgr)
/// - Begins round-robin scheduling
fn init_phase4_scheduler() -> ! {
    println!("Kernel core ready");
    
    println!("User space is not yet implemented until complete the kernel.");


    loop {
        crate::arch::cpu::wait_for_interrupt();
    }
    
    // println!("Attempting to bootstrap init process...");
    // match loader::bootstrap_init_process() {
    //     Ok((entry, sp, pt_base)) => {
    //         println!("Init process loaded successfully!");
    //         println!("  Entry: 0x{:X}", entry);
    //         println!("  Stack: 0x{:X}", sp);
    //         println!("  PT:    0x{:X}", pt_base);
            
    //         let pid = process::create_process(128).expect("Failed to create init process");
    //         process::update_process_memory(pid as usize, pt_base, 0);
            
    //         let mut task = process::Task::new_kernel(unsafe {
    //             ::core::mem::transmute::<usize, fn() -> !>(entry)
    //         });
            
    //         unsafe {
    //             task.ttbr0_base = pt_base;
    //             task.context.sp = sp;
    //             task.entry_point = entry;
    //             task.asid = pid as u16;
    //         }
            
    //         process::set_process_state(pid as usize, process::ProcState::Ready);
            
    //         println!("Init process created with PID {}", pid);
    //         println!("Starting scheduler - init will run at EL0...");
            
    //         crate::core::scheduler::run_task(task);
    //     }
    //     Err(_) => {
    //         crate::error!("Failed to load init process!");
    //         crate::error!("System cannot boot without init.");
    //         crate::error!("Ensure initrd contains a valid 'init' ELF binary.");
            
    //         loop {
    //             crate::arch::cpu::wait_for_interrupt();
    //         }
    //     }
    // }
}

/// Parse boot command line parameters (currently unused)
#[allow(dead_code)]
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
    }
}

/// Print CPU state for debugging (currently unused)
#[allow(dead_code)]
fn print_cpu_state() {
    // VBAR_EL1
    let mut vbar: u64;
    unsafe {
        ::core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
    }
    crate::debug!("VBAR_EL1 = 0x{:016X}", vbar);
    
    // CurrentEL
    let mut cur_el: u64;
    unsafe {
        ::core::arch::asm!("mrs {c}, CurrentEL", c = out(reg) cur_el);
    }
    crate::debug!("CurrentEL = 0x{:016X}", cur_el);
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    rust_main()
}
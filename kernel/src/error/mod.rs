//! Enhanced error handling and panic management
//!
//! This module provides improved error handling mechanisms and panic management
//! for the HNX kernel.

use core::fmt::Write;

/// Custom error type for kernel operations
#[derive(Debug)]
pub enum KernelError {
    /// Out of memory error
    OutOfMemory,
    /// Invalid argument error
    InvalidArgument,
    /// Permission denied error
    PermissionDenied,
    /// Resource unavailable error
    ResourceUnavailable,
    /// Internal error with description
    InternalError(&'static str),
}

impl KernelError {
    /// Converts the error to a numeric error code
    pub fn to_error_code(&self) -> isize {
        match self {
            KernelError::OutOfMemory => -1,
            KernelError::InvalidArgument => -2,
            KernelError::PermissionDenied => -3,
            KernelError::ResourceUnavailable => -4,
            KernelError::InternalError(_) => -5,
        }
    }
}

impl core::fmt::Display for KernelError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            KernelError::OutOfMemory => write!(f, "Out of memory"),
            KernelError::InvalidArgument => write!(f, "Invalid argument"),
            KernelError::PermissionDenied => write!(f, "Permission denied"),
            KernelError::ResourceUnavailable => write!(f, "Resource unavailable"),
            KernelError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Enhanced panic handler with detailed diagnostic information
pub fn panic_handler(
    info: &core::panic::PanicInfo,
    file: &'static str,
    line: u32,
    column: u32,
) -> ! {
    // Print panic information
    crate::console::write(format_args!("\nKERNEL PANIC at {}:{}:{}\n", file, line, column));
    crate::console::write(format_args!("Message: {}\n", info));
    
    // Dump CPU state if available
    crate::arch::dump_panic_state();
    
    // Print stack trace if available
    // In a real implementation, this would involve unwinding the stack
    // and printing function addresses or symbols
    
    // Halt the system
    loop {
        crate::arch::cpu::wait_for_interrupt();
    }
}

// These macros are now defined in console.rs to avoid conflicts
// #[macro_export]
// macro_rules! error {
//     ($($arg:tt)*) => ({
//         $crate::console::log("ERROR", module_path!(), format_args!($($arg)*));
//     });
// }
//
// #[macro_export]
// macro_rules! warn {
//     ($($arg:tt)*) => ({
//         $crate::console::log("WARN", module_path!(), format_args!($($arg)*));
//     });
// }
//
// #[macro_export]
// macro_rules! info {
//     ($($arg:tt)*) => ({
//         $crate::console::log("INFO", module_path!(), format_args!($($arg)*));
//     });
// }
//
// #[macro_export]
// macro_rules! debug {
//     ($($arg:tt)*) => ({
//         // Debug messages are only enabled in debug builds
//         #[cfg(debug_assertions)]
//         $crate::console::log("DEBUG", module_path!(), format_args!($($arg)*));
//     });
// }
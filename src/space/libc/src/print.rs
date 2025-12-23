//! Print utilities for HNX user space
//! Provides logging functionality for user space services

use core::fmt;

// Simple write function that sends data to a console device
pub fn _print(args: fmt::Arguments) {
    // For now, we'll just pass the string to a system call that can handle output
    // This is a placeholder implementation
    let _ = args; // Suppress unused variable warning
    // In a real implementation, this would make a system call to output the string
    // For example: sys_write(1, s.as_ptr(), s.len())
}

/// Print macro for user space services
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::_print(format_args!($($arg)*));
    };
}

/// Print line macro for user space services
#[macro_export]
macro_rules! println {
    () => ($crate::print!(""));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
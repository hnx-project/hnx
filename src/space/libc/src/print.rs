//! Print utilities for HNX user space
//! Provides logging functionality for user space services

use core::fmt;
use crate::syscalls;

// Simple write function that sends data to a console device
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    // Create a simple string writer
    struct StringWriter {
        buffer: [u8; 256],
        pos: usize,
    }

    impl Write for StringWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buffer.len() - self.pos;
            let to_copy = bytes.len().min(remaining);

            self.buffer[self.pos..self.pos + to_copy].copy_from_slice(&bytes[..to_copy]);
            self.pos += to_copy;

            // Flush if buffer is full or if we can't copy all
            if to_copy < bytes.len() || self.pos >= self.buffer.len() - 64 {
                self.flush();
            }

            Ok(())
        }
    }

    impl StringWriter {
        fn new() -> Self {
            Self {
                buffer: [0u8; 256],
                pos: 0,
            }
        }

        fn flush(&mut self) {
            if self.pos > 0 {
                unsafe {
                    syscalls::sys_write(1, self.buffer.as_ptr(), self.pos);
                }
                self.pos = 0;
            }
        }
    }

    let mut writer = StringWriter::new();
    let _ = writer.write_fmt(args);
    writer.flush();
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
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
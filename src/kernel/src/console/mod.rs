//! 内核控制台模块

use crate::drivers::uart::r#trait::UartDriver;
use core::fmt;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;

// Re-export the macros from the error module
// pub use crate::debug;
// pub use crate::error;
// pub use crate::info;
// pub use crate::warn;

lazy_static! {
    static ref CONSOLE_LOCK: Mutex<()> = Mutex::new(());
}
static mut LAST_CHAR: u8 = 0;
static mut REPEAT_COUNT: u32 = 0;
static mut TOTAL_COUNT: u32 = 0;
static FALLBACK_ENABLED: AtomicBool = AtomicBool::new(true);
 

#[inline]
fn irq_save_disable() -> u64 {
    0
}

#[inline]
fn irq_restore(_saved: u64) {}

/// 初始化控制台
pub fn init() {
    crate::arch::console::init();
    crate::drivers::uart::default().init();
}

pub fn driver_ready() {
    FALLBACK_ENABLED.store(false, Ordering::Relaxed);
}

#[inline]
fn uart_write_byte_ascii(c: u8) {
    unsafe {
        // Increase limit significantly to prevent silent output dropping
        if TOTAL_COUNT > 1000000 { 
            // Reset counter instead of stopping output
            TOTAL_COUNT = 0;
        }
        if LAST_CHAR == c {
            REPEAT_COUNT = REPEAT_COUNT.saturating_add(1);
            if REPEAT_COUNT > 64 {
                return;
            }
        } else {
            LAST_CHAR = c;
            REPEAT_COUNT = 1;
        }
        TOTAL_COUNT = TOTAL_COUNT.saturating_add(1);
        if FALLBACK_ENABLED.load(Ordering::Relaxed) {
            crate::arch::console::putc(c);
        } else {
            crate::drivers::uart::default().putc(c);
        }
    }
}

/// 读取一个字符（当前未实现通用输入，返回 None）
pub fn getc() -> Option<u8> {
    crate::drivers::uart::default().getc()
}

/// 内核打印宏
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::write(format_args!($($arg)*));
    };
}

/// 内核打印宏（带换行）
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::console::write(format_args!("{}\n", format_args!($($arg)*)));
    };
}

pub fn write(args: fmt::Arguments) {
    use core::fmt::Write;
    let _lock = CONSOLE_LOCK.lock();
    struct ConsoleWriter;
    impl fmt::Write for ConsoleWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for c in s.bytes() {
                uart_write_byte_ascii(c);
            }
            Ok(())
        }
    }
    let mut writer = ConsoleWriter;
    let _ = writer.write_fmt(args);
}

pub fn write_raw(s: &str) {
    let _lock = CONSOLE_LOCK.lock();
    for b in s.bytes() {
        uart_write_byte_ascii(b);
    }
}

#[macro_export]
macro_rules! print_raw {
    ($s:expr) => {
        $crate::console::write_raw($s)
    };
}

#[macro_export]
macro_rules! println_raw {
    ($s:expr) => {
        $crate::console::write_raw(concat!($s, "\n"))
    };
}

pub fn log(level: &str, module: &str, args: fmt::Arguments) {
    // if level == "ERROR" || level == "WARN" {
    write(format_args!("[{}] <{}> => {}\n", level, module, args));
        // return;
    // }
    // write(format_args!("[{}] => {}\n", level, args));
}

pub mod loglvl {
    use core::sync::atomic::{AtomicU8, Ordering};
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum LogLevel {
        Trace = 0,
        Debug = 1,
        Info = 2,
        Warn = 3,
        Error = 4,
    }
    static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Debug as u8);
    pub fn set_log_level(level: LogLevel) {
        LOG_LEVEL.store(level as u8, Ordering::Relaxed)
    }
    pub fn enabled(level: LogLevel) -> bool {
        (level as u8) >= LOG_LEVEL.load(Ordering::Relaxed)
    }
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        if $crate::console::loglvl::enabled($crate::console::loglvl::LogLevel::Trace) {
            $crate::console::log("TRACE", module_path!(), format_args!($($arg)*));
        }
    }}
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        if $crate::console::loglvl::enabled($crate::console::loglvl::LogLevel::Debug) {
            $crate::console::log("DEBUG", module_path!(), format_args!($($arg)*));
        }
    }}
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        if $crate::console::loglvl::enabled($crate::console::loglvl::LogLevel::Info) {
            $crate::console::log("INFO", module_path!(), format_args!($($arg)*));
        }
    }}
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        if $crate::console::loglvl::enabled($crate::console::loglvl::LogLevel::Warn) {
            $crate::console::log("WARN", module_path!(), format_args!($($arg)*));
        }
    }}
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        if $crate::console::loglvl::enabled($crate::console::loglvl::LogLevel::Error) {
            $crate::console::log("ERROR", module_path!(), format_args!($($arg)*));
        }
    }}
}
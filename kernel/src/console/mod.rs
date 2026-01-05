//! 内核控制台模块

use crate::drivers::uart::r#trait::UartDriver;
use core::fmt;
use shared::sync::mutex::Mutex;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};


// Re-export the macros from the error module
// pub use crate::debug;
// pub use crate::error;
// pub use crate::info;
// pub use crate::warn;

/// 控制台管理器
///
/// 管理控制台输出状态，包括字符重复检测、回退模式等
pub struct ConsoleManager {
    /// 控制台互斥锁
    console_lock: Mutex<()>,
    /// 上一个字符
    last_char: AtomicU8,
    /// 重复计数
    repeat_count: AtomicU32,
    /// 总输出计数
    total_count: AtomicU32,
    /// 回退使能标志（true = 使用 arch 控制台，false = 使用 UART 驱动程序）
    fallback_enabled: AtomicBool,
}

// 全局控制台管理器实例（临时，迁移期间使用）
static CONSOLE_MANAGER: ConsoleManager = ConsoleManager::new();

impl ConsoleManager {
    /// 创建新的控制台管理器
    pub const fn new() -> Self {
        Self {
            console_lock: Mutex::new(()),
            last_char: AtomicU8::new(0),
            repeat_count: AtomicU32::new(0),
            total_count: AtomicU32::new(0),
            fallback_enabled: AtomicBool::new(true),
        }
    }

    /// 初始化控制台
    pub fn init(&self) {
        crate::arch::console::init();
        crate::drivers::uart::default().init();
    }

    /// 标记驱动程序就绪（切换到 UART 驱动程序）
    pub fn driver_ready(&self) {
        self.fallback_enabled.store(false, Ordering::Relaxed);
    }

    /// 写入单个字符（内部方法）
    fn uart_write_byte_ascii(&self, c: u8) {
        // 增加限制以防止静默输出丢失
        let total_count = self.total_count.load(Ordering::Relaxed);
        if total_count > 1000000 {
            // 重置计数器而不是停止输出
            self.total_count.store(0, Ordering::Relaxed);
        }
        let last_char = self.last_char.load(Ordering::Relaxed);
        if last_char == c {
            let repeat_count = self.repeat_count.load(Ordering::Relaxed);
            let new_repeat_count = repeat_count.saturating_add(1);
            self.repeat_count.store(new_repeat_count, Ordering::Relaxed);
            if new_repeat_count > 64 {
                return;
            }
        } else {
            self.last_char.store(c, Ordering::Relaxed);
            self.repeat_count.store(1, Ordering::Relaxed);
        }
        self.total_count.fetch_add(1, Ordering::Relaxed);
        if self.fallback_enabled.load(Ordering::Relaxed) {
            crate::arch::console::putc(c);
        } else {
            crate::drivers::uart::default().putc(c);
        }
    }

    /// 写入格式化参数
    pub fn write(&self, args: fmt::Arguments) {
        use core::fmt::Write;
        let _lock = self.console_lock.lock();
        struct ConsoleWriter<'a> {
            manager: &'a ConsoleManager,
        }
        impl<'a> fmt::Write for ConsoleWriter<'a> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                for c in s.bytes() {
                    self.manager.uart_write_byte_ascii(c);
                }
                Ok(())
            }
        }
        let mut writer = ConsoleWriter { manager: self };
        let _ = writer.write_fmt(args);
    }

    /// 写入原始字符串
    pub fn write_raw(&self, s: &str) {
        let _lock = self.console_lock.lock();
        for b in s.bytes() {
            self.uart_write_byte_ascii(b);
        }
    }

    /// 记录日志
    pub fn log(&self, level: &str, module: &str, args: fmt::Arguments) {
        self.write(format_args!("[{}] <{}> => {}\n", level, module, args));
    }

    /// 获取一个字符（当前未实现通用输入，返回 None）
    pub fn getc(&self) -> Option<u8> {
        crate::drivers::uart::default().getc()
    }
}



/// 初始化控制台
pub fn init() {
    CONSOLE_MANAGER.init();
}

pub fn driver_ready() {
    CONSOLE_MANAGER.driver_ready();
}


/// 读取一个字符（当前未实现通用输入，返回 None）
pub fn getc() -> Option<u8> {
    CONSOLE_MANAGER.getc()
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
    CONSOLE_MANAGER.write(args);
}

pub fn write_raw(s: &str) {
    CONSOLE_MANAGER.write_raw(s);
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
    CONSOLE_MANAGER.log(level, module, args);
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
    static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Warn as u8);
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

#[macro_export]
macro_rules! test {
    ($($arg:tt)*) => {{
        // test!宏总是输出，不受日志级别控制，格式为[TEST] <消息>
        $crate::console::log("[TEST]", module_path!(), format_args!($($arg)*));
    }}
}
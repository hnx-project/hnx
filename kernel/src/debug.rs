//! 内核调试输出模块
//!
//! 提供早期调试输出功能，在控制台系统初始化之前使用。
//! 当前实现针对AArch64 QEMU virt机器的PL011 UART。

use core::fmt;
use core::fmt::Write;

use shared::sync::mutex::Mutex;

/// PL011 UART寄存器定义
mod pl011 {
    /// UART数据寄存器（读写）
    pub const DR: usize = 0x00;
    /// UART标志寄存器（只读）
    pub const FR: usize = 0x18;
    /// UART控制寄存器
    pub const CR: usize = 0x30;

    /// 标志寄存器位定义
    pub mod fr {
        /// 发送FIFO满
        pub const TXFF: u32 = 1 << 5;
        /// 发送FIFO空
        pub const TXFE: u32 = 1 << 7;
    }
}

/// PL011 UART驱动程序
struct Uart {
    base: usize,
}

impl Uart {
    /// 创建UART实例
    const fn new(base: usize) -> Self {
        Self { base }
    }

    /// 初始化UART
    unsafe fn init(&mut self) {
        // 对于QEMU virt机器，UART通常已经由固件初始化
        // 这里我们只确保UART处于启用状态
        let cr = self.read_reg(pl011::CR);
        if cr == 0 {
            // 如果控制寄存器为0，则启用UART
            // 启用发送和接收
            self.write_reg(pl011::CR, 0x301); // 启用UART，启用TX和RX
        }
    }

    /// 读取寄存器
    unsafe fn read_reg(&self, reg: usize) -> u32 {
        let addr = self.base + reg;
        (addr as *const u32).read_volatile()
    }

    /// 写入寄存器
    unsafe fn write_reg(&self, reg: usize, value: u32) {
        let addr = self.base + reg;
        (addr as *mut u32).write_volatile(value)
    }

    /// 检查是否可以发送字符
    fn is_transmit_ready(&self) -> bool {
        unsafe {
            let fr = self.read_reg(pl011::FR);
            // 如果TXFF位为0（发送FIFO未满），则可以发送
            (fr & pl011::fr::TXFF) == 0
        }
    }

    /// 发送一个字符
    fn write_char(&mut self, c: u8) {
        // 等待发送就绪
        while !self.is_transmit_ready() {}

        // 发送字符
        unsafe {
            self.write_reg(pl011::DR, c as u32);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.write_char(c);
        }
        Ok(())
    }
}

/// 全局UART实例
static UART: Mutex<Option<Uart>> = Mutex::new(None);

/// 初始化调试输出
///
/// # 安全性
/// 此函数必须在单线程环境下调用，且只能调用一次。
pub unsafe fn init() {
    let mut uart_opt = UART.lock();
    if uart_opt.is_none() {
        // QEMU virt机器的PL011 UART地址
        const UART_BASE: usize = 0x0900_0000;
        let mut uart = Uart::new(UART_BASE);
        uart.init();
        *uart_opt = Some(uart);
    }
}

/// 打印字符串（原始接口）
pub fn print_str(s: &str) {
    let mut uart_opt = UART.lock();
    if let Some(uart) = uart_opt.as_mut() {
        let _ = uart.write_str(s);
    }
}

/// 打印单个字符
pub fn putc(c: u8) {
    let mut uart_opt = UART.lock();
    if let Some(uart) = uart_opt.as_mut() {
        uart.write_char(c);
    }
}

/// 打印格式化字符串
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::debug::_print(format_args!($($arg)*));
    };
}

/// 打印格式化字符串并换行
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}

/// 内部函数：打印格式化参数
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;

    let mut uart_opt = UART.lock();
    if let Some(uart) = uart_opt.as_mut() {
        let _ = uart.write_fmt(args);
    }
}

/// 早期调试输出（在全局分配器初始化之前使用）
///
/// 此函数直接操作UART，不依赖任何内存分配。
pub fn early_print(s: &str) {
    const UART_BASE: usize = 0x0900_0000;

    for c in s.bytes() {
        // 等待发送就绪
        unsafe {
            while (UART_BASE as *const u32).add(pl011::FR / 4).read_volatile() & pl011::fr::TXFF
                != 0
            {}
            // 发送字符
            (UART_BASE as *mut u32).write_volatile(c as u32);
        }
    }
}

/// 早期调试输出宏（不依赖全局分配器）
#[macro_export]
macro_rules! early_println {
    ($($arg:tt)*) => {
        $crate::debug::_early_print(format_args_nl!($($arg)*));
    };
}

pub fn _early_print(args: core::fmt::Arguments) {
    struct EarlyWriter;

    impl core::fmt::Write for EarlyWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            crate::debug::early_print(s);
            Ok(())
        }
    }

    let mut w = EarlyWriter;
    let _ = w.write_fmt(args);
}

/// 测试调试输出
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_print() {
        // 注意：此测试只能在QEMU环境中运行
        early_print("[TEST] Testing early debug output\n");
    }
}

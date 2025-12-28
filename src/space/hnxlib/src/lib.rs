#![no_std]

// 安全系统调用模块
pub mod safe_syscall;

// 内存屏障模块
pub mod barrier;

// 传统系统调用模块（向后兼容）
pub mod syscall {
    use core::arch::asm;
    use hnx_abi::{HNX_SYS_WRITE, HNX_SYS_YIELD, HNX_SYS_SPAWN_SERVICE};

    // 重新导出安全系统调用作为主要实现
    pub use crate::safe_syscall::{
        write,
        debug_print,
        yield_cpu,
        spawn_service,
    };

    // 旧的实现保留作为备份
    #[doc(hidden)]
    #[inline(always)]
    pub fn write_old(fd: i32, buf: &[u8]) -> isize {
        let ret: isize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") HNX_SYS_WRITE,
                in("x0") fd as usize,
                in("x1") buf.as_ptr() as usize,
                in("x2") buf.len(),
                lateout("x0") ret,
                clobber_abi("C"),
                options(nostack)
            );
        }
        ret
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn debug_print_old(s: &str) {
        let _ret: isize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") HNX_SYS_WRITE,
                in("x0") 1usize,   // fd=1 (stdout)
                in("x1") s.as_ptr() as usize,
                in("x2") s.len(),
                lateout("x0") _ret,
                clobber_abi("C"),
                options(nostack)
            );
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn yield_cpu_old() {
        let _ret: isize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") HNX_SYS_YIELD,
                lateout("x0") _ret,
                clobber_abi("C"),
                options(nostack)
            );
        }
    }

    #[doc(hidden)]
    #[inline(never)]
    pub fn spawn_service_old(path: &str) -> isize {
        let ret: isize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") HNX_SYS_SPAWN_SERVICE,
                in("x0") path.as_ptr() as usize,
                in("x1") path.len(),
                lateout("x0") ret,
                clobber_abi("C"),
                options(nostack, preserves_flags)
            );
        }
        ret
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::DebugWriter;
        let _ = core::fmt::write(&mut writer, format_args!($($arg)*));
        writer.flush();
    }};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::DebugWriter;
        let _ = core::fmt::write(&mut writer, format_args!($($arg)*));
    }};
}

pub struct DebugWriter;
impl core::fmt::Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        syscall::debug_print(s);
        Ok(())
    }
}

impl DebugWriter {
    pub fn flush(&mut self) {
        // No buffering in debug output
    }
}

// Basic C-compatible functions
#[no_mangle]
pub unsafe extern "C" fn write(fd: i32, buf: *const u8, count: usize) -> isize {
    if buf.is_null() {
        return -1;
    }
    let slice = core::slice::from_raw_parts(buf, count);
    syscall::write(fd, slice)
}

#[no_mangle]
pub unsafe extern "C" fn debug_print_str(s: *const u8, len: usize) {
    if s.is_null() {
        return;
    }
    let slice = core::slice::from_raw_parts(s, len);
    let s_str = core::str::from_utf8(slice).unwrap_or("<invalid-utf8>");
    syscall::debug_print(s_str);
}
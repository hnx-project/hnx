#![no_std]

pub mod syscall {
    use core::arch::asm;

    #[inline(always)]
    pub fn write(fd: i32, buf: &[u8]) -> isize {
        let ret: isize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") 0x1001,  // HNX_SYS_WRITE
                in("x0") fd as usize,
                in("x1") buf.as_ptr() as usize,
                in("x2") buf.len(),
                lateout("x0") ret,
                options(nostack)
            );
        }
        ret
    }

    #[inline(always)]
    pub fn debug_print(s: &str) {
        unsafe {
            asm!(
                "mov x8, #0",
                "svc #0",
                in("x0") s.as_ptr() as usize,
                in("x1") s.len(),
                out("x8") _,  // x8 is modified by the instruction
            );
        }
    }

    #[inline(always)]
    pub fn yield_cpu() {
        unsafe {
            asm!(
                "mov x8, #0x1007",  // HNX_SYS_YIELD
                "svc #0",
                options(nostack)
            );
        }
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::DebugWriter;
        write!(&mut writer, $($arg)*).unwrap();
        writer.flush();
    }};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::DebugWriter;
        write!(&mut writer, $($arg)*).unwrap();
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
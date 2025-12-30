//! ARM AArch64 用户空间架构实现
//!
//! 实现AArch64架构特定的用户空间操作：
//! - 系统调用指令: `svc #0`
//! - TLS寄存器: TPIDR_EL0
//! - 栈对齐: 16字节
//! - 原子操作: LDXR/STXR指令

use core::arch::asm;
use crate::arch::common::traits::*;

/// ARM AArch64 架构实现
pub struct AArch64;

impl UserArch for AArch64 {
    const ARCH_NAME: &'static str = "aarch64";
    const STACK_ALIGNMENT: usize = 16;
    const TLS_ALIGNMENT: usize = 8;
    const SYSCALL_INSTRUCTION: &'static str = "svc #0";
}

impl UserSyscall for AArch64 {
    #[inline(always)]
    fn syscall0(num: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
    
    #[inline(always)]
    fn syscall1(num: usize, arg1: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                in("x0") arg1,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
    
    #[inline(always)]
    fn syscall2(num: usize, arg1: usize, arg2: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                in("x0") arg1,
                in("x1") arg2,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
    
    #[inline(always)]
    fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                in("x0") arg1,
                in("x1") arg2,
                in("x2") arg3,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
    
    #[inline(always)]
    fn syscall4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                in("x0") arg1,
                in("x1") arg2,
                in("x2") arg3,
                in("x3") arg4,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
    
    #[inline(always)]
    fn syscall5(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                in("x0") arg1,
                in("x1") arg2,
                in("x2") arg3,
                in("x3") arg4,
                in("x4") arg5,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
    
    #[inline(always)]
    fn syscall6(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> usize {
        let ret: usize;
        unsafe {
            asm!(
                "svc #0",
                in("x8") num,
                in("x0") arg1,
                in("x1") arg2,
                in("x2") arg3,
                in("x3") arg4,
                in("x4") arg5,
                in("x5") arg6,
                lateout("x0") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
}

impl UserTls for AArch64 {
    type TlsBlock = [u8; 64]; // 示例大小，实际由运行时确定
    
    fn get_tls() -> *mut Self::TlsBlock {
        let tp = Self::get_thread_pointer();
        tp as *mut Self::TlsBlock
    }
    
    fn set_tls(ptr: *mut Self::TlsBlock) {
        let tp = ptr as usize;
        unsafe {
            asm!("msr TPIDR_EL0, {}", in(reg) tp);
        }
    }
    
    fn get_thread_pointer() -> usize {
        let tp: usize;
        unsafe {
            asm!("mrs {}, TPIDR_EL0", out(reg) tp);
        }
        tp
    }
}

impl UserContext for AArch64 {
    type Context = [usize; 32]; // 简化上下文存储
    
    fn make_context(stack: *mut u8, func: extern fn(usize), arg: usize) -> Self::Context {
        // 简化的上下文创建
        // 实际实现需要设置栈帧和寄存器
        let mut ctx = [0usize; 32];
        
        // 设置返回地址（函数入口）
        ctx[0] = func as usize;
        
        // 设置参数寄存器
        ctx[1] = arg;
        
        // 设置栈指针
        ctx[2] = stack as usize;
        
        ctx
    }
    
    fn swap_context(old: *mut Self::Context, new: *const Self::Context) {
        // 简化的上下文切换
        // 实际实现需要保存/恢复所有调用者保存寄存器
        unsafe {
            asm!(
                "
                // 保存当前上下文（简化版）
                stp x19, x20, [x0, #0]
                stp x21, x22, [x0, #16]
                stp x23, x24, [x0, #32]
                stp x25, x26, [x0, #48]
                stp x27, x28, [x0, #64]
                stp x29, x30, [x0, #80]
                mov x2, sp
                str x2, [x0, #96]
                
                // 恢复新上下文
                ldp x19, x20, [x1, #0]
                ldp x21, x22, [x1, #16]
                ldp x23, x24, [x1, #32]
                ldp x25, x26, [x1, #48]
                ldp x27, x28, [x1, #64]
                ldp x29, x30, [x1, #80]
                ldr x2, [x1, #96]
                mov sp, x2
                
                // 返回到新上下文的入口点
                ret
                ",
                in("x0") old,
                in("x1") new,
                options(nostack)
            );
        }
    }
}

impl UserAtomic for AArch64 {
    fn compare_exchange(ptr: *mut usize, expected: usize, desired: usize) -> Result<usize, usize> {
        let result: usize;
        unsafe {
            asm!(
                "
                mov x3, {expected}
                mov x4, {desired}
                ldaxr x5, [{ptr}]
                cmp x5, x3
                b.ne 1f
                stlxr w6, x4, [{ptr}]
                cbnz w6, 2f
                mov {result}, x3
                b 3f
                1:
                mov {result}, x5
                b 3f
                2:
                mov {result}, x5
                3:
                ",
                ptr = in(reg) ptr,
                expected = in(reg) expected,
                desired = in(reg) desired,
                result = out(reg) result,
                out("x3") _,
                out("x4") _,
                out("x5") _,
                out("w6") _,
                options(nostack, preserves_flags)
            );
        }
        
        if result == expected {
            Ok(result)
        } else {
            Err(result)
        }
    }
    
    fn fetch_add(ptr: *mut usize, val: usize) -> usize {
        let result: usize;
        unsafe {
            asm!(
                "
                mov x2, {val}
                1:
                ldaxr x3, [{ptr}]
                add x4, x3, x2
                stlxr w5, x4, [{ptr}]
                cbnz w5, 1b
                mov {result}, x3
                ",
                ptr = in(reg) ptr,
                val = in(reg) val,
                result = out(reg) result,
                out("x2") _,
                out("x3") _,
                out("x4") _,
                out("w5") _,
                options(nostack, preserves_flags)
            );
        }
        result
    }
    
    fn load_acquire(ptr: *const usize) -> usize {
        let result: usize;
        unsafe {
            asm!(
                "ldar {result}, [{ptr}]",
                ptr = in(reg) ptr,
                result = out(reg) result,
                options(nostack, preserves_flags)
            );
        }
        result
    }
    
    fn store_release(ptr: *mut usize, val: usize) {
        unsafe {
            asm!(
                "stlr {val}, [{ptr}]",
                ptr = in(reg) ptr,
                val = in(reg) val,
                options(nostack, preserves_flags)
            );
        }
    }
}

/// AArch64特定的扩展功能
impl AArch64 {
    /// 获取当前异常级别
    pub fn get_current_el() -> u8 {
        let el: u64;
        unsafe {
            asm!("mrs {}, CurrentEL", out(reg) el);
        }
        ((el >> 2) & 0x3) as u8
    }
    
    /// 获取向量浮点/NEON寄存器
    pub fn get_fpsr() -> u32 {
        let fpsr: u32;
        unsafe {
            asm!("mrs {}, FPSR", out(reg) fpsr);
        }
        fpsr
    }
    
    /// 设置向量浮点/NEON寄存器
    pub fn set_fpsr(fpsr: u32) {
        unsafe {
            asm!("msr FPSR, {}", in(reg) fpsr);
        }
    }
}
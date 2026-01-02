//! RISC-V 64位 用户空间架构实现（占位符）
//!
//! 目前仅提供占位符实现，实际支持需要后续开发。

use crate::arch::common::traits::*;

/// RISC-V 64位 架构实现（占位符）
pub struct RiscV64;

impl UserArch for RiscV64 {
    const ARCH_NAME: &'static str = "riscv64";
    const STACK_ALIGNMENT: usize = 16;
    const TLS_ALIGNMENT: usize = 8;
    const SYSCALL_INSTRUCTION: &'static str = "ecall";
}

impl UserSyscall for RiscV64 {
    #[inline(always)]
    fn syscall0(num: usize) -> usize {
        unimplemented!("RISC-V syscall0 not implemented")
    }
    
    #[inline(always)]
    fn syscall1(num: usize, arg1: usize) -> usize {
        unimplemented!("RISC-V syscall1 not implemented")
    }
    
    #[inline(always)]
    fn syscall2(num: usize, arg1: usize, arg2: usize) -> usize {
        unimplemented!("RISC-V syscall2 not implemented")
    }
    
    #[inline(always)]
    fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
        unimplemented!("RISC-V syscall3 not implemented")
    }
    
    #[inline(always)]
    fn syscall4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> usize {
        unimplemented!("RISC-V syscall4 not implemented")
    }
    
    #[inline(always)]
    fn syscall5(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
        unimplemented!("RISC-V syscall5 not implemented")
    }
    
    #[inline(always)]
    fn syscall6(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> usize {
        unimplemented!("RISC-V syscall6 not implemented")
    }
}

impl UserTls for RiscV64 {
    type TlsBlock = [u8; 64];
    
    fn get_tls() -> *mut Self::TlsBlock {
        unimplemented!("RISC-V TLS not implemented")
    }
    
    fn set_tls(_ptr: *mut Self::TlsBlock) {
        unimplemented!("RISC-V TLS not implemented")
    }
    
    fn get_thread_pointer() -> usize {
        unimplemented!("RISC-V thread pointer not implemented")
    }
}

impl UserContext for RiscV64 {
    type Context = [usize; 32];
    
    fn make_context(_stack: *mut u8, _func: extern fn(usize), _arg: usize) -> Self::Context {
        unimplemented!("RISC-V context not implemented")
    }
    
    fn swap_context(_old: *mut Self::Context, _new: *const Self::Context) {
        unimplemented!("RISC-V context switch not implemented")
    }
}

impl UserAtomic for RiscV64 {
    fn compare_exchange(_ptr: *mut usize, _expected: usize, _desired: usize) -> Result<usize, usize> {
        unimplemented!("RISC-V atomic not implemented")
    }
    
    fn fetch_add(_ptr: *mut usize, _val: usize) -> usize {
        unimplemented!("RISC-V atomic not implemented")
    }
    
    fn load_acquire(_ptr: *const usize) -> usize {
        unimplemented!("RISC-V atomic not implemented")
    }
    
    fn store_release(_ptr: *mut usize, _val: usize) {
        unimplemented!("RISC-V atomic not implemented")
    }
}
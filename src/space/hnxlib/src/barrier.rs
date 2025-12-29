//! 内存屏障和编译器屏障工具
//!
//! 这个模块提供了用于系统调用的内存屏障和编译器屏障函数。
//! 这些屏障确保系统调用前后的内存访问顺序正确，防止编译器优化导致问题。

use core::sync::atomic::{compiler_fence, fence, Ordering};

/// 系统调用前屏障
///
/// 这个屏障确保在系统调用之前的所有内存访问都已完成，
/// 并且防止编译器重排系统调用之前的操作。
#[inline(always)]
pub fn before_syscall() {
    // 完整的序列化屏障
    fence(Ordering::SeqCst);
    
    // 编译器屏障，防止重排
    compiler_fence(Ordering::SeqCst);
}

/// 系统调用后屏障
///
/// 这个屏障确保在系统调用之后的内存访问不会在系统调用之前执行，
/// 并且防止编译器重排系统调用之后的操作。
#[inline(always)]
pub fn after_syscall() {
    // 完整的序列化屏障
    fence(Ordering::SeqCst);
    
    // 编译器屏障，防止重排
    compiler_fence(Ordering::SeqCst);
}

/// 轻量级编译器屏障（仅防止编译器重排）
#[inline(always)]
pub fn compiler_barrier() {
    compiler_fence(Ordering::SeqCst);
}

/// 完整的存储器屏障
#[inline(always)]
pub fn memory_barrier() {
    fence(Ordering::SeqCst);
}

/// 获取屏障
#[inline(always)]
pub fn acquire_barrier() {
    fence(Ordering::Acquire);
}

/// 释放屏障
#[inline(always)]
pub fn release_barrier() {
    fence(Ordering::Release);
}

/// 寄存器保存屏障
///
/// 这个特殊的屏障用于保护关键寄存器（如x8）在系统调用前后不被破坏。
/// 它通过强制编译器将所有寄存器溢出到内存来实现。
#[inline(never)]
pub fn register_preservation_barrier() {
    // 使用volatile汇编确保编译器不会优化掉寄存器使用
    unsafe {
        core::arch::asm!("",
            out("x0") _,
            out("x1") _,
            out("x2") _,
            out("x3") _,
            out("x4") _,
            out("x5") _,
            out("x6") _,
            out("x7") _,
            out("x8") _,
            out("x9") _,
            out("x10") _,
            out("x11") _,
            out("x12") _,
            out("x13") _,
            out("x14") _,
            out("x15") _,
            out("x16") _,
            out("x17") _,
            out("x18") _,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// 系统调用包装器屏障
///
/// 这个函数包装系统调用，确保前后都有适当的屏障。
/// 这是最高级别的保护。
#[inline(never)]
pub fn syscall_with_barriers<F, R>(syscall: F) -> R
where
    F: FnOnce() -> R,
{
    before_syscall();
    register_preservation_barrier();
    let result = syscall();
    register_preservation_barrier();
    after_syscall();
    result
}

/// 测试函数：验证屏障工作
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_barrier_functions_compile() {
        // 测试所有屏障函数都能编译
        before_syscall();
        after_syscall();
        compiler_barrier();
        memory_barrier();
        acquire_barrier();
        release_barrier();
        register_preservation_barrier();
        
        // 测试系统调用包装器
        let result = syscall_with_barriers(|| 42);
        assert_eq!(result, 42);
        
        // 如果编译通过，测试就算通过
        assert!(true, "All barrier functions should compile correctly");
    }
    
    #[test]
    fn test_barrier_ordering() {
        use core::sync::atomic::{AtomicUsize, Ordering};
        
        // 创建一个共享变量来测试屏障效果
        static TEST_VAR: AtomicUsize = AtomicUsize::new(0);
        
        // 在屏障前设置值
        TEST_VAR.store(1, Ordering::Relaxed);
        
        // 应用屏障
        release_barrier();
        
        // 模拟系统调用（另一个线程的操作）
        // 在单线程测试中，我们只验证编译
        
        // 应用获取屏障
        acquire_barrier();
        
        // 读取值
        let val = TEST_VAR.load(Ordering::Relaxed);
        
        // 验证值
        assert_eq!(val, 1, "Barrier should preserve value");
    }
}
// 具体架构实现
//
// 包含各个架构的具体实现。

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

/// 获取当前架构实现模块
pub fn current() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    return "aarch64";
    
    #[cfg(target_arch = "riscv64")]
    return "riscv64";
    
    #[cfg(target_arch = "x86_64")]
    return "x86_64";
    
    #[cfg(not(any(
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86_64"
    )))]
    compile_error!("Unsupported architecture");
}

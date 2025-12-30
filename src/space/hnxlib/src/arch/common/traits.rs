//! 用户空间架构抽象接口定义
//!
//! 这些trait定义了用户空间需要处理的架构差异：
//! 1. 系统调用指令封装
//! 2. TLS（线程局部存储）操作
//! 3. 上下文切换（用于协程/纤程）
//! 4. 原子操作（用户空间版本）

/// 用户空间系统调用接口
///
/// 不同架构的系统调用指令和寄存器约定不同：
/// - ARM AArch64: `svc #0`，x8=系统调用号，x0-x5=参数，x0=返回值
/// - x86_64: `syscall`，rax=系统调用号，rdi/rsi/rdx/r10/r8/r9=参数，rax=返回值
/// - RISC-V: `ecall`，a7=系统调用号，a0-a5=参数，a0=返回值
pub trait UserSyscall {
    /// 执行0参数系统调用
    fn syscall0(num: usize) -> usize;
    
    /// 执行1参数系统调用
    fn syscall1(num: usize, arg1: usize) -> usize;
    
    /// 执行2参数系统调用
    fn syscall2(num: usize, arg1: usize, arg2: usize) -> usize;
    
    /// 执行3参数系统调用
    fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize;
    
    /// 执行4参数系统调用
    fn syscall4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> usize;
    
    /// 执行5参数系统调用
    fn syscall5(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize;
    
    /// 执行6参数系统调用
    fn syscall6(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> usize;
}

/// TLS（线程局部存储）操作
///
/// 不同架构的TLS实现方式不同：
/// - ARM AArch64: 使用TPIDR_EL0寄存器
/// - x86_64: 使用fs/gs段寄存器
/// - RISC-V: 使用tp寄存器
pub trait UserTls {
    /// TLS块类型（架构特定布局）
    type TlsBlock;
    
    /// 获取当前线程的TLS指针
    fn get_tls() -> *mut Self::TlsBlock;
    
    /// 设置当前线程的TLS指针
    fn set_tls(ptr: *mut Self::TlsBlock);
    
    /// 获取线程指针（原始值）
    fn get_thread_pointer() -> usize;
}

/// 用户空间上下文（用于协程/纤程）
///
/// 用户空间的上下文切换，不涉及特权模式切换。
/// 主要用于协程库和用户态调度器。
pub trait UserContext {
    /// 上下文结构类型
    type Context;
    
    /// 创建新的上下文
    ///
    /// # 参数
    /// - `stack`: 栈顶指针（高地址）
    /// - `func`: 入口函数
    /// - `arg`: 传递给函数的参数
    fn make_context(stack: *mut u8, func: extern fn(usize), arg: usize) -> Self::Context;
    
    /// 切换上下文
    ///
    /// # 参数
    /// - `old`: 保存当前上下文的位置
    /// - `new`: 要切换到的上下文
    fn swap_context(old: *mut Self::Context, new: *const Self::Context);
}

/// 原子操作（用户空间版本）
///
/// 用户空间的原子操作通常使用LL/SC（Load-Link/Store-Conditional）
/// 或对应的架构特定指令。
pub trait UserAtomic {
    /// 比较并交换
    fn compare_exchange(ptr: *mut usize, expected: usize, desired: usize) -> Result<usize, usize>;
    
    /// 原子加法
    fn fetch_add(ptr: *mut usize, val: usize) -> usize;
    
    /// 获取加载（带acquire语义）
    fn load_acquire(ptr: *const usize) -> usize;
    
    /// 释放存储（带release语义）
    fn store_release(ptr: *mut usize, val: usize);
}

/// 用户空间架构特征
///
/// 聚合所有用户空间架构相关的操作
pub trait UserArch: UserSyscall + UserTls + UserContext + UserAtomic {
    /// 架构名称
    const ARCH_NAME: &'static str;
    
    /// 栈对齐要求（字节）
    const STACK_ALIGNMENT: usize;
    
    /// TLS对齐要求（字节）
    const TLS_ALIGNMENT: usize;
    
    /// 系统调用指令名称（用于调试）
    const SYSCALL_INSTRUCTION: &'static str;
}

/// 默认实现占位符
///
/// 用于在没有特定架构实现时提供编译时错误
pub struct UnimplementedArch;

impl UserSyscall for UnimplementedArch {
    fn syscall0(_num: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
    
    fn syscall1(_num: usize, _arg1: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
    
    fn syscall2(_num: usize, _arg1: usize, _arg2: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
    
    fn syscall3(_num: usize, _arg1: usize, _arg2: usize, _arg3: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
    
    fn syscall4(_num: usize, _arg1: usize, _arg2: usize, _arg3: usize, _arg4: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
    
    fn syscall5(_num: usize, _arg1: usize, _arg2: usize, _arg3: usize, _arg4: usize, _arg5: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
    
    fn syscall6(_num: usize, _arg1: usize, _arg2: usize, _arg3: usize, _arg4: usize, _arg5: usize, _arg6: usize) -> usize {
        unimplemented!("UserSyscall not implemented for this architecture")
    }
}

impl UserTls for UnimplementedArch {
    type TlsBlock = ();
    
    fn get_tls() -> *mut Self::TlsBlock {
        unimplemented!("UserTls not implemented for this architecture")
    }
    
    fn set_tls(_ptr: *mut Self::TlsBlock) {
        unimplemented!("UserTls not implemented for this architecture")
    }
    
    fn get_thread_pointer() -> usize {
        unimplemented!("UserTls not implemented for this architecture")
    }
}

impl UserContext for UnimplementedArch {
    type Context = ();
    
    fn make_context(_stack: *mut u8, _func: extern fn(usize), _arg: usize) -> Self::Context {
        unimplemented!("UserContext not implemented for this architecture")
    }
    
    fn swap_context(_old: *mut Self::Context, _new: *const Self::Context) {
        unimplemented!("UserContext not implemented for this architecture")
    }
}

impl UserAtomic for UnimplementedArch {
    fn compare_exchange(_ptr: *mut usize, _expected: usize, _desired: usize) -> Result<usize, usize> {
        unimplemented!("UserAtomic not implemented for this architecture")
    }
    
    fn fetch_add(_ptr: *mut usize, _val: usize) -> usize {
        unimplemented!("UserAtomic not implemented for this architecture")
    }
    
    fn load_acquire(_ptr: *const usize) -> usize {
        unimplemented!("UserAtomic not implemented for this architecture")
    }
    
    fn store_release(_ptr: *mut usize, _val: usize) {
        unimplemented!("UserAtomic not implemented for this architecture")
    }
}

impl UserArch for UnimplementedArch {
    const ARCH_NAME: &'static str = "unimplemented";
    const STACK_ALIGNMENT: usize = 16;
    const TLS_ALIGNMENT: usize = 8;
    const SYSCALL_INSTRUCTION: &'static str = "unimplemented";
}
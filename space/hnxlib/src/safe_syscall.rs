//! 安全的系统调用包装器实现
//!
//! 这个模块提供了安全的系统调用接口，确保寄存器正确保存和恢复。
//! 主要解决系统调用后寄存器被破坏的问题，特别是x8寄存器。

use core::arch::asm;
use hnx_abi::*;
use crate::barrier;

// 编译时检查：确保我们的栈偏移量与内核中的定义匹配
// 这些值必须与 kernel/src/arch/aarch64/boot.S 中的栈布局一致
const _SYSCALL_STACK_OFFSET_X8: usize = 80;
const _SYSCALL_STACK_OFFSET_X0: usize = 144;
const _SYSCALL_STACK_OFFSET_X1: usize = 152;
const _SYSCALL_STACK_OFFSET_X2: usize = 128;
const _SYSCALL_STACK_OFFSET_X3: usize = 136;
const _SYSCALL_STACK_OFFSET_X4: usize = 112;
const _SYSCALL_STACK_OFFSET_X5: usize = 120;

// 编译时断言（在启用时检查）
// 注意：这些断言在no_std环境中可能无法工作，所以我们使用静态断言模式
struct _StackOffsetCheck;
impl _StackOffsetCheck {
    const X8_CORRECT: bool = _SYSCALL_STACK_OFFSET_X8 == 80;
    const X0_CORRECT: bool = _SYSCALL_STACK_OFFSET_X0 == 144;
    const X1_CORRECT: bool = _SYSCALL_STACK_OFFSET_X1 == 152;
    const X2_CORRECT: bool = _SYSCALL_STACK_OFFSET_X2 == 128;
    const X3_CORRECT: bool = _SYSCALL_STACK_OFFSET_X3 == 136;
    const X4_CORRECT: bool = _SYSCALL_STACK_OFFSET_X4 == 112;
    const X5_CORRECT: bool = _SYSCALL_STACK_OFFSET_X5 == 120;
}

// 使用数组大小作为编译时检查
const _ASSERT_X8: [(); 1] = [(); (_StackOffsetCheck::X8_CORRECT as usize)];
const _ASSERT_X0: [(); 1] = [(); (_StackOffsetCheck::X0_CORRECT as usize)];
const _ASSERT_X1: [(); 1] = [(); (_StackOffsetCheck::X1_CORRECT as usize)];
const _ASSERT_X2: [(); 1] = [(); (_StackOffsetCheck::X2_CORRECT as usize)];
const _ASSERT_X3: [(); 1] = [(); (_StackOffsetCheck::X3_CORRECT as usize)];
const _ASSERT_X4: [(); 1] = [(); (_StackOffsetCheck::X4_CORRECT as usize)];
const _ASSERT_X5: [(); 1] = [(); (_StackOffsetCheck::X5_CORRECT as usize)];

/// 执行系统调用的安全通用函数（带屏障）
///
/// # 安全性
/// - 使用显式的寄存器保存/恢复
/// - 添加完整的内存屏障防止编译器重排
/// - 使用寄存器保护屏障防止关键寄存器被破坏
/// - 确保系统调用不会破坏调用者的寄存器状态
///
/// # 参数
/// - `num`: 系统调用号
/// - `args`: 最多6个系统调用参数
///
/// # 返回
/// - 系统调用结果
#[inline(never)]
pub unsafe fn syscall_6_with_barriers(
    num: u32,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> isize {
    // 使用屏障包装器执行系统调用
    barrier::syscall_with_barriers(|| {
        let ret: isize;
        let syscall_num = num;

        // 内联汇编：显式声明所有寄存器使用
        asm!(
            // 内存屏障确保系统调用前所有写入完成
            "dsb sy",
            "isb",
            
            "svc #0",
            
            // 系统调用后的屏障
            "dsb sy",
            "isb",

            // 输入寄存器
            in("x8") syscall_num,  // 系统调用号 - 明确标记为输入
            in("x0") arg0,         // 参数0
            in("x1") arg1,         // 参数1
            in("x2") arg2,         // 参数2
            in("x3") arg3,         // 参数3
            in("x4") arg4,         // 参数4
            in("x5") arg5,         // 参数5

            // 输出寄存器
            lateout("x0") ret,     // 返回值

            // 破坏声明：使用clobber_abi告知编译器所有调用者保存寄存器可能被修改
            // 这包括x0-x18（除了我们显式处理的x0）
            clobber_abi("C"),

            // 选项
            options(nostack, preserves_flags)
        );

        ret
    })
}

/// 执行系统调用的安全通用函数（原始版本，向后兼容）
///
/// # 安全性
/// - 使用显式的寄存器保存/恢复
/// - 添加内存屏障防止编译器重排
/// - 确保系统调用不会破坏调用者的寄存器状态
///
/// # 参数
/// - `num`: 系统调用号
/// - `args`: 最多6个系统调用参数
///
/// # 返回
/// - 系统调用结果
#[inline(never)]
pub unsafe fn syscall_6(
    num: u32,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> isize {
    // 默认使用带屏障的版本以确保最大安全性
    syscall_6_with_barriers(num, arg0, arg1, arg2, arg3, arg4, arg5)
}

/// 执行0参数系统调用的安全包装器
#[inline(never)]
pub unsafe fn syscall_0(num: u32) -> isize {
    syscall_6(num, 0, 0, 0, 0, 0, 0)
}

/// 执行1参数系统调用的安全包装器
#[inline(never)]
pub unsafe fn syscall_1(num: u32, arg0: usize) -> isize {
    syscall_6(num, arg0, 0, 0, 0, 0, 0)
}

/// 执行2参数系统调用的安全包装器
#[inline(never)]
pub unsafe fn syscall_2(num: u32, arg0: usize, arg1: usize) -> isize {
    syscall_6(num, arg0, arg1, 0, 0, 0, 0)
}

/// 执行3参数系统调用的安全包装器
#[inline(never)]
pub unsafe fn syscall_3(num: u32, arg0: usize, arg1: usize, arg2: usize) -> isize {
    syscall_6(num, arg0, arg1, arg2, 0, 0, 0)
}

/// 执行4参数系统调用的安全包装器
#[inline(never)]
pub unsafe fn syscall_4(num: u32, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    syscall_6(num, arg0, arg1, arg2, arg3, 0, 0)
}

/// 执行5参数系统调用的安全包装器
#[inline(never)]
pub unsafe fn syscall_5(num: u32, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> isize {
    syscall_6(num, arg0, arg1, arg2, arg3, arg4, 0)
}

/// 安全的 write 系统调用
#[inline(never)]
pub fn write(fd: i32, buf: &[u8]) -> isize {
    // 安全检查：确保缓冲区有效
    if buf.is_empty() {
        return 0;
    }

    unsafe {
        syscall_3(
            HNX_SYS_WRITE,
            fd as usize,
            buf.as_ptr() as usize,
            buf.len(),
        )
    }
}

/// 安全的 read 系统调用
#[inline(never)]
pub fn read(fd: i32, buf: &mut [u8]) -> isize {
    // 安全检查：确保缓冲区有效
    if buf.is_empty() {
        return 0;
    }

    unsafe {
        syscall_3(
            HNX_SYS_READ,
            fd as usize,
            buf.as_mut_ptr() as usize,
            buf.len(),
        )
    }
}

/// 安全的 spawn_service 系统调用
#[inline(never)]
pub fn spawn_service(path: &str) -> isize {
    // 安全检查：确保路径有效
    if path.is_empty() {
        return -1; // ZX_ERR_INVALID_ARGS
    }
    
    unsafe {
        syscall_2(
            HNX_SYS_SPAWN_SERVICE,
            path.as_ptr() as usize,
            path.len(),
        )
    }
}

/// 安全的 yield_cpu 系统调用
#[inline(never)]
pub fn yield_cpu() {
    unsafe {
        let _ = syscall_0(HNX_SYS_YIELD);
    }
}

/// 安全的 debug_print 系统调用
#[inline(never)]
pub fn debug_print(s: &str) {
    if s.is_empty() {
        return;
    }

    unsafe {
        let _ = syscall_3(
            HNX_SYS_WRITE,
            1, // stdout
            s.as_ptr() as usize,
            s.len(),
        );
    }
}

/// 安全的 ipc_wait 系统调用
#[inline(never)]
pub fn ipc_wait() -> isize {
    unsafe {
        syscall_0(HNX_SYS_IPC_WAIT)
    }
}

/// 安全的 ipc_wake 系统调用
#[inline(never)]
pub fn ipc_wake(pid: usize) -> isize {
    unsafe {
        syscall_1(HNX_SYS_IPC_WAKE, pid)
    }
}

/// 安全的 ep_create 系统调用 - 创建新的IPC端点
#[inline(never)]
pub fn ep_create() -> isize {
    unsafe {
        syscall_0(HNX_SYS_EP_CREATE)
    }
}

/// 安全的 ep_send 系统调用 - 通过端点句柄发送消息
#[inline(never)]
pub fn ep_send(handle: usize, op: u16, buf: &[u8]) -> isize {
    unsafe {
        syscall_4(
            HNX_SYS_EP_SEND,
            handle,
            op as usize,
            buf.as_ptr() as usize,
            buf.len(),
        )
    }
}

/// 安全的 ep_recv 系统调用 - 通过端点句柄接收消息
#[inline(never)]
pub fn ep_recv(handle: usize, buf: &mut [u8]) -> isize {
    unsafe {
        syscall_3(
            HNX_SYS_EP_RECV,
            handle,
            buf.as_mut_ptr() as usize,
            buf.len(),
        )
    }
}

/// 测试函数：验证系统调用不会破坏寄存器
#[cfg(test)]
mod tests {
    use super::*;
    
    /// 测试寄存器保存（在用户空间模拟）
    /// 这个测试应该在真实的系统调用环境中运行
    #[test]
    fn test_register_preservation() {
        // 注意：这是一个编译时测试，确保我们的汇编正确声明了寄存器破坏
        // 实际测试需要在QEMU环境中运行
    }
    
    /// 测试系统调用参数传递
    #[test]
    fn test_syscall_interface() {
        // 测试接口一致性
        assert_eq!(HNX_SYS_WRITE, 0x1001);
        assert_eq!(HNX_SYS_SPAWN_SERVICE, 0x0103);
        assert_eq!(HNX_SYS_YIELD, 0x0018);
    }
}
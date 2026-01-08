//! 系统调用分发器
//!
//! 基于对象模型的系统调用实现，遵循Zircon微内核设计。
//! 系统调用通过异常进入内核，由本模块分派到具体的对象操作。

use crate::object::traits::*;
use crate::object::handle::Handle;
use crate::object::table::HandleTable;
use alloc::sync::Arc;
use alloc::vec::Vec;
use shared::abi::syscalls::*;

/// 系统调用结果类型
pub type SyscallResult = Result<usize, ObjectError>;

/// 系统调用分发器
///
/// 每个进程有一个独立的系统调用分发器，包含该进程的句柄表。
pub struct SyscallDispatcher {
    /// 进程句柄表
    handle_table: HandleTable,
}

impl SyscallDispatcher {
    /// 创建新的系统调用分发器
    pub fn new() -> Self {
        Self {
            handle_table: HandleTable::new(),
        }
    }

    /// 根据系统调用号分派处理
    pub fn dispatch(&mut self, syscall: u32, args: &[usize; 6]) -> SyscallResult {
        match syscall {
            // 通道操作
            HNX_SYS_CHANNEL_CREATE => self.sys_channel_create(args),
            HNX_SYS_CHANNEL_WRITE => self.sys_channel_write(args),
            HNX_SYS_CHANNEL_READ => self.sys_channel_read(args),

            // 进程操作
            HNX_SYS_PROCESS_CREATE => self.sys_process_create(args),
            HNX_SYS_SPAWN_SERVICE => self.sys_spawn_service(args),

            // 线程操作
            HNX_SYS_THREAD_CREATE => self.sys_thread_create(args),

            // VMO操作
            HNX_SYS_VMO_CREATE => self.sys_vmo_create(args),

            // 基本I/O
            HNX_SYS_WRITE => self.sys_write(args),
            HNX_SYS_READ => self.sys_read(args),

            // IPC操作
            HNX_SYS_IPC_WAIT => self.sys_ipc_wait(args),
            HNX_SYS_IPC_WAKE => self.sys_ipc_wake(args),

            // 未实现的系统调用
            _ => Err(ObjectError::NotSupported),
        }
    }

    /// 创建通道对象（HNX_SYS_CHANNEL_CREATE）
    ///
    /// 参数：
    /// - args[0]: 指向存储两个句柄的数组的指针（输出参数）
    /// 返回值：
    /// - 成功返回0，失败返回错误码
    fn sys_channel_create(&mut self, args: &[usize; 6]) -> SyscallResult {
        // 创建一对通道
        use crate::object::types::channel::Channel;
        let (chan_a, chan_b) = Channel::create_pair();

        // 创建两个句柄，分别对应通道的两端
        // 通道两端具有相同的权限
        let rights = ObjectRights::READ | ObjectRights::WRITE | ObjectRights::DUPLICATE;
        let handle_a = Handle::new(chan_a, rights, 0); // 假设进程ID为0
        let handle_b = Handle::new(chan_b, rights, 0);

        // 将句柄添加到句柄表
        let handle_a_id = self.handle_table.add(handle_a);
        let handle_b_id = self.handle_table.add(handle_b);

        // 将句柄ID写入用户空间（暂未实现）
        // 需要将handle_a_id和handle_b_id写入args[0]指向的内存
        // 对于测试，我们只返回成功
        Ok(0)
    }

    /// 向通道写入消息（HNX_SYS_CHANNEL_WRITE）
    fn sys_channel_write(&mut self, args: &[usize; 6]) -> SyscallResult {
        let handle_id = args[0] as u32;
        let data_ptr = args[1] as *const u8;
        let data_len = args[2];
        let _handles_ptr = args[3] as *const u32;
        let _handles_count = args[4];

        // 获取句柄
        let handle = self.handle_table.get(handle_id)
            .ok_or(ObjectError::BadHandle)?;

        // 检查写权限
        handle.check_rights(ObjectRights::WRITE)?;

        // 获取通道对象
        let channel = handle.object()
            .downcast_arc::<crate::object::types::channel::Channel>()
            .map_err(|_| ObjectError::WrongType)?;

        // 读取用户空间数据（暂未实现安全检查）
        // 实际实现需要将数据复制到内核缓冲区
        let data = unsafe { core::slice::from_raw_parts(data_ptr, data_len) };
        
        // 写入通道，不传递句柄（暂未实现句柄传递）
        channel.write(data, Vec::new()).map_err(|e| e)?;

        Ok(data_len)
    }

    /// 从通道读取消息（HNX_SYS_CHANNEL_READ）
    fn sys_channel_read(&mut self, args: &[usize; 6]) -> SyscallResult {
        let handle_id = args[0] as u32;
        let data_ptr = args[1] as *mut u8;
        let data_capacity = args[2];
        let _handles_ptr = args[3] as *mut u32;
        let _handles_capacity = args[4];

        // 获取句柄
        let handle = self.handle_table.get(handle_id)
            .ok_or(ObjectError::BadHandle)?;

        // 检查读权限
        handle.check_rights(ObjectRights::READ)?;

        // 获取通道对象
        let channel = handle.object()
            .downcast_arc::<crate::object::types::channel::Channel>()
            .map_err(|_| ObjectError::WrongType)?;

        // 准备缓冲区
        let mut buffer = vec![0u8; data_capacity];
        
        // 从通道读取（暂未实现超时和句柄接收）
        let (read_len, _handles) = channel.read(&mut buffer, 0).map_err(|e| e)?;

        // 将数据复制到用户空间
        unsafe {
            core::ptr::copy_nonoverlapping(buffer.as_ptr(), data_ptr, read_len);
        }

        // 返回读取的字节数
        Ok(read_len)
    }

    /// 创建进程（HNX_SYS_PROCESS_CREATE）
    fn sys_process_create(&mut self, _args: &[usize; 6]) -> SyscallResult {
        // 创建进程对象
        use crate::object::types::process::Process;
        let process = Process::new();

        // 创建句柄
        let rights = ObjectRights::READ | ObjectRights::WRITE | ObjectRights::DUPLICATE | ObjectRights::DESTROY;
        let handle = Handle::new(process, rights, 0);

        // 将句柄添加到句柄表
        let handle_id = self.handle_table.add(handle);

        Ok(handle_id as usize)
    }

    /// 从initrd加载服务（HNX_SYS_SPAWN_SERVICE）
    fn sys_spawn_service(&mut self, _args: &[usize; 6]) -> SyscallResult {
        // 暂未实现
        Err(ObjectError::NotSupported)
    }

    /// 创建线程（HNX_SYS_THREAD_CREATE）
    fn sys_thread_create(&mut self, _args: &[usize; 6]) -> SyscallResult {
        // 创建线程对象
        use crate::object::types::thread::Thread;
        let thread = Thread::new();

        // 创建句柄
        let rights = ObjectRights::READ | ObjectRights::WRITE | ObjectRights::DUPLICATE | ObjectRights::DESTROY;
        let handle = Handle::new(thread, rights, 0);

        // 将句柄添加到句柄表
        let handle_id = self.handle_table.add(handle);

        Ok(handle_id as usize)
    }

    /// 创建虚拟内存对象（HNX_SYS_VMO_CREATE）
    fn sys_vmo_create(&mut self, args: &[usize; 6]) -> SyscallResult {
        let size = args[0];

        // 创建VMO对象
        use crate::object::types::vmo::Vmo;
        let vmo = Vmo::new(size);

        // 创建句柄
        let rights = ObjectRights::READ | ObjectRights::WRITE | ObjectRights::DUPLICATE | ObjectRights::MAP;
        let handle = Handle::new(vmo, rights, 0);

        // 将句柄添加到句柄表
        let handle_id = self.handle_table.add(handle);

        Ok(handle_id as usize)
    }

    /// 向文件描述符写入数据（HNX_SYS_WRITE）
    fn sys_write(&mut self, args: &[usize; 6]) -> SyscallResult {
        let fd = args[0] as i32;
        let buf = args[1] as *const u8;
        let count = args[2];

        // 目前只支持标准输出（fd=1）
        if fd != 1 {
            return Err(ObjectError::InvalidArgs);
        }

        // 将数据复制到内核缓冲区并打印
        let data = unsafe { core::slice::from_raw_parts(buf, count) };
        for &byte in data {
            crate::debug::putc(byte);
        }

        Ok(count)
    }

    /// 从文件描述符读取数据（HNX_SYS_READ）
    fn sys_read(&mut self, _args: &[usize; 6]) -> SyscallResult {
        // 暂未实现
        Err(ObjectError::NotSupported)
    }

    /// 等待IPC消息（HNX_SYS_IPC_WAIT）
    fn sys_ipc_wait(&mut self, _args: &[usize; 6]) -> SyscallResult {
        // 暂未实现
        Err(ObjectError::NotSupported)
    }

    /// 唤醒等待IPC的线程（HNX_SYS_IPC_WAKE）
    fn sys_ipc_wake(&mut self, _args: &[usize; 6]) -> SyscallResult {
        // 暂未实现
        Err(ObjectError::NotSupported)
    }
}
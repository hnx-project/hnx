//! 进程对象
//!
//! 进程对象表示一个执行环境，包含：
//! - 地址空间（页表）
//! - 句柄表
//! - 线程集合
//! - 资源限制

use crate::impl_kernel_object;
use crate::object::traits::*;
use crate::object::HandleTable;
use alloc::sync::Arc;
use alloc::vec::Vec;
use shared::sync::mutex::Mutex;

/// 进程对象
pub struct Process {
    /// 对象ID
    id: KObjectId,
    /// 进程ID
    pid: u64,
    /// 进程句柄表
    handle_table: HandleTable,
    /// 进程中的线程列表
    threads: Mutex<Vec<Arc<crate::object::types::thread::Thread>>>,
    /// 进程状态
    state: Mutex<ProcessState>,
    /// 退出代码（如果进程已退出）
    exit_code: Mutex<Option<i32>>,
}

/// 进程状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// 进程正在运行
    Running,
    /// 进程已退出
    Exited,
    /// 进程已终止
    Terminated,
}

impl Process {
    /// 创建新进程
    pub fn new(pid: u64) -> Arc<Self> {
        Arc::new(Self {
            id: KObjectId::new(),
            pid,
            handle_table: HandleTable::new(pid),
            threads: Mutex::new(Vec::new()),
            state: Mutex::new(ProcessState::Running),
            exit_code: Mutex::new(None),
        })
    }

    /// 获取进程ID
    pub fn pid(&self) -> u64 {
        self.pid
    }

    /// 获取句柄表
    pub fn handle_table(&self) -> &HandleTable {
        &self.handle_table
    }

    /// 添加线程到进程
    pub fn add_thread(&self, thread: Arc<crate::object::types::thread::Thread>) {
        self.threads.lock().push(thread);
    }

    /// 从进程移除线程
    pub fn remove_thread(&self, tid: u64) {
        let mut threads = self.threads.lock();
        if let Some(pos) = threads.iter().position(|t| t.tid() == tid) {
            threads.remove(pos);
        }
    }

    /// 退出进程
    pub fn exit(&self, code: i32) {
        *self.state.lock() = ProcessState::Exited;
        *self.exit_code.lock() = Some(code);

        // 终止所有线程
        let threads = self.threads.lock();
        for _thread in threads.iter() {
            // 实际实现需要终止线程
        }
    }

    /// 获取进程状态
    pub fn state(&self) -> ProcessState {
        *self.state.lock()
    }

    /// 获取退出代码
    pub fn exit_code(&self) -> Option<i32> {
        *self.exit_code.lock()
    }
}

impl_kernel_object!(Process, "Process");

impl Dispatcher for Process {
    fn can_block(&self) -> bool {
        false // 进程本身不会阻塞，但线程会
    }

    fn on_block(&self) {
        // 进程不支持阻塞
    }

    fn on_unblock(&self) {
        // 进程不支持阻塞
    }
}

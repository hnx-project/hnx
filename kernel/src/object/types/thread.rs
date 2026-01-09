//! 线程对象
//!
//! 线程对象表示一个执行单元，包含：
//! - CPU上下文（寄存器状态）
//! - 栈
//! - 线程局部存储（TLS）
//! - 调度状态

use alloc::sync::Arc;
use shared::sync::mutex::Mutex;
use crate::object::traits::*;
use crate::impl_kernel_object;

/// 线程对象
pub struct Thread {
    /// 对象ID
    id: KObjectId,
    /// 线程ID
    tid: u64,
    /// 所属进程
    process: Arc<crate::object::types::process::Process>,
    /// 线程状态
    state: Mutex<ThreadState>,
    /// 线程上下文（寄存器状态）
    context: Mutex<ThreadContext>,
    /// 栈信息
    stack: Option<Arc<crate::object::types::vmo::Vmo>>,
    /// 线程局部存储
    tls: Mutex<Option<usize>>,
}

/// 线程状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// 线程已创建但未运行
    Created,
    /// 线程正在运行
    Running,
    /// 线程可运行（就绪）
    Ready,
    /// 线程阻塞等待
    Blocked,
    /// 线程已退出
    Exited,
}

/// 线程上下文（寄存器状态）
#[derive(Debug, Clone)]
pub struct ThreadContext {
    /// 通用寄存器（AArch64有31个通用寄存器）
    pub regs: [usize; 31],
    /// 程序计数器
    pub pc: usize,
    /// 栈指针
    pub sp: usize,
    /// 链接寄存器
    pub lr: usize,
    /// 处理器状态
    pub pstate: u64,
}

impl Thread {
    /// 创建新线程
    pub fn new(
        tid: u64,
        process: Arc<crate::object::types::process::Process>,
        entry_point: usize,
        stack: Option<Arc<crate::object::types::vmo::Vmo>>,
    ) -> Arc<Self> {
        let thread = Arc::new(Self {
            id: KObjectId::new(),
            tid,
            process,
            state: Mutex::new(ThreadState::Created),
            context: Mutex::new(ThreadContext {
                regs: [0; 31],
                pc: entry_point,
                sp: 0,
                lr: 0,
                pstate: 0,
            }),
            stack,
            tls: Mutex::new(None),
        });
        
        // 将线程添加到进程
        thread.process.add_thread(thread.clone());
        
        thread
    }
    
    /// 获取线程ID
    pub fn tid(&self) -> u64 {
        self.tid
    }
    
    /// 获取所属进程
    pub fn process(&self) -> &Arc<crate::object::types::process::Process> {
        &self.process
    }
    
    /// 获取线程状态
    pub fn state(&self) -> ThreadState {
        *self.state.lock()
    }
    
    /// 设置线程状态
    pub fn set_state(&self, state: ThreadState) {
        *self.state.lock() = state;
    }
    
    /// 获取线程上下文
    pub fn context(&self) -> ThreadContext {
        self.context.lock().clone()
    }
    
    /// 设置线程上下文
    pub fn set_context(&self, context: ThreadContext) {
        *self.context.lock() = context;
    }
    
    /// 获取线程局部存储地址
    pub fn tls(&self) -> Option<usize> {
        *self.tls.lock()
    }
    
    /// 设置线程局部存储地址
    pub fn set_tls(&self, tls: usize) {
        *self.tls.lock() = Some(tls);
    }
    
    /// 启动线程
    pub fn start(&self) -> Result<(), ObjectError> {
        let mut state = self.state.lock();
        if *state != ThreadState::Created {
            return Err(ObjectError::InvalidArgs);
        }
        
        *state = ThreadState::Ready;
        Ok(())
    }
    
    /// 退出线程
    pub fn exit(&self, _code: i32) {
        *self.state.lock() = ThreadState::Exited;
        
        // 从进程中移除线程
        self.process.remove_thread(self.tid);
    }
}

impl_kernel_object!(Thread, "Thread");

impl Dispatcher for Thread {
    fn can_block(&self) -> bool {
        true // 线程可以阻塞
    }
    
    fn on_block(&self) {
        *self.state.lock() = ThreadState::Blocked;
    }
    
    fn on_unblock(&self) {
        *self.state.lock() = ThreadState::Ready;
    }
}
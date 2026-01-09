// 异常处理抽象特征
//
// 要求实现：
// - 异常向量表
// - 异常上下文保存/恢复
// - 异常分类处理
// - 页错误处理
// - 断点/单步
// - 系统调用入口
// - 调试异常

use super::super::ArchResult;

/// 异常架构抽象特征
pub trait ExceptionArch: Send + Sync {
    /// 异常上下文对象类型
    type Context: ExceptionContext;

    /// 异常向量表类型
    type VectorTable: ExceptionVectorTable;

    /// 初始化异常处理
    fn init() -> ArchResult<()>;

    /// 设置异常向量表
    fn set_vector_table(table: &Self::VectorTable);

    /// 获取当前异常上下文
    unsafe fn current_context() -> Self::Context;

    /// 恢复异常上下文
    unsafe fn restore_context(context: Self::Context);

    /// 处理异常
    fn handle_exception(context: &mut Self::Context) -> ExceptionAction;

    /// 注册异常处理程序
    fn register_handler(
        vector: ExceptionVector,
        handler: ExceptionHandler,
        data: usize,
    ) -> ArchResult<()>;

    /// 注销异常处理程序
    fn unregister_handler(vector: ExceptionVector) -> ArchResult<()>;

    /// 触发软件异常
    fn trigger_software_exception(vector: ExceptionVector, code: u32) -> ArchResult<()>;

    /// 设置断点
    fn set_breakpoint(address: usize, kind: BreakpointKind) -> ArchResult<BreakpointHandle>;

    /// 清除断点
    fn clear_breakpoint(handle: BreakpointHandle) -> ArchResult<()>;

    /// 启用单步执行
    fn enable_single_step(context: &mut Self::Context);

    /// 禁用单步执行
    fn disable_single_step(context: &mut Self::Context);
}

/// 异常上下文特征
pub trait ExceptionContext: Send + Sync {
    /// 获取程序计数器
    fn program_counter(&self) -> usize;

    /// 设置程序计数器
    fn set_program_counter(&mut self, pc: usize);

    /// 获取栈指针
    fn stack_pointer(&self) -> usize;

    /// 设置栈指针
    fn set_stack_pointer(&mut self, sp: usize);

    /// 获取通用寄存器
    fn general_register(&self, index: usize) -> usize;

    /// 设置通用寄存器
    fn set_general_register(&mut self, index: usize, value: usize);

    /// 获取异常编号
    fn exception_number(&self) -> u32;

    /// 获取错误码
    fn error_code(&self) -> u64;

    /// 获取异常地址
    fn fault_address(&self) -> usize;

    /// 是否来自用户模式
    fn from_user_mode(&self) -> bool;

    /// 保存寄存器状态
    fn save_registers(&mut self);

    /// 恢复寄存器状态
    fn restore_registers(&mut self);
}

/// 异常向量表特征
pub trait ExceptionVectorTable: Send + Sync {
    /// 向量表基地址
    fn base_address(&self) -> usize;

    /// 向量数量
    fn vector_count(&self) -> usize;

    /// 设置向量处理程序
    fn set_handler(&mut self, vector: ExceptionVector, handler: ExceptionHandler);

    /// 获取向量处理程序
    fn get_handler(&self, vector: ExceptionVector) -> Option<ExceptionHandler>;

    /// 安装向量表
    fn install(&self);

    /// 重新加载向量表
    fn reload(&self);
}

/// 异常向量
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExceptionVector(u16);

impl ExceptionVector {
    pub const SYNC_EL1T: Self = Self(0);
    pub const IRQ_EL1T: Self = Self(1);
    pub const FIQ_EL1T: Self = Self(2);
    pub const SERROR_EL1T: Self = Self(3);
    pub const SYNC_EL1H: Self = Self(4);
    pub const IRQ_EL1H: Self = Self(5);
    pub const FIQ_EL1H: Self = Self(6);
    pub const SERROR_EL1H: Self = Self(7);
    pub const SYNC_EL0_64: Self = Self(8);
    pub const IRQ_EL0_64: Self = Self(9);
    pub const FIQ_EL0_64: Self = Self(10);
    pub const SERROR_EL0_64: Self = Self(11);

    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(&self) -> u16 {
        self.0
    }
}

/// 异常处理程序类型
pub type ExceptionHandler = fn(context: &mut dyn ExceptionContext) -> ExceptionAction;

/// 异常处理动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionAction {
    /// 恢复执行
    Resume,
    /// 终止线程
    TerminateThread,
    /// 终止进程
    TerminateProcess,
    /// 重启系统
    RebootSystem,
    /// 调用用户处理程序
    CallHandler(usize),
}

/// 断点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointKind {
    Instruction,
    DataRead,
    DataWrite,
    DataReadWrite,
}

/// 断点句柄
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreakpointHandle(u64);

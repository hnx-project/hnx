// CPU架构抽象特征
//
// 要求实现：
// - CPU识别和配置
// - 特权级管理
// - 屏障指令
// - 原子操作
// - 性能监控
// - 多核同步
// - 电源管理

use super::super::ArchResult;

/// CPU架构抽象特征
pub trait CpuArch: Send + Sync {
    /// CPU信息对象
    type CpuInfo: CpuInfo;
    
    /// 寄存器上下文对象
    type Context: CpuContext;
    
    /// 初始化CPU架构
    fn init() -> ArchResult<()>;
    
    /// 获取当前CPU信息
    fn current_cpu() -> Self::CpuInfo;
    
    /// 获取CPU数量
    fn cpu_count() -> usize;
    
    /// 启动从核
    fn start_secondary(cpu_id: usize, entry_point: usize) -> ArchResult<()>;
    
    /// 数据同步屏障
    fn data_synchronization_barrier();
    
    /// 指令同步屏障
    fn instruction_synchronization_barrier();
    
    /// 内存屏障
    fn memory_barrier();
    
    /// 等待中断
    fn wait_for_interrupt();
    
    /// 禁用中断
    fn disable_interrupts();
    
    /// 启用中断
    fn enable_interrupts();
    
    /// 保存中断状态并禁用
    fn save_and_disable_interrupts() -> bool;
    
    /// 恢复中断状态
    fn restore_interrupts(previous_state: bool);
    
    /// 获取当前异常级别/模式
    fn current_privilege_level() -> u8;
    
    /// 设置栈指针
    fn set_stack_pointer(sp: usize);
    
    /// 获取时间戳计数器
    fn read_timestamp_counter() -> u64;
}

/// CPU信息特征
pub trait CpuInfo: Send + Sync {
    /// CPU ID
    fn id(&self) -> usize;
    
    /// 供应商字符串
    fn vendor(&self) -> &str;
    
    /// 架构版本
    fn arch_version(&self) -> u32;
    
    /// 是否支持虚拟化
    fn has_virtualization(&self) -> bool;
    
    /// 是否支持SIMD
    fn has_simd(&self) -> bool;
    
    /// 缓存信息
    fn cache_info(&self) -> CacheInfo;
}

/// CPU上下文特征
pub trait CpuContext: Send + Sync {
    /// 创建新上下文
    fn new(entry_point: usize, stack_pointer: usize, arg: usize) -> Self;
    
    /// 切换到上下文
    unsafe fn switch_to(&self);
    
    /// 保存当前上下文
    unsafe fn save_current() -> Self;
}

/// 缓存信息对象
#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub l1_data: CacheLevel,
    pub l1_instruction: CacheLevel,
    pub l2: Option<CacheLevel>,
    pub l3: Option<CacheLevel>,
}

/// 缓存层级信息
#[derive(Debug, Clone, Copy)]
pub struct CacheLevel {
    pub size: usize,
    pub line_size: usize,
    pub associativity: usize,
}

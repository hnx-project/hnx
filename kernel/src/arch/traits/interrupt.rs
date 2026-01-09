// 中断控制器抽象特征
//
// 要求实现：
// - 中断控制器初始化
// - 中断启用/禁用
// - 中断优先级
// - 中断亲和性
// - 软件中断
// - 中断掩码
// - 嵌套中断支持

use super::super::ArchResult;

/// 中断架构抽象特征
pub trait InterruptArch: Send + Sync {
    /// 中断控制器对象类型
    type Controller: InterruptController;

    /// 中断描述符对象类型
    type Descriptor: InterruptDescriptor;

    /// 中断号类型
    type IrqNumber: IrqNumber;

    /// 初始化中断系统
    fn init() -> ArchResult<()>;

    /// 获取中断控制器
    fn controller() -> &'static Self::Controller;

    /// 注册中断处理程序
    fn register_handler(
        irq: Self::IrqNumber,
        handler: InterruptHandler,
        data: usize,
    ) -> ArchResult<()>;

    /// 注销中断处理程序
    fn unregister_handler(irq: Self::IrqNumber) -> ArchResult<()>;

    /// 启用中断
    fn enable_irq(irq: Self::IrqNumber) -> ArchResult<()>;

    /// 禁用中断
    fn disable_irq(irq: Self::IrqNumber) -> ArchResult<()>;

    /// 获取中断状态
    fn get_irq_state(irq: Self::IrqNumber) -> IrqState;

    /// 发送EOI（中断结束）
    fn send_eoi(irq: Self::IrqNumber);

    /// 设置中断优先级
    fn set_priority(irq: Self::IrqNumber, priority: u8) -> ArchResult<()>;

    /// 设置中断亲和性
    fn set_affinity(irq: Self::IrqNumber, cpu_mask: u32) -> ArchResult<()>;

    /// 触发软件中断
    fn trigger_software_irq(irq: Self::IrqNumber, cpu_mask: u32) -> ArchResult<()>;
}

/// 中断控制器特征
pub trait InterruptController: Send + Sync {
    /// 控制器名称
    fn name(&self) -> &str;

    /// 支持的最大中断数
    fn max_interrupts(&self) -> usize;

    /// 初始化控制器
    fn initialize(&self) -> ArchResult<()>;

    /// 启用控制器
    fn enable(&self);

    /// 禁用控制器
    fn disable(&self);

    /// 获取待处理中断
    fn get_pending(&self) -> Option<u32>;

    /// 确认中断
    fn acknowledge(&self, irq: u32);

    /// 完成中断处理
    fn complete(&self, irq: u32);

    /// 屏蔽所有中断
    fn mask_all(&self);

    /// 取消屏蔽所有中断
    fn unmask_all(&self);

    /// 获取控制器类型
    fn controller_type(&self) -> ControllerType;
}

/// 中断描述符特征
pub trait InterruptDescriptor: Send + Sync {
    /// 中断号
    fn irq_number(&self) -> u32;

    /// 中断类型
    fn irq_type(&self) -> IrqType;

    /// 处理程序
    fn handler(&self) -> Option<InterruptHandler>;

    /// 设置处理程序
    fn set_handler(&mut self, handler: Option<InterruptHandler>);

    /// 优先级
    fn priority(&self) -> u8;

    /// 设置优先级
    fn set_priority(&mut self, priority: u8);

    /// 亲和性掩码
    fn affinity_mask(&self) -> u32;

    /// 设置亲和性掩码
    fn set_affinity_mask(&mut self, mask: u32);

    /// 是否已启用
    fn is_enabled(&self) -> bool;

    /// 启用状态
    fn set_enabled(&mut self, enabled: bool);

    /// 是否在处理中
    fn is_in_service(&self) -> bool;

    /// 设置处理状态
    fn set_in_service(&mut self, in_service: bool);
}

/// 中断号特征
pub trait IrqNumber: Send + Sync + Copy + Eq + Ord {
    /// 转换为u32
    fn to_u32(&self) -> u32;

    /// 从u32创建
    fn from_u32(value: u32) -> Self;

    /// 是否为有效中断号
    fn is_valid(&self) -> bool;

    /// 是否为软件中断
    fn is_software(&self) -> bool;

    /// 是否为私有中断
    fn is_private(&self) -> bool;
}

/// 中断处理程序类型
pub type InterruptHandler = fn(irq: u32, data: usize) -> bool;

/// 中断状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqState {
    Disabled,
    Enabled,
    Pending,
    InService,
    Masked,
}

/// 中断类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    EdgeTriggered,
    LevelTriggered,
    MessageSignaled,
}

/// 控制器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerType {
    Pic,
    Apic,
    X2Apic,
    Gicv2,
    Gicv3,
    Plic,
}

// ARMv8-A interrupt模块实现
//
// 要求实现：
// - 定义具体类型：
//   Aarch64interrupt 等
// - 实现架构抽象特征对应的具体方法
// - 使用tock-registers定义寄存器
// - 使用aarch64-cpu访问CPU功能
// - 遵循"一切皆对象"原则
//
// 主要API：
// - init(): 初始化本模块
// - 实现对应trait的所有方法
// - 提供对象构造和销毁接口
//
// 对象类型：
// - Aarch64interrupt: 主对象类型
// - Aarch64interruptInfo: 信息对象
// - Aarch64interruptConfig: 配置对象
// - Aarch64interruptState: 状态对象
//
// 关键功能：
// - GICv2/GICv3支持
// - 中断优先级分组
// - CPU接口管理
// - 软件中断生成
// - 中断亲和性设置
// - LPI支持(可选)
// - 虚拟中断支持(可选)

// 需要实现的具体类型（示例）：
// pub struct Aarch64Module;
// pub struct Aarch64ModuleInfo;
// pub struct Aarch64ModuleConfig;
//
// 需要实现的方法（示例）：
// pub fn init() -> ArchResult<()>;
// pub fn create(config: &Aarch64ModuleConfig) -> ArchResult<Aarch64Module>;
// pub fn info(&self) -> Aarch64ModuleInfo;
//
// 寄存器定义（示例）：
// use tock_registers::{register_bitfields, register_structs, registers::*};
//
// 对象管理（示例）：
// use crate::object::{KernelObject, Handle, ObjectRights};
//
// 遵循"一切皆对象"原则，所有资源都封装为对象。

use crate::arch::traits::interrupt::*;
use crate::arch::{ArchError, ArchResult};
use shared::sync::mutex::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Aarch64IrqNumber(u32);

impl IrqNumber for Aarch64IrqNumber {
    fn to_u32(&self) -> u32 {
        self.0
    }

    fn from_u32(value: u32) -> Self {
        Self(value)
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn is_software(&self) -> bool {
        self.0 < 16
    }

    fn is_private(&self) -> bool {
        self.0 < 32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Aarch64IrqDescriptor {
    irq: u32,
    irq_type: IrqType,
    handler: Option<InterruptHandler>,
    data: usize,
    priority: u8,
    affinity_mask: u32,
    enabled: bool,
    in_service: bool,
}

impl Aarch64IrqDescriptor {
    pub const fn new(irq: u32) -> Self {
        Self {
            irq,
            irq_type: IrqType::LevelTriggered,
            handler: None,
            data: 0,
            priority: 0,
            affinity_mask: 1,
            enabled: false,
            in_service: false,
        }
    }
}

impl InterruptDescriptor for Aarch64IrqDescriptor {
    fn irq_number(&self) -> u32 {
        self.irq
    }

    fn irq_type(&self) -> IrqType {
        self.irq_type
    }

    fn handler(&self) -> Option<InterruptHandler> {
        self.handler
    }

    fn set_handler(&mut self, handler: Option<InterruptHandler>) {
        self.handler = handler;
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn set_priority(&mut self, priority: u8) {
        self.priority = priority;
    }

    fn affinity_mask(&self) -> u32 {
        self.affinity_mask
    }

    fn set_affinity_mask(&mut self, mask: u32) {
        self.affinity_mask = mask;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_in_service(&self) -> bool {
        self.in_service
    }

    fn set_in_service(&mut self, in_service: bool) {
        self.in_service = in_service;
    }
}

#[derive(Debug)]
pub struct Aarch64Gic;

impl InterruptController for Aarch64Gic {
    fn name(&self) -> &str {
        "gic"
    }

    fn max_interrupts(&self) -> usize {
        256
    }

    fn initialize(&self) -> ArchResult<()> {
        Ok(())
    }

    fn enable(&self) {}

    fn disable(&self) {}

    fn get_pending(&self) -> Option<u32> {
        None
    }

    fn acknowledge(&self, _irq: u32) {}

    fn complete(&self, _irq: u32) {}

    fn mask_all(&self) {}

    fn unmask_all(&self) {}

    fn controller_type(&self) -> ControllerType {
        ControllerType::Gicv2
    }
}

#[derive(Debug)]
pub struct Aarch64Interrupt;

const MAX_IRQS: usize = 256;
static HANDLERS: Mutex<[Option<(InterruptHandler, usize)>; MAX_IRQS]> =
    Mutex::new([None; MAX_IRQS]);

static CONTROLLER: Aarch64Gic = Aarch64Gic;

pub fn init() -> ArchResult<()> {
    CONTROLLER.initialize()?;
    Ok(())
}

pub fn controller() -> &'static Aarch64Gic {
    &CONTROLLER
}

pub fn register_handler(
    irq: Aarch64IrqNumber,
    handler: InterruptHandler,
    data: usize,
) -> ArchResult<()> {
    let index = irq.to_u32() as usize;
    if index >= MAX_IRQS {
        return Err(ArchError::InvalidArgument);
    }
    let mut table = HANDLERS.lock();
    table[index] = Some((handler, data));
    Ok(())
}

pub fn unregister_handler(irq: Aarch64IrqNumber) -> ArchResult<()> {
    let index = irq.to_u32() as usize;
    if index >= MAX_IRQS {
        return Err(ArchError::InvalidArgument);
    }
    let mut table = HANDLERS.lock();
    table[index] = None;
    Ok(())
}

pub fn enable_irq(_irq: Aarch64IrqNumber) -> ArchResult<()> {
    Ok(())
}

pub fn disable_irq(_irq: Aarch64IrqNumber) -> ArchResult<()> {
    Ok(())
}

pub fn get_irq_state(_irq: Aarch64IrqNumber) -> IrqState {
    IrqState::Enabled
}

pub fn send_eoi(_irq: Aarch64IrqNumber) {}

pub fn set_priority(_irq: Aarch64IrqNumber, _priority: u8) -> ArchResult<()> {
    Ok(())
}

pub fn set_affinity(_irq: Aarch64IrqNumber, _cpu_mask: u32) -> ArchResult<()> {
    Ok(())
}

pub fn trigger_software_irq(_irq: Aarch64IrqNumber, _cpu_mask: u32) -> ArchResult<()> {
    Err(ArchError::NotSupported)
}

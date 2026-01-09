// 架构抽象特征定义
//
// 定义所有架构必须实现的抽象接口。

pub mod cpu;
pub mod mmu;
pub mod interrupt;
pub mod timer;
pub mod exception;
pub mod boot;

// 重新导出主要特征
pub use cpu::CpuArch;
pub use mmu::MmuArch;
pub use interrupt::InterruptArch;
pub use timer::TimerArch;
pub use exception::ExceptionArch;
pub use boot::BootArch;

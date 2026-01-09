// 架构抽象特征定义
//
// 定义所有架构必须实现的抽象接口。

pub mod boot;
pub mod cpu;
pub mod exception;
pub mod interrupt;
pub mod mmu;
pub mod timer;

// 重新导出主要特征
pub use boot::BootArch;
pub use cpu::CpuArch;
pub use exception::ExceptionArch;
pub use interrupt::InterruptArch;
pub use mmu::MmuArch;
pub use timer::TimerArch;

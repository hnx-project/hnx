// 通用架构代码
pub mod boot;
pub mod traits;
// pub mod interrupt;
pub mod mmu;
pub mod timer;

// 导出到内核其他部分使用
pub use self::boot::BootInfo;
pub use self::traits::*;
// pub use self::interrupt::IrqHandler;

// 内核架构抽象层
//
// 提供统一的架构抽象接口，支持多种CPU架构。
// 遵循"一切皆对象"的设计原则。

#![allow(unused_imports)]

pub mod common;
pub mod implementations;
pub mod traits;

pub use common::PAGE_SIZE;

#[cfg(target_arch = "aarch64")]
pub use implementations::aarch64 as current;

#[cfg(target_arch = "riscv64")]
pub use implementations::riscv64 as current;

pub mod platform {
    use crate::arch::ArchResult;

    pub fn init() -> ArchResult<()> {
        unsafe { crate::arch::current::boot::early_init() };
        crate::arch::current::boot::init()?;
        crate::arch::current::cpu::init()
    }
}

pub mod cpu {
    pub fn wait_for_interrupt() {
        crate::arch::current::cpu::wait_for_interrupt()
    }
}

pub mod mmu {
    use crate::arch::ArchResult;

    pub fn init() -> ArchResult<()> {
        crate::arch::current::mmu::init()
    }
}

pub mod interrupt {
    use crate::arch::ArchResult;

    pub fn init() -> ArchResult<()> {
        crate::arch::current::interrupt::init()
    }
}

pub mod timer {
    use crate::arch::ArchResult;

    pub fn init() -> ArchResult<()> {
        crate::arch::current::timer::init()
    }
}

// 架构抽象层公共类型
pub type ArchResult<T> = Result<T, ArchError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchError {
    NotSupported,
    InvalidArgument,
    PermissionDenied,
    NoMemory,
    Timeout,
    DeviceError,
    UnalignedAccess,
    AlreadyMapped,
    NotMapped,
    InternalError,
}

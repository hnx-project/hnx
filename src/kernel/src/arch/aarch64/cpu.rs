//! AArch64 CPU 操作实现

use crate::arch::common::traits::Cpu;

/// AArch64 CPU 实现
pub struct AArch64Cpu;

impl Cpu for AArch64Cpu {
    fn id() -> u32 {
        let mut cpu_id: u64;
        unsafe {
            core::arch::asm!("mrs {}, mpidr_el1", out(reg) cpu_id);
        }
        (cpu_id & 0xFF) as u32
    }
    
    fn barrier() {
        unsafe {
            core::arch::asm!("dmb ish");
        }
    }
    
    fn read_barrier() {
        unsafe {
            core::arch::asm!("dmb ishld");
        }
    }
    
    fn write_barrier() {
        unsafe {
            core::arch::asm!("dmb ishst");
        }
    }

    fn yield_cpu() {
        unsafe {
            core::arch::asm!("yield");
        }
    }

    fn wait_for_interrupt() {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

// 模块级函数包装
pub fn id() -> u32 {
    AArch64Cpu::id()
}

pub fn barrier() {
    AArch64Cpu::barrier();
}

pub fn read_barrier() {
    AArch64Cpu::read_barrier();
}

pub fn write_barrier() {
    AArch64Cpu::write_barrier();
}

pub fn yield_cpu() {
    AArch64Cpu::yield_cpu();
}

pub fn wait_for_interrupt() {
    AArch64Cpu::wait_for_interrupt();
}
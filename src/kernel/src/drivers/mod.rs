//! Device Drivers - Core Abstractions Only
//!
//! The microkernel only includes minimal device driver abstractions required for:
//! - **UART**: Serial console for kernel debugging (PL011 only)
//! - **GIC**: ARM Generic Interrupt Controller for interrupt routing
//! - **DTB**: Device Tree parsing for hardware discovery
//! - **Bus**: Basic bus detection and enumeration
//!
//! All other drivers (block, network, USB, etc.) should be moved to user space services.

pub mod uart;
pub mod gic;
pub mod bus;
pub mod dtb;

// TODO: Move to user space
pub mod ipc_protocol;
pub mod device_manager;
pub mod block;

use crate::arch::common::boot::BootInfo;
use crate::arch::common::mmu::{ArchType, MmuFlags};

/// Initialize core device drivers from Device Tree Blob
///
/// This function parses the DTB and initializes only the minimal drivers
/// needed for kernel operation:
/// 1. PL011 UART for debug console
/// 2. GIC for interrupt handling
/// 3. Memory-mapped I/O regions
pub fn init_from_dtb(boot_info: &BootInfo) {
    crate::info!("drivers: initializing from DTB");
    
    let det = bus::detect(boot_info.dtb_ptr as usize, ArchType::AArch64);
    
    // Initialize UART (PL011 only for microkernel)
    if let Some(b) = det.pl011_base {
        uart::pl011::set_base(b);
        crate::memory::map(b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::memory::map(crate::memory::KERNEL_BASE + b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::info!("drivers: pl011 uart base=0x{:016X}", b as u64);
    } else {
        let b = env!("HNX_FALLBACK_PL011_BASE_DEC")
            .parse::<usize>()
            .unwrap_or(0x0900_0000usize);
        uart::pl011::set_base(b);
        crate::memory::map(b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::memory::map(crate::memory::KERNEL_BASE + b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::info!("drivers: pl011 not found in DTB, using fallback base=0x{:016X}", b as u64);
    }
    
    // Initialize GIC (Generic Interrupt Controller)
    if let (Some(d), Some(c)) = (det.gicd_base, det.gicc_base) {
        gic::set_bases(d, c);
        for base in [d, c] {
            crate::memory::map(base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
            crate::memory::map(crate::memory::KERNEL_BASE + base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        }
        crate::info!("drivers: gic gicd=0x{:016X} gicc=0x{:016X}", d as u64, c as u64);
    } else {
        let d = env!("HNX_FALLBACK_GICD_BASE_DEC").parse::<usize>().unwrap_or(0x0800_0000usize);
        let c = env!("HNX_FALLBACK_GICC_BASE_DEC").parse::<usize>().unwrap_or(0x0801_0000usize);
        gic::set_bases(d, c);
        for base in [d, c] {
            crate::memory::map(base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
            crate::memory::map(crate::memory::KERNEL_BASE + base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        }
        crate::info!("drivers: gic not found in DTB, using fallback gicd=0x{:016X} gicc=0x{:016X}", d as u64, c as u64);
    }
    
    // Map other MMIO regions discovered in DTB
    for base in det.other_mmio.iter().copied().flatten() {
        crate::memory::map(base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::memory::map(crate::memory::KERNEL_BASE + base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::info!("drivers: mapped mmio region at 0x{:016X}", base as u64);
    }
    
    crate::info!("drivers: core drivers initialized");
}
pub mod uart;
pub mod gic;
pub mod bus;
pub mod dtb_parser;
pub mod ipc_protocol;
pub mod device_manager;
// pub mod net;
pub mod block;


use crate::arch::common::boot::BootInfo;
use crate::arch::common::mmu::{ArchType, MmuFlags};

pub fn init_from_dtb(boot_info: &BootInfo) {
    let det = bus::detect(boot_info.dtb_ptr as usize, ArchType::AArch64);
    if let Some(b) = det.pl011_base {
        uart::pl011::set_base(b);
        crate::memory::virtual_::map(b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::memory::virtual_::map(crate::memory::virtual_::KERNEL_BASE + b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::info!("dtb: pl011 base=0x{:016X}", b as u64);
    } else {
        let b = env!("HNX_FALLBACK_PL011_BASE_DEC")
            .parse::<usize>()
            .unwrap_or(0x0900_0000usize);
        uart::pl011::set_base(b);
        crate::memory::virtual_::map(b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::memory::virtual_::map(crate::memory::virtual_::KERNEL_BASE + b, b, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::info!("dtb: pl011 not found, fallback base=0x{:016X}", b as u64);
    }
    if let (Some(d), Some(c)) = (det.gicd_base, det.gicc_base) {
        gic::set_bases(d, c);
        for base in [d, c] {
            crate::memory::virtual_::map(base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
            crate::memory::virtual_::map(crate::memory::virtual_::KERNEL_BASE + base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        }
        crate::info!("dtb: gicd=0x{:016X} gicc=0x{:016X}", d as u64, c as u64);
    } else {
        let d = env!("HNX_FALLBACK_GICD_BASE_DEC").parse::<usize>().unwrap_or(0x0800_0000usize);
        let c = env!("HNX_FALLBACK_GICC_BASE_DEC").parse::<usize>().unwrap_or(0x0801_0000usize);
        gic::set_bases(d, c);
        for base in [d, c] {
            crate::memory::virtual_::map(base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
            crate::memory::virtual_::map(crate::memory::virtual_::KERNEL_BASE + base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        }
        crate::info!("dtb: gic not found, fallback gicd=0x{:016X} gicc=0x{:016X}", d as u64, c as u64);
    }
    for base in det.other_mmio.iter().copied().flatten() {
        crate::memory::virtual_::map(base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::memory::virtual_::map(crate::memory::virtual_::KERNEL_BASE + base, base, MmuFlags::READ.combine(MmuFlags::WRITE).combine(MmuFlags::DEVICE));
        crate::info!("dtb: other mmio base=0x{:016X}", base as u64);
    }
}
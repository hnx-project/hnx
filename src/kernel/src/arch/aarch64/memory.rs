//! AArch64 架构特定的内存操作实现

use crate::arch::common::traits::Memory;

/// AArch64 内存操作实现
pub struct AArch64Memory;

impl Memory for AArch64Memory {
    fn data_sync_barrier() {
        unsafe {
            core::arch::asm!("dsb ish");
        }
    }
    
    fn instruction_barrier() {
        unsafe {
            core::arch::asm!("isb");
        }
    }
    
    fn data_memory_barrier() {
        unsafe {
            core::arch::asm!("dmb ish");
        }
    }
    
    fn tlb_flush_all() {
        unsafe {
            core::arch::asm!("dsb ish", "tlbi vmalle1", "dsb ish", "isb");
        }
    }
    
    fn tlb_invalidate(vaddr: usize, asid: Option<u16>) {
        unsafe {
            core::arch::asm!("dsb ish");
            if let Some(asid_val) = asid {
                let va_bits = (vaddr as u64 >> 12) & 0xFFFFFFFFFFFF;
                let value = va_bits | ((asid_val as u64) << 48);
                // tlbi vae1is: Invalidate TLB entry by VA and ASID, inner shareable
                core::arch::asm!("tlbi vae1is, {}", in(reg) value);
            } else {
                // Invalidate by VA only (all ASIDs)
                let value = (vaddr as u64 >> 12) & 0xFFFFFFFFFFFF;
                core::arch::asm!("tlbi vaae1is, {}", in(reg) value);
            }
            core::arch::asm!("dsb ish", "isb");
        }
    }
    
    fn get_current_asid() -> u16 {
        unsafe {
            let ttbr0: u64;
            core::arch::asm!("mrs {}, ttbr0_el1", out(reg) ttbr0);
            ((ttbr0 >> 48) & 0xFFFF) as u16
        }
    }
    
    fn clean_dcache_range(addr: usize, size: usize) {
        if size == 0 { return; }
        let mut p = addr & !63usize;
        let end = addr.saturating_add(size);
        unsafe {
            while p < end {
                core::arch::asm!("dc cvau, {x}", x = in(reg) p);
                p = p.saturating_add(64);
            }
            core::arch::asm!("dsb ish; isb");
        }
    }
    
    fn flush_icache_all() {
        unsafe {
            core::arch::asm!("ic iallu; dsb ish; isb");
        }
    }
    
    fn yield_cpu() {
        unsafe {
            core::arch::asm!("yield");
        }
    }
}

// 为方便使用，提供模块级函数包装
pub fn data_sync_barrier() {
    AArch64Memory::data_sync_barrier();
}

pub fn instruction_barrier() {
    AArch64Memory::instruction_barrier();
}

pub fn data_memory_barrier() {
    AArch64Memory::data_memory_barrier();
}

pub fn tlb_flush_all() {
    AArch64Memory::tlb_flush_all();
}

pub fn tlb_invalidate(vaddr: usize, asid: Option<u16>) {
    AArch64Memory::tlb_invalidate(vaddr, asid);
}

pub fn get_current_asid() -> u16 {
    AArch64Memory::get_current_asid()
}

pub fn clean_dcache_range(addr: usize, size: usize) {
    AArch64Memory::clean_dcache_range(addr, size);
}

pub fn flush_icache_all() {
    AArch64Memory::flush_icache_all();
}

pub fn yield_cpu() {
    AArch64Memory::yield_cpu();
}
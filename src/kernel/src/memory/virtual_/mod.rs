use super::physical::alloc_pages;
use super::physical::PhysAddr;
use crate::arch::common::mmu::{ArchType, MmuFlags};
use crate::console;
use spin::Mutex;

pub type VirtAddr = usize;

pub const KERNEL_BASE: usize = 0xFFFF_8000_0000_0000;
const BLOCK_SIZE_2M: usize = 2 * 1024 * 1024;
const PAGE_SIZE_4K: usize = 4 * 1024;
const UART_BASE_PL011: usize = 0x0900_0000;

static PT_LOCK: Mutex<()> = Mutex::new(());
static VMA_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
pub struct Vma {
    pub base: usize,
    pub end: usize,
    pub flags: MmuFlags,
}

static VMA_TABLE: Mutex<[(usize, Option<Vma>); 64]> = Mutex::new([(0, None); 64]);

extern "C" {
    static mut L2_TABLE_USER: [u64; 512];
    static mut L2_TABLE_KERNEL: [u64; 512];
}

pub fn init() {
    crate::info!("mm/vmm basic init");
}

pub fn create_user_l1() -> Option<usize> {
    if let Some(l1_pa) = alloc_pages(1) {
        crate::info!("mm/vmm create_user_l1: l1_pa=0x{:016X}", l1_pa);
        unsafe {
            core::ptr::write_bytes(l1_pa as *mut u8, 0, 4096);
        }
        unsafe {
            // Allocate L2 table for low addresses (0x00000000 - 0x3FFFFFFF)
            if let Some(l2_low_pa) = alloc_pages(1) {
                core::ptr::write_bytes(l2_low_pa as *mut u8, 0, 4096);
                
                // Allocate L2 table for kernel addresses (0x40000000 - 0x7FFFFFFF)
                if let Some(l2_kernel_pa) = alloc_pages(1) {
                    core::ptr::write_bytes(l2_kernel_pa as *mut u8, 0, 4096);
                    
                    let l1_ptr = l1_pa as *mut u64;
                    
                    // Map L2 table for low addresses (index 0) - contains devices like UART
                    let l2_low_desc = ((l2_low_pa as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64; // Table descriptor
                    core::ptr::write_volatile(l1_ptr.add(0), l2_low_desc);
                    
                    // Map L2 table for kernel addresses (index 1) - contains kernel code/data
                    let l2_kernel_desc = ((l2_kernel_pa as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64; // Table descriptor
                    core::ptr::write_volatile(l1_ptr.add(1), l2_kernel_desc);
                    
                    // Map devices in the low address L2 table
                    let l2_low_ptr = l2_low_pa as *mut u64;
                    
                    // Map UART at its virtual address (0x09000000)
                    // L2 index for 0x09000000 is (0x09000000 >> 21) = 72
                    let uart_l2_idx = (UART_BASE_PL011 >> 21) & 0x1FF;
                    let uart_entry = ((UART_BASE_PL011 as u64) & !((BLOCK_SIZE_2M as u64) - 1)) | (1u64 << 2) | (1u64 << 10) | 1u64; // Device memory, AF=1, valid
                    core::ptr::write_volatile(l2_low_ptr.add(uart_l2_idx), uart_entry);
                    
                    // Map kernel physical memory in the kernel address L2 table
                    let l2_kernel_ptr = l2_kernel_pa as *mut u64;
                    for i in 0..128 {  // Map 256MB of kernel memory (0x40000000 - 0x4FFFFFFF)
                        let phys_addr = 0x40000000u64 + (i as u64 * 2 * 1024 * 1024);
                        let entry = (phys_addr & !((BLOCK_SIZE_2M as u64) - 1)) | (0u64 << 12) | (1u64 << 10) | (0u64 << 2) | 1u64; // Normal memory, AF=1, valid
                        core::ptr::write_volatile(l2_kernel_ptr.add(i), entry);
                    }
                    
                    crate::info!("mm/vmm create_user_l1: l2_low_pa=0x{:016X}, l2_kernel_pa=0x{:016X}", l2_low_pa, l2_kernel_pa);
                }
            }
        }
        Some(l1_pa)
    } else {
        None
    }
}

fn is_kernel_va(vaddr: VirtAddr) -> bool {
    vaddr >= KERNEL_BASE
}

fn l2_index(vaddr: VirtAddr) -> usize {
    ((vaddr >> 21) & 0x1FF)
}

fn l3_index(vaddr: VirtAddr) -> usize {
    ((vaddr >> 12) & 0x1FF)
}

unsafe fn ensure_l3_table(vaddr: VirtAddr) -> Option<*mut u64> {
    let idx = l2_index(vaddr);
    let l2_ptr = if is_kernel_va(vaddr) {
        core::ptr::addr_of_mut!(L2_TABLE_KERNEL) as *mut u64
    } else {
        core::ptr::addr_of_mut!(L2_TABLE_USER) as *mut u64
    };
    let entry = core::ptr::read_volatile(l2_ptr.add(idx));
    let ty = entry & 0x3;
    if ty == 0 {
        if let Some(base) = alloc_pages(1) {
            core::ptr::write_bytes(base as *mut u8, 0, 4096);
            let desc = ((base as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64;
            core::ptr::write_volatile(l2_ptr.add(idx), desc);
            Some(base as *mut u64)
        } else {
            None
        }
    } else if ty == 1 {
        if let Some(base) = alloc_pages(1) {
            let desc = ((base as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64;
            core::ptr::write_volatile(l2_ptr.add(idx), desc);
            Some(base as *mut u64)
        } else {
            None
        }
    } else {
        let table_pa = (entry & !((PAGE_SIZE_4K as u64) - 1)) as usize;
        Some(table_pa as *mut u64)
    }
}

unsafe fn ensure_l3_table_in_pt(pt_base: usize, vaddr: VirtAddr) -> Option<*mut u64> {
    let l1 = pt_base as *mut u64;
    // User page tables are rooted at L1, then L2, then L3 (4KB granule, 3-level)
    // L1 index uses VA[38:30] when T0SZ=25 (39-bit VA space)
    let l1i = ((vaddr >> 30) & 0x1FF);
    let l1ent = core::ptr::read_volatile(l1.add(l1i));
    let l2_pa = if l1ent & 0x3 == 3 {
        (l1ent & !((PAGE_SIZE_4K as u64) - 1)) as usize
    } else if let Some(pa) = alloc_pages(1) {
        core::ptr::write_bytes(pa as *mut u8, 0, 4096);
        let desc = ((pa as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64;
        core::ptr::write_volatile(l1.add(l1i), desc);
        pa
    } else {
        return None;
    };
    let l2 = l2_pa as *mut u64;
    // L2 index uses VA[29:21]
    let l2i = ((vaddr >> 21) & 0x1FF);
    let l2ent = core::ptr::read_volatile(l2.add(l2i));
    if l2ent & 0x3 == 3 {
        let l3_pa = (l2ent & !((PAGE_SIZE_4K as u64) - 1)) as usize;
        Some(l3_pa as *mut u64)
    } else if let Some(pa) = alloc_pages(1) {
        core::ptr::write_bytes(pa as *mut u8, 0, 4096);
        let desc = ((pa as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64;
        core::ptr::write_volatile(l2.add(l2i), desc);
        Some(pa as *mut u64)
    } else {
        None
    }
}

pub fn map(vaddr: VirtAddr, paddr: PhysAddr, flags: MmuFlags) {
    let _g = PT_LOCK.lock();
    let attrs = (1u64 << 10) | (2u64 << 8) | flags.to_arch(ArchType::AArch64);
    unsafe {
        if vaddr.is_multiple_of(BLOCK_SIZE_2M) && paddr.is_multiple_of(BLOCK_SIZE_2M) {
            let idx = l2_index(vaddr);
            let desc = ((paddr as u64) & !((BLOCK_SIZE_2M as u64) - 1)) | 1u64 | attrs;
            let l2_ptr = if is_kernel_va(vaddr) {
                core::ptr::addr_of_mut!(L2_TABLE_KERNEL) as *mut u64
            } else {
                core::ptr::addr_of_mut!(L2_TABLE_USER) as *mut u64
            };
            core::ptr::write_volatile(l2_ptr.add(idx), desc);
        } else if vaddr.is_multiple_of(PAGE_SIZE_4K) && paddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3_ptr) = ensure_l3_table(vaddr) {
                let idx3 = l3_index(vaddr);
                let entry = ((paddr as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64 | attrs;
                core::ptr::write_volatile(l3_ptr.add(idx3), entry);
            }
        }
        core::arch::asm!("dsb ish", "isb");
    }
}

pub fn remap(vaddr: VirtAddr, new_flags: MmuFlags) {
    let _g = PT_LOCK.lock();
    unsafe {
        if vaddr.is_multiple_of(BLOCK_SIZE_2M) {
            let idx = l2_index(vaddr);
            let l2_ptr = if is_kernel_va(vaddr) {
                core::ptr::addr_of_mut!(L2_TABLE_KERNEL) as *mut u64
            } else {
                core::ptr::addr_of_mut!(L2_TABLE_USER) as *mut u64
            };
            let entry = core::ptr::read_volatile(l2_ptr.add(idx));
            if entry & 0x3 != 0 {
                let paddr = entry & !0x1FFFFF;
                let attrs = (1u64 << 10) | (2u64 << 8) | new_flags.to_arch(ArchType::AArch64);
                let new_entry = paddr | 1u64 | attrs;
                core::ptr::write_volatile(l2_ptr.add(idx), new_entry);
            }
        } else if vaddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3_ptr) = ensure_l3_table(vaddr) {
                let idx3 = l3_index(vaddr);
                let entry = core::ptr::read_volatile(l3_ptr.add(idx3));
                if entry & 0x3 != 0 {
                    let paddr = entry & !0xFFF;
                    let attrs = (1u64 << 10) | (2u64 << 8) | new_flags.to_arch(ArchType::AArch64);
                    let new_entry = paddr | 3u64 | attrs;
                    core::ptr::write_volatile(l3_ptr.add(idx3), new_entry);
                }
            }
        }
        core::arch::asm!("dsb ish", "isb");
    }
}

pub fn map_in_pt(pt_base: usize, vaddr: VirtAddr, paddr: PhysAddr, flags: MmuFlags) {
    let _g = PT_LOCK.lock();
    let attrs = (1u64 << 10) | (2u64 << 8) | flags.to_arch(ArchType::AArch64);
    unsafe {
        if vaddr.is_multiple_of(PAGE_SIZE_4K) && paddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3) = ensure_l3_table_in_pt(pt_base, vaddr) {
                let idx3 = l3_index(vaddr);
                let entry = ((paddr as u64) & !((PAGE_SIZE_4K as u64) - 1)) | 3u64 | attrs;
                core::ptr::write_volatile(l3.add(idx3), entry);
                crate::info!("mm/vmm map_in_pt: pt=0x{:X} va=0x{:X} pa=0x{:X} l3=0x{:X} idx3={} entry=0x{:016X} flags={:?} flags.bits=0x{:X} attrs=0x{:X}",
                    pt_base, vaddr, paddr, l3 as usize, idx3, entry, flags, flags.bits(), attrs);
            }
            core::arch::asm!("dsb ish", "isb");
        }
    }
}

pub fn remap_in_pt(pt_base: usize, vaddr: VirtAddr, new_flags: MmuFlags) {
    let _g = PT_LOCK.lock();
    unsafe {
        if let Some(l3) = ensure_l3_table_in_pt(pt_base, vaddr) {
            let idx3 = l3_index(vaddr);
            let entry = core::ptr::read_volatile(l3.add(idx3));
            if entry & 0x3 != 0 {
                let paddr = entry & !0xFFF;
                let attrs = (1u64 << 10) | (2u64 << 8) | new_flags.to_arch(ArchType::AArch64);
                let new_entry = paddr | 3u64 | attrs;
                core::ptr::write_volatile(l3.add(idx3), new_entry);
                core::arch::asm!("dsb ish", "isb");
            }
        }
    }
}

pub fn vma_add(pt_base: usize, base: usize, size: usize, flags: MmuFlags) {
    let _g = VMA_LOCK.lock();
    let mut tbl = VMA_TABLE.lock();
    let end = base.saturating_add(size);
    for slot in tbl.iter_mut() {
        if slot.1.is_none() {
            *slot = (pt_base, Some(Vma { base, end, flags }));
            break;
        }
    }
}

pub fn handle_page_fault(pt: usize, vaddr: usize) -> bool {
    crate::info!("handle_page_fault: pt=0x{:016X} vaddr=0x{:016X}", pt, vaddr);
    let _g = VMA_LOCK.lock();
    let tbl = VMA_TABLE.lock();
    for (owner, entry) in tbl.iter() {
        if *owner == pt {
            if let Some(vma) = entry {
                crate::info!("handle_page_fault: vma base=0x{:016X} end=0x{:016X} flags={:?}", vma.base, vma.end, vma.flags);
                if vaddr >= vma.base && vaddr < vma.end {
                    let va = vaddr & !((PAGE_SIZE_4K) - 1);
                    crate::info!("handle_page_fault: allocating page for va=0x{:016X}", va);
                    if let Some(pa) = alloc_pages(1) {
                        let mut f = vma.flags;
                        crate::info!("handle_page_fault: mapping va=0x{:016X} pa=0x{:016X} flags={:?}", va, pa, f);
                        map_in_pt(pt, va, pa, f);
                        // Verify the mapping was written
                        unsafe {
                            if let Some(l3) = ensure_l3_table_in_pt(pt, va) {
                                let idx3 = l3_index(va);
                                let entry = core::ptr::read_volatile(l3.add(idx3));
                                crate::info!("handle_page_fault: verified entry=0x{:016X} for va=0x{:016X}", entry, va);
                                if entry & 0x3 == 0 {
                                    crate::info!("handle_page_fault: WARNING: entry not valid!");
                                }
                            }
                        }
                        // Invalidate TLB entry for this page
                        unsafe {
                            core::arch::asm!("dsb ish");
                            // tlbi vmalle1is: Invalidate all EL1 TLB entries, inner shareable
                            core::arch::asm!("tlbi vmalle1is");
                            core::arch::asm!("dsb ish", "isb");
                        }
                        crate::info!("handle_page_fault: success, TLB invalidated");
                        return true;
                    } else {
                        crate::info!("handle_page_fault: alloc_pages failed");
                        return false;
                    }
                }
            }
        }
    }
    crate::info!("handle_page_fault: no matching VMA");
    false
}

pub fn unmap(vaddr: VirtAddr) {
    let _g = PT_LOCK.lock();
    unsafe {
        if vaddr.is_multiple_of(BLOCK_SIZE_2M) {
            let idx = l2_index(vaddr);
            let l2_ptr = if is_kernel_va(vaddr) {
                core::ptr::addr_of_mut!(L2_TABLE_KERNEL) as *mut u64
            } else {
                core::ptr::addr_of_mut!(L2_TABLE_USER) as *mut u64
            };
            core::ptr::write_volatile(l2_ptr.add(idx), 0u64);
        } else if vaddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3_ptr) = ensure_l3_table(vaddr) {
                let idx3 = l3_index(vaddr);
                core::ptr::write_volatile(l3_ptr.add(idx3), 0u64);
            }
        }
        core::arch::asm!("dsb ish", "tlbi vmalle1", "dsb ish", "isb");
    }
}

pub fn unmap_in_pt(pt_base: usize, vaddr: VirtAddr) {
    let _g = PT_LOCK.lock();
    unsafe {
        if vaddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3) = ensure_l3_table_in_pt(pt_base, vaddr) {
                let idx3 = l3_index(vaddr);
                core::ptr::write_volatile(l3.add(idx3), 0u64);
            }
            core::arch::asm!("dsb ish", "tlbi vmalle1", "dsb ish", "isb");
        }
    }
}

pub fn query_mapping(vaddr: VirtAddr) -> Option<(PhysAddr, MmuFlags)> {
    let _g = PT_LOCK.lock();
    unsafe {
        if vaddr.is_multiple_of(BLOCK_SIZE_2M) {
            let idx = l2_index(vaddr);
            let l2_ptr = if is_kernel_va(vaddr) {
                core::ptr::addr_of_mut!(L2_TABLE_KERNEL) as *mut u64
            } else {
                core::ptr::addr_of_mut!(L2_TABLE_USER) as *mut u64
            };
            let entry = core::ptr::read_volatile(l2_ptr.add(idx));
            if entry & 0x1 != 0 {
                let paddr = (entry & !0x1FFFFF) as PhysAddr;
                // Extract flags from the entry
                let flags = MmuFlags::from_arch(entry, ArchType::AArch64);
                Some((paddr, flags))
            } else {
                None
            }
        } else if vaddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3_ptr) = ensure_l3_table(vaddr) {
                let idx3 = l3_index(vaddr);
                let entry = core::ptr::read_volatile(l3_ptr.add(idx3));
                if entry & 0x3 != 0 {
                    let paddr = (entry & !0xFFF) as PhysAddr;
                    // Extract flags from the entry
                    let flags = MmuFlags::from_arch(entry, ArchType::AArch64);
                    Some((paddr, flags))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn query_mapping_in_pt(pt_base: usize, vaddr: VirtAddr) -> Option<(PhysAddr, MmuFlags)> {
    let _g = PT_LOCK.lock();
    unsafe {
        if vaddr.is_multiple_of(PAGE_SIZE_4K) {
            if let Some(l3) = ensure_l3_table_in_pt(pt_base, vaddr) {
                let idx3 = l3_index(vaddr);
                let entry = core::ptr::read_volatile(l3.add(idx3));
                if entry & 0x3 != 0 {
                    let paddr = (entry & !0xFFF) as PhysAddr;
                    // Extract flags from the entry
                    let flags = MmuFlags::from_arch(entry, ArchType::AArch64);
                    Some((paddr, flags))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn walk_page_table(pt_base: usize, callback: &dyn Fn(VirtAddr, PhysAddr, MmuFlags)) {
    let _g = PT_LOCK.lock();
    unsafe {
        let l1 = pt_base as *const u64;
        // Walk L1 entries (512 entries)
        for l1i in 0..512 {
            let l1ent = core::ptr::read_volatile(l1.add(l1i));
            if l1ent & 0x3 == 3 {
                let l2_pa = (l1ent & !0xFFF) as usize;
                let l2 = l2_pa as *const u64;
                // Walk L2 entries (512 entries)
                for l2i in 0..512 {
                    let l2ent = core::ptr::read_volatile(l2.add(l2i));
                    let ty = l2ent & 0x3;
                    if ty == 1 {
                        // Block descriptor (2MB block)
                        let paddr = (l2ent & !0x1FFFFF) as PhysAddr;
                        let flags = MmuFlags::from_arch(l2ent, ArchType::AArch64);
                        let vaddr = (l1i << 30) | (l2i << 21);
                        callback(vaddr, paddr, flags);
                    } else if ty == 3 {
                        // Table descriptor, walk L3
                        let l3_pa = (l2ent & !0xFFF) as usize;
                        let l3 = l3_pa as *const u64;
                        // Walk L3 entries (512 entries)
                        for l3i in 0..512 {
                            let l3ent = core::ptr::read_volatile(l3.add(l3i));
                            if l3ent & 0x3 == 3 {
                                let paddr = (l3ent & !0xFFF) as PhysAddr;
                                let flags = MmuFlags::from_arch(l3ent, ArchType::AArch64);
                                let vaddr = (l1i << 30) | (l2i << 21) | (l3i << 12);
                                callback(vaddr, paddr, flags);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn dump_page_table(pt_base: usize) {
    crate::info!("Page table dump for PT base: 0x{:016X}", pt_base);
    walk_page_table(pt_base, &|vaddr, paddr, flags| {
        let readable = if flags.contains(MmuFlags::READ) { 'R' } else { '-' };
        let writable = if flags.contains(MmuFlags::WRITE) { 'W' } else { '-' };
        let executable = if flags.contains(MmuFlags::EXECUTE) { 'X' } else { '-' };
        let user = if flags.contains(MmuFlags::USER) { 'U' } else { 'K' };
        let device = if flags.contains(MmuFlags::DEVICE) { 'D' } else { 'N' };
        crate::info!(
            "  VA: 0x{:016X} -> PA: 0x{:016X} [{}{}{}{}{}]",
            vaddr, paddr, readable, writable, executable, user, device
        );
    });
}

pub fn map_common_mmio() {
    let mmios: &[(usize, MmuFlags)] = &[
        (
            0x0800_0000,
            MmuFlags::READ
                .combine(MmuFlags::WRITE)
                .combine(MmuFlags::DEVICE),
        ), // GICD
        (
            0x0801_0000,
            MmuFlags::READ
                .combine(MmuFlags::WRITE)
                .combine(MmuFlags::DEVICE),
        ), // GICC
        (
            0x0900_0000,
            MmuFlags::READ
                .combine(MmuFlags::WRITE)
                .combine(MmuFlags::DEVICE),
        ), // PL011
        (
            0x1000_0000,
            MmuFlags::READ
                .combine(MmuFlags::WRITE)
                .combine(MmuFlags::DEVICE),
        ), // VirtIO base
    ];
    for (base, flags) in mmios.iter().copied() {
        map(base, base, flags);
        map(KERNEL_BASE + base, base, flags);
    }
}
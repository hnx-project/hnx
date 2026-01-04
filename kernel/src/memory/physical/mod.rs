use crate::arch::common::boot::BootInfo;
use crate::console;

const PAGE_SIZE: usize = 4096;
const MAX_ORDER: usize = 12;
const MAX_PAGES: usize = 4096;

pub type PhysAddr = usize;

static mut REGION_START: usize = 0;
static mut REGION_END: usize = 0;
static mut FREE_LIST: [[usize; MAX_PAGES]; MAX_ORDER + 1] = [[0; MAX_PAGES]; MAX_ORDER + 1];
static mut FREE_COUNT: [usize; MAX_ORDER + 1] = [0; MAX_ORDER + 1];
static mut FREE_CALLS: usize = 0;
static mut COALESCE_EVENTS: usize = 0;
static mut ALLOC_CALLS: usize = 0;

fn order_for_pages(pages: usize) -> usize {
    let mut o = 0;
    let mut n = 1usize;
    while n < pages && o < MAX_ORDER {
        n <<= 1;
        o += 1;
    }
    o
}

fn push_block(order: usize, addr: usize) {
    unsafe {
        // Skip invalid blocks (outside managed region)
        if !(addr >= REGION_START && addr + (PAGE_SIZE << order) <= REGION_END) {
            // Silently ignore invalid block - likely from early boot or reserved memory
            return;
        }
        // Validate inputs
        if order > MAX_ORDER {
            return;
        }
        if !addr.is_multiple_of(PAGE_SIZE << order) {
            return;
        }
        let c = FREE_COUNT[order];
        if c >= MAX_PAGES {
            return;
        }
        FREE_LIST[order][c] = addr;
        FREE_COUNT[order] = c + 1;
    }
}

fn pop_block(order: usize) -> Option<usize> {
    unsafe {
        if FREE_COUNT[order] == 0 {
            None
        } else {
            FREE_COUNT[order] -= 1;
            Some(FREE_LIST[order][FREE_COUNT[order]])
        }
    }
}

fn remove_block(order: usize, addr: usize) -> bool {
    unsafe {
        let c = FREE_COUNT[order];
        for i in 0..c {
            if FREE_LIST[order][i] == addr {
                FREE_LIST[order][i] = FREE_LIST[order][c - 1];
                FREE_COUNT[order] -= 1;
                return true;
            }
        }
        false
    }
}

pub fn init(boot: BootInfo) {
    unsafe {
        extern "C" {
            static __kernel_end: u8;
        }
        let k_end = &__kernel_end as *const u8 as usize;
        
        // CRITICAL: Reserve boot page tables (0x40081000-0x40085000)
        // These are L1_TABLE_KERNEL, L1_TABLE_USER, L2_TABLE_KERNEL, L2_TABLE_USER used by MMU
        // They are in .data.boot/.bss.boot sections and MUST NOT be reused
        const BOOT_PAGE_TABLES_START: usize = 0x40081000;
        const BOOT_PAGE_TABLES_END: usize = 0x40085000;
        
        // Start allocating after both kernel end and boot page tables
        let reserved_end = core::cmp::max(k_end, BOOT_PAGE_TABLES_END);
        let start = core::cmp::max(reserved_end, boot.phys_mem_start as usize);
        let end = (boot.phys_mem_start as usize).saturating_add(boot.phys_mem_size as usize);
        REGION_START = (start + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        REGION_END = end & !(PAGE_SIZE - 1);
        
        crate::error!("PHYS ALLOC INIT: phys_mem_start=0x{:X}, phys_mem_size=0x{:X}",
                     boot.phys_mem_start, boot.phys_mem_size);
        crate::error!("PHYS ALLOC INIT: k_end=0x{:X}, pt_end=0x{:X}, reserved_end=0x{:X}",
                     k_end, BOOT_PAGE_TABLES_END, reserved_end);
        crate::error!("PHYS ALLOC INIT: start=0x{:X}, end=0x{:X}",
                     start, end);
        crate::error!("PHYS ALLOC INIT: REGION_START=0x{:X}, REGION_END=0x{:X}",
                     REGION_START, REGION_END);
        crate::error!("PHYS ALLOC INIT: REGION_SIZE=0x{:X} pages",
                     (REGION_END - REGION_START) / PAGE_SIZE);
        for o in 0..=MAX_ORDER {
            FREE_COUNT[o] = 0;
        }
        let mut cur = REGION_START;
        while cur < REGION_END {
            let mut o = MAX_ORDER;
            loop {
                let size = (PAGE_SIZE << o);
                if cur.is_multiple_of(size) && cur + size <= REGION_END {
                    push_block(o, cur);
                    cur += size;
                    break;
                }
                if o == 0 {
                    break;
                } else {
                    o -= 1;
                }
            }
        }
    }
    crate::info!("memory buddy allocator ready");
}

pub fn alloc_pages(count: usize) -> Option<PhysAddr> {
    let need_order = order_for_pages(count);
    unsafe {
        ALLOC_CALLS = ALLOC_CALLS.saturating_add(1);
        let mut o = need_order;
        while o <= MAX_ORDER {
            if let Some(addr) = pop_block(o) {
                let mut cur_order = o;
                while cur_order > need_order {
                    let size = PAGE_SIZE << (cur_order - 1);
                    let buddy = addr + size;
                    push_block(cur_order - 1, buddy);
                    cur_order -= 1;
                }
                return Some(addr);
            }
            o += 1;
        }
        None
    }
}

pub fn free_pages(addr: PhysAddr, count: usize) {
    unsafe {
        FREE_CALLS = FREE_CALLS.saturating_add(1);
        // Debug logging
        if addr < REGION_START || addr + (PAGE_SIZE << order_for_pages(count)) > REGION_END {
            crate::error!("PHYS ALLOC free_pages: addr=0x{:X}, count={}, order={}, REGION_START=0x{:X}, REGION_END=0x{:X} - IGNORING",
                addr, count, order_for_pages(count), REGION_START, REGION_END);
            return; // Ignore invalid free requests
        }
        let mut order = order_for_pages(count);
        let mut a = addr;
        loop {
            let size = PAGE_SIZE << order;
            let buddy = a ^ size;
            if remove_block(order, buddy) {
                a = core::cmp::min(a, buddy);
                order += 1;
                COALESCE_EVENTS = COALESCE_EVENTS.saturating_add(1);
                if order > MAX_ORDER {
                    break;
                }
                continue;
            } else {
                push_block(order, a);
                break;
            }
        }
    }
}

pub struct BuddyStats {
    pub free_counts: [usize; MAX_ORDER + 1],
    pub total_free_pages: usize,
    pub alloc_calls: usize,
    pub free_calls: usize,
    pub coalesce_events: usize,
    pub fragmentation_index: f32,
}

pub fn stats() -> BuddyStats {
    unsafe {
        let mut total_pages = 0usize;
        let mut weighted_small = 0f32;
        let mut weighted_total = 0f32;
        let mut counts = [0usize; MAX_ORDER + 1];
        for o in 0..=MAX_ORDER {
            counts[o] = FREE_COUNT[o];
            let pages = FREE_COUNT[o] * (1usize << o);
            total_pages += pages;
            let weight = if o == 0 { 1.0 } else { 1.0 / (o as f32 + 1.0) };
            weighted_small += (FREE_COUNT[o] as f32) * weight;
            weighted_total += FREE_COUNT[o] as f32;
        }
        let frag = if weighted_total > 0.0 { weighted_small / weighted_total } else { 0.0 };
        BuddyStats {
            free_counts: counts,
            total_free_pages: total_pages,
            alloc_calls: ALLOC_CALLS,
            free_calls: FREE_CALLS,
            coalesce_events: COALESCE_EVENTS,
            fragmentation_index: frag,
        }
    }
}

pub fn check_invariants() -> bool {
    unsafe {
        // Validate alignment and region bounds
        for o in 0..=MAX_ORDER {
            let size = PAGE_SIZE << o;
            for i in 0..FREE_COUNT[o] {
                let a = FREE_LIST[o][i];
                if !a.is_multiple_of(size) { return false; }
                if a < REGION_START || a + size > REGION_END { return false; }
            }
        }
        true
    }
}
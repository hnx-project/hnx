use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

static GICD_BASE: AtomicUsize = AtomicUsize::new(0x0800_0000);
static GICC_BASE: AtomicUsize = AtomicUsize::new(0x0801_0000);

// GICv2 Priority Levels (0 = highest, 255 = lowest)
pub const PRIORITY_CRITICAL: u8 = 0x00;   // System critical (NMI-like)
pub const PRIORITY_VERY_HIGH: u8 = 0x20;  // Timer, scheduling
pub const PRIORITY_HIGH: u8 = 0x40;       // Fast devices (UART, network)
pub const PRIORITY_NORMAL: u8 = 0x80;     // Standard devices
pub const PRIORITY_LOW: u8 = 0xC0;        // Background tasks
pub const PRIORITY_LOWEST: u8 = 0xE0;     // Deferred work

// Binary point values for preemption control
pub const BINARY_POINT_PREEMPT_ALL: u8 = 0b000;  // All bits for grouping
pub const BINARY_POINT_PREEMPT_NONE: u8 = 0b111; // No preemption

// Priority mask (allow all priorities >= this value)
pub const PRIORITY_MASK_NONE: u8 = 0xFF;  // Allow all interrupts
pub const PRIORITY_MASK_HIGH: u8 = 0x40;  // Only high priority and above

pub fn init() {
    unsafe {
        let gicd = GICD_BASE.load(Ordering::Relaxed) as *mut u32;
        let gicc = GICC_BASE.load(Ordering::Relaxed) as *mut u32;
        
        crate::info!("GICv2: Initializing interrupt priority management");
        
        // Enable distributor
        ptr::write_volatile(gicd.add(0), 1u32);
        
        // Configure binary point for preemption (4-bit group, 4-bit sub-priority)
        // GICC_BPR at offset 0x08
        ptr::write_volatile(gicc.add(2), 0b011u32);
        crate::info!("GICv2: Binary point set to 0b011 (4-bit group, 4-bit sub-priority)");
        
        // Set priority mask to allow all interrupts
        // GICC_PMR at offset 0x04
        ptr::write_volatile(gicc.add(1), 0xFFu32);
        crate::info!("GICv2: Priority mask set to 0xFF (allow all interrupts)");
        
        // Enable CPU interface
        ptr::write_volatile(gicc.add(0), 1u32);
        
        // Set default priorities for all interrupts to PRIORITY_NORMAL
        // GICD_IPRIORITYn starts at offset 0x400
        for i in 0..255 {
            ptr::write_volatile(gicd.add(0x100 + i), 
                (PRIORITY_NORMAL as u32) * 0x01010101);  // Set all 4 bytes
        }
        crate::info!("GICv2: Default priorities set to {} (NORMAL)", PRIORITY_NORMAL);
        
        // Configure system interrupt priorities
        configure_system_priorities();
        
        // Enable timer interrupt (IRQ 30)
        let isenabler0 = gicd.add(0x40);
        let v = ptr::read_volatile(isenabler0);
        ptr::write_volatile(isenabler0, v | (1u32 << 30));
        
        // Clear pending timer interrupt
        let icpend0 = gicd.add(0xA0);
        ptr::write_volatile(icpend0, 1u32 << 30);
        
        crate::info!("GICv2: Interrupt priority management initialized successfully");
    }
}

/// Configure priorities for known system interrupts
fn configure_system_priorities() {
    // Timer interrupt (IRQ 30) - very high priority for scheduling
    set_interrupt_priority(30, PRIORITY_VERY_HIGH);
    crate::info!("GICv2: IRQ 30 (timer) priority set to {} (VERY_HIGH)", PRIORITY_VERY_HIGH);
    
    // PL011 UART interrupt (IRQ 33) - high priority for responsiveness
    set_interrupt_priority(33, PRIORITY_HIGH);
    crate::info!("GICv2: IRQ 33 (UART) priority set to {} (HIGH)", PRIORITY_HIGH);
    
    // Additional device-specific priorities can be added here as needed
}

pub fn enable_irq(irq: u32) {
    unsafe {
        let gicd = GICD_BASE.load(Ordering::Relaxed) as *mut u32;
        let idx = (irq / 32) as usize;
        let bit = 1u32 << (irq % 32);
        let isenabler = gicd.add(0x40 + idx);
        let v = ptr::read_volatile(isenabler);
        ptr::write_volatile(isenabler, v | bit);
    }
}

pub fn read_iar() -> u32 {
    unsafe { ptr::read_volatile((GICC_BASE.load(Ordering::Relaxed) as *mut u32).add(3)) }
}

pub fn write_eoi(iar: u32) {
    unsafe { ptr::write_volatile((GICC_BASE.load(Ordering::Relaxed) as *mut u32).add(4), iar) }
}

pub struct GicV2Controller;

impl crate::arch::common::traits::InterruptController for GicV2Controller {
    fn init() { init() }
    fn enable_irq(irq: u32) { enable_irq(irq) }
    fn disable_irq(irq: u32) {
        unsafe {
            let gicd = GICD_BASE.load(Ordering::Relaxed) as *mut u32;
            let idx = (irq / 32) as usize;
            let bit = 1u32 << (irq % 32);
            let icenabler = gicd.add(0x60 + idx);
            let v = ptr::read_volatile(icenabler);
            ptr::write_volatile(icenabler, v | bit);
        }
    }
    fn ack_irq(irq: u32) {
        unsafe { ptr::write_volatile((GICC_BASE.load(Ordering::Relaxed) as *mut u32).add(4), irq) }
    }
}

pub static GICV2: GicV2Controller = GicV2Controller;

pub fn set_bases(gicd: usize, gicc: usize) {
    GICD_BASE.store(gicd, Ordering::Relaxed);
    GICC_BASE.store(gicc, Ordering::Relaxed);
}

/// Set priority for a specific interrupt
/// Priority: 0 = highest, 255 = lowest
pub fn set_interrupt_priority(irq: u32, priority: u8) {
    unsafe {
        let gicd = GICD_BASE.load(Ordering::Relaxed) as *mut u32;
        // GICD_IPRIORITYn starts at offset 0x400
        // Each register holds 4 priority values (8 bits each)
        let offset = (irq / 4) as usize;
        let shift = (irq % 4) * 8;
        
        let reg_addr = gicd.add(0x100 + offset);  // 0x400 / 4 = 0x100
        let mut val = ptr::read_volatile(reg_addr);
        val &= !(0xFF << shift);  // Clear old priority
        val |= (priority as u32) << shift;  // Set new priority
        ptr::write_volatile(reg_addr, val);
    }
}

/// Get priority for a specific interrupt (for debugging)
pub fn get_interrupt_priority(irq: u32) -> u8 {
    unsafe {
        let gicd = GICD_BASE.load(Ordering::Relaxed) as *mut u32;
        let offset = (irq / 4) as usize;
        let shift = (irq % 4) * 8;
        let reg_addr = gicd.add(0x100 + offset);
        
        let val = ptr::read_volatile(reg_addr);
        ((val >> shift) & 0xFF) as u8
    }
}

/// Configure binary point for preemption control
pub fn set_binary_point(bp: u8) {
    unsafe {
        let gicc = GICC_BASE.load(Ordering::Relaxed) as *mut u32;
        // GICC_BPR at offset 0x08 (word offset 2)
        ptr::write_volatile(gicc.add(2), (bp & 0x7) as u32);
    }
}

/// Set priority mask (minimum priority to accept)
pub fn set_priority_mask(mask: u8) {
    unsafe {
        let gicc = GICC_BASE.load(Ordering::Relaxed) as *mut u32;
        // GICC_PMR at offset 0x04 (word offset 1)
        ptr::write_volatile(gicc.add(1), mask as u32);
    }
}

/// Get current running priority
pub fn get_running_priority() -> u8 {
    unsafe {
        let gicc = GICC_BASE.load(Ordering::Relaxed) as *mut u32;
        // GICC_RPR at offset 0x14 (word offset 5)
        ptr::read_volatile(gicc.add(5)) as u8
    }
}

/// Get highest pending interrupt priority
pub fn get_highest_pending_priority() -> u8 {
    unsafe {
        let gicc = GICC_BASE.load(Ordering::Relaxed) as *mut u32;
        // GICC_HPPIR at offset 0x18 (word offset 6)
        let val = ptr::read_volatile(gicc.add(6));
        ((val >> 8) & 0xFF) as u8  // Extract priority field
    }
}
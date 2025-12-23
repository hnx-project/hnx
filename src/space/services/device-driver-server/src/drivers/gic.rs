//! GIC Driver Implementation for User-space Device Driver Server
//!
//! This module implements a user-space GIC driver that communicates with the kernel
//! through IPC to handle interrupt controller operations.

extern crate alloc;

use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

// GIC register offsets
const GICD_OFFSET: usize = 0x0000;
const GICC_OFFSET: usize = 0x1000;

// GICD registers
const GICD_CTLR: usize = 0x000;
const GICD_ISENABLER: usize = 0x100;
const GICD_ICENABLER: usize = 0x180;
const GICD_IPRIORITYR: usize = 0x400;

// GICC registers
const GICC_CTLR: usize = 0x000;
const GICC_PMR: usize = 0x004;
const GICC_BPR: usize = 0x008;
const GICC_IAR: usize = 0x00C;
const GICC_EOIR: usize = 0x010;

/// GICv2 driver implementation
pub struct GicV2Driver {
    gicd_base: AtomicUsize,
    gicc_base: AtomicUsize,
}

impl GicV2Driver {
    /// Create a new GICv2 driver instance
    pub const fn new() -> Self {
        Self {
            gicd_base: AtomicUsize::new(0x0800_0000), // Default GICD base address
            gicc_base: AtomicUsize::new(0x0801_0000), // Default GICC base address
        }
    }
    
    /// Initialize the GIC hardware
    pub fn init(&self) {
        unsafe {
            let gicd = self.gicd_base.load(Ordering::Relaxed) as *mut u32;
            let gicc = self.gicc_base.load(Ordering::Relaxed) as *mut u32;
            
            crate::println!("GICv2: Initializing interrupt controller");
            
            // Enable distributor
            ptr::write_volatile(gicd.add(GICD_CTLR / 4), 1u32);
            
            // Configure binary point for preemption (4-bit group, 4-bit sub-priority)
            ptr::write_volatile(gicc.add(GICC_BPR / 4), 0b011u32);
            crate::println!("GICv2: Binary point set to 0b011 (4-bit group, 4-bit sub-priority)");
            
            // Set priority mask to allow all interrupts
            ptr::write_volatile(gicc.add(GICC_PMR / 4), 0xFFu32);
            crate::println!("GICv2: Priority mask set to 0xFF (allow all interrupts)");
            
            // Enable CPU interface
            ptr::write_volatile(gicc.add(GICC_CTLR / 4), 1u32);
            
            // Set default priorities for all interrupts to normal priority
            for i in 0..255 {
                ptr::write_volatile(gicd.add(GICD_IPRIORITYR / 4 + i), 0x80808080); // Normal priority
            }
            crate::println!("GICv2: Default priorities set to normal");
            
            crate::println!("GICv2: Interrupt controller initialized successfully");
        }
    }
    
    /// Enable an interrupt
    pub fn enable_irq(&self, irq: u32) {
        unsafe {
            let gicd = self.gicd_base.load(Ordering::Relaxed) as *mut u32;
            let idx = (irq / 32) as usize;
            let bit = 1u32 << (irq % 32);
            let isenabler = gicd.add(GICD_ISENABLER / 4 + idx);
            let v = ptr::read_volatile(isenabler);
            ptr::write_volatile(isenabler, v | bit);
        }
    }
    
    /// Disable an interrupt
    pub fn disable_irq(&self, irq: u32) {
        unsafe {
            let gicd = self.gicd_base.load(Ordering::Relaxed) as *mut u32;
            let idx = (irq / 32) as usize;
            let bit = 1u32 << (irq % 32);
            let icenabler = gicd.add(GICD_ICENABLER / 4 + idx);
            let v = ptr::read_volatile(icenabler);
            ptr::write_volatile(icenabler, v | bit);
        }
    }
    
    /// Read the interrupt acknowledge register
    pub fn read_iar(&self) -> u32 {
        unsafe {
            let gicc = self.gicc_base.load(Ordering::Relaxed) as *mut u32;
            ptr::read_volatile(gicc.add(GICC_IAR / 4))
        }
    }
    
    /// Write to the end of interrupt register
    pub fn write_eoi(&self, iar: u32) {
        unsafe {
            let gicc = self.gicc_base.load(Ordering::Relaxed) as *mut u32;
            ptr::write_volatile(gicc.add(GICC_EOIR / 4), iar)
        }
    }
    
    /// Set interrupt priority
    pub fn set_interrupt_priority(&self, irq: u32, priority: u8) {
        unsafe {
            let gicd = self.gicd_base.load(Ordering::Relaxed) as *mut u32;
            let offset = (irq / 4) as usize;
            let shift = (irq % 4) * 8;
            
            let reg_addr = gicd.add(GICD_IPRIORITYR / 4 + offset);
            let mut val = ptr::read_volatile(reg_addr);
            val &= !(0xFF << shift);  // Clear old priority
            val |= (priority as u32) << shift;  // Set new priority
            ptr::write_volatile(reg_addr, val);
        }
    }
    
    /// Set the base addresses of the GIC hardware
    pub fn set_bases(&self, gicd: usize, gicc: usize) {
        self.gicd_base.store(gicd, Ordering::Relaxed);
        self.gicc_base.store(gicc, Ordering::Relaxed);
    }
}

/// Global GICv2 driver instance
pub static GICV2_DRIVER: GicV2Driver = GicV2Driver::new();
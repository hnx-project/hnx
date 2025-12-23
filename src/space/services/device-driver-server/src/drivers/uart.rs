//! UART Driver Implementation for User-space Device Driver Server
//!
//! This module implements a user-space UART driver that communicates with the kernel
//! through IPC to handle UART operations.

extern crate alloc;

use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

// UART register offsets
const PL011_DR: usize = 0x000;
const PL011_FR: usize = 0x018;
const PL011_IBRD: usize = 0x024;
const PL011_FBRD: usize = 0x028;
const PL011_LCR_H: usize = 0x02C;
const PL011_CR: usize = 0x030;
const PL011_ICR: usize = 0x044;

// UART flags
const PL011_FR_RXFE: u32 = 1 << 4;
const PL011_FR_TXFF: u32 = 1 << 5;
const PL011_FR_BUSY: u32 = 1 << 3;

/// PL011 UART driver implementation
pub struct Pl011Uart {
    base_address: AtomicUsize,
}

impl Pl011Uart {
    /// Create a new PL011 UART driver instance
    pub const fn new() -> Self {
        Self {
            base_address: AtomicUsize::new(0x0900_0000), // Default base address
        }
    }
    
    /// Initialize the UART hardware
    pub fn init(&self) {
        unsafe {
            let base = self.base_address.load(Ordering::Relaxed) as *mut u32;
            let cr = ptr::read_volatile(base.add(PL011_CR / 4));
            if cr == 0 {
                ptr::write_volatile(base.add(PL011_CR / 4), 0);
                ptr::write_volatile(base.add(PL011_IBRD / 4), 13);
                ptr::write_volatile(base.add(PL011_FBRD / 4), 2);
                ptr::write_volatile(base.add(PL011_LCR_H / 4), 0x70);
                ptr::write_volatile(base.add(PL011_CR / 4), 0x301);
                ptr::write_volatile(base.add(PL011_ICR / 4), 0x7FF);
            }
        }
    }
    
    /// Write a character to the UART
    pub fn putc(&self, c: u8) {
        unsafe {
            let base = self.base_address.load(Ordering::Relaxed) as *mut u32;
            let mut ch = c;
            if ch < 0x20 && ch != b'\n' && ch != b'\t' { return; }
            if ch >= 0x80 { return; }
            if ch == b'\n' {
                while (ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_TXFF) != 0 { core::hint::spin_loop(); }
                while (ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_BUSY) != 0 { core::hint::spin_loop(); }
                ptr::write_volatile(base.add(PL011_DR / 4), b'\r' as u32);
            }
            while (ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_TXFF) != 0 { core::hint::spin_loop(); }
            while (ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_BUSY) != 0 { core::hint::spin_loop(); }
            ptr::write_volatile(base.add(PL011_DR / 4), ch as u32);
        }
    }
    
    /// Read a character from the UART
    pub fn getc(&self) -> Option<u8> {
        unsafe {
            let base = self.base_address.load(Ordering::Relaxed) as *mut u32;
            if ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_RXFE != 0 {
                None
            } else {
                Some(ptr::read_volatile(base.add(PL011_DR / 4)) as u8)
            }
        }
    }
    
    /// Set the base address of the UART hardware
    pub fn set_base(&self, addr: usize) {
        self.base_address.store(addr, Ordering::Relaxed);
    }
}

/// Global PL011 UART instance
pub static PL011_UART: Pl011Uart = Pl011Uart::new();
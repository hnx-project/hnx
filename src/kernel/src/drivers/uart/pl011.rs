use core::ptr;
use crate::drivers::uart::r#trait::UartDriver;
use core::sync::atomic::{AtomicUsize, Ordering};

static PL011_BASE: AtomicUsize = AtomicUsize::new(0x0900_0000);
const PL011_DR: usize = 0x000;
const PL011_FR: usize = 0x018;
const PL011_IBRD: usize = 0x024;
const PL011_FBRD: usize = 0x028;
const PL011_LCR_H: usize = 0x02C;
const PL011_CR: usize = 0x030;
const PL011_ICR: usize = 0x044;
const PL011_FR_RXFE: u32 = 1 << 4;
const PL011_FR_TXFF: u32 = 1 << 5;
const PL011_FR_BUSY: u32 = 1 << 3;

pub struct Pl011Uart;

impl Pl011Uart {
    pub const fn new() -> Self { Pl011Uart }
}

impl UartDriver for Pl011Uart {
    fn init(&self) {
        unsafe {
            let base = PL011_BASE.load(Ordering::Relaxed) as *mut u32;
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

    fn putc(&self, c: u8) {
        unsafe {
            let b0 = PL011_BASE.load(Ordering::Relaxed) as *mut u32;
            let mut ch = c;
            if ch < 0x20 && ch != b'\n' && ch != b'\t' { return; }
            if ch >= 0x80 { return; }
            if ch == b'\n' {
                while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_TXFF) != 0 { core::hint::spin_loop(); }
                while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_BUSY) != 0 { core::hint::spin_loop(); }
                ptr::write_volatile(b0.add(PL011_DR / 4), b'\r' as u32);
            }
            while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_TXFF) != 0 { core::hint::spin_loop(); }
            while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_BUSY) != 0 { core::hint::spin_loop(); }
            ptr::write_volatile(b0.add(PL011_DR / 4), ch as u32);
        }
    }

    fn getc(&self) -> Option<u8> {
        unsafe {
            let base = PL011_BASE.load(Ordering::Relaxed) as *mut u32;
            if ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_RXFE != 0 {
                None
            } else {
                Some(ptr::read_volatile(base.add(PL011_DR / 4)) as u8)
            }
        }
    }
}

pub static PL011: Pl011Uart = Pl011Uart::new();

pub fn set_base(addr: usize) {
    PL011_BASE.store(addr, Ordering::Relaxed);
}

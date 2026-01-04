use core::ptr;
const PL011_BASE: usize = 0x0900_0000;
const PL011_DR: usize = 0x00;
const PL011_FR: usize = 0x18;
const PL011_IBRD: usize = 0x24;
const PL011_FBRD: usize = 0x28;
const PL011_LCR_H: usize = 0x2c;
const PL011_CR: usize = 0x30;
const PL011_ICR: usize = 0x44;
const PL011_FR_TXFF: u32 = 1 << 5;
const PL011_FR_RXFE: u32 = 1 << 4;
const PL011_FR_BUSY: u32 = 1 << 3;

pub fn init() {
    unsafe {
        let base = PL011_BASE as *mut u32;
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

pub fn putc(c: u8) {
    unsafe {
        let b0 = PL011_BASE as *mut u32;
        let mut ch = c;
        if ch < 0x20 && ch != b'\n' && ch != b'\t' {
            return;
        }
        if ch >= 0x80 {
            return;
        }
        if ch == b'\n' {
            while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_TXFF) != 0 {
                core::hint::spin_loop();
            }
            while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_BUSY) != 0 {
                core::hint::spin_loop();
            }
            ptr::write_volatile(b0.add(PL011_DR / 4), b'\r' as u32);
        }
        while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_TXFF) != 0 {
            core::hint::spin_loop();
        }
        while (ptr::read_volatile(b0.add(PL011_FR / 4)) & PL011_FR_BUSY) != 0 {
            core::hint::spin_loop();
        }
        ptr::write_volatile(b0.add(PL011_DR / 4), ch as u32);
    }
}

pub fn getc() -> Option<u8> {
    unsafe {
        let base = PL011_BASE as *mut u32;
        if ptr::read_volatile(base.add(PL011_FR / 4)) & PL011_FR_RXFE != 0 {
            None
        } else {
            Some(ptr::read_volatile(base.add(PL011_DR / 4)) as u8)
        }
    }
}

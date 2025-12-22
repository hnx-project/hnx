// drivers/bus/mod.rs
use crate::arch::common::mmu::ArchType;
use crate::arch::common::traits::Result;

pub struct Detected {
    pub pl011_base: Option<usize>,
    pub gicd_base: Option<usize>,
    pub gicc_base: Option<usize>,
    pub other_mmio: [Option<usize>; 8],
}

fn be32(x: u32) -> u32 { u32::from_be(x) }
fn be64(x: u64) -> u64 { u64::from_be(x) }

#[allow(clippy::manual_is_multiple_of)]
pub fn detect(dtb_ptr: usize, arch: ArchType) -> Detected {
    let mut pl011: Option<usize> = None;
    let mut gicd: Option<usize> = None;
    let mut gicc: Option<usize> = None;
    let mut others: [Option<usize>; 8] = [None; 8];
    let mut other_pos: usize = 0;
    if dtb_ptr == 0 { return Detected { pl011_base: pl011, gicd_base: gicd, gicc_base: gicc, other_mmio: others }; }
    unsafe {
        let hdr = core::slice::from_raw_parts(dtb_ptr as *const u8, 40);
        let magic = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        if magic != 0xD00D_FEEE { return Detected { pl011_base: pl011, gicd_base: gicd, gicc_base: gicc, other_mmio: others }; }
        let off_struct = u32::from_be_bytes([hdr[8], hdr[9], hdr[10], hdr[11]]) as usize;
        let off_strings = u32::from_be_bytes([hdr[12], hdr[13], hdr[14], hdr[15]]) as usize;
        let size_struct = u32::from_be_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]) as usize;
        let blob = core::slice::from_raw_parts(dtb_ptr as *const u8, off_struct + size_struct);
        let strings = core::slice::from_raw_parts((dtb_ptr + off_strings) as *const u8, 1 << 20);
        let mut i = off_struct;
        let mut compat_pl011 = false;
        let mut compat_gic = false;
        let mut compat_virtio = false;
        let mut reg_tmp: [u8; 32] = [0; 32];
        while i + 4 <= blob.len() {
            let token = u32::from_be_bytes([blob[i], blob[i + 1], blob[i + 2], blob[i + 3]]);
            i += 4;
            if token == 1 {
                while i < blob.len() && blob[i] != 0 { i += 1; }
                while i % 4 != 0 { i += 1; }
                compat_pl011 = false;
                compat_gic = false;
                compat_virtio = false;
            } else if token == 2 {
                compat_pl011 = false;
                compat_gic = false;
                compat_virtio = false;
            } else if token == 3 {
                if i + 8 > blob.len() { break; }
                let nameoff = u32::from_be_bytes([blob[i], blob[i + 1], blob[i + 2], blob[i + 3]]) as usize;
                let len = u32::from_be_bytes([blob[i + 4], blob[i + 5], blob[i + 6], blob[i + 7]]) as usize;
                i += 8;
                if i + len > blob.len() { break; }
                let mut pname_end = nameoff;
                while pname_end < strings.len() && strings[pname_end] != 0 { pname_end += 1; }
                let pname = core::str::from_utf8_unchecked(&strings[nameoff..pname_end]);
                if pname == "compatible" {
                    let s = core::str::from_utf8_unchecked(&blob[i..i + len]);
                    if s.contains("arm,pl011") { compat_pl011 = true; }
                    if s.contains("arm,gic") || s.contains("arm,gic-400") { compat_gic = true; }
                    if s.contains("virtio,mmio") { compat_virtio = true; }
                } else if pname == "reg" {
                    let cplen = len.min(32);
                    reg_tmp[..cplen].copy_from_slice(&blob[i..i + cplen]);
                    if compat_pl011 && len >= 8 && pl011.is_none() {
                        let b = be64(u64::from_be_bytes([reg_tmp[0], reg_tmp[1], reg_tmp[2], reg_tmp[3], reg_tmp[4], reg_tmp[5], reg_tmp[6], reg_tmp[7]]));
                        pl011 = Some(b as usize);
                    }
                    if compat_gic && len >= 16 {
                        let b0 = be64(u64::from_be_bytes([
                            reg_tmp[0], reg_tmp[1], reg_tmp[2], reg_tmp[3],
                            reg_tmp[4], reg_tmp[5], reg_tmp[6], reg_tmp[7],
                        ]));
                        let b1 = be64(u64::from_be_bytes([
                            reg_tmp[8], reg_tmp[9], reg_tmp[10], reg_tmp[11],
                            reg_tmp[12], reg_tmp[13], reg_tmp[14], reg_tmp[15],
                        ]));
                        gicd = Some(b0 as usize);
                        gicc = Some(b1 as usize);
                    }
                    if compat_virtio && len >= 8 {
                        let b = be64(u64::from_be_bytes([
                            reg_tmp[0], reg_tmp[1], reg_tmp[2], reg_tmp[3],
                            reg_tmp[4], reg_tmp[5], reg_tmp[6], reg_tmp[7],
                        ]));
                        if other_pos < others.len() { others[other_pos] = Some(b as usize); other_pos += 1; }
                    }
                }
                i += len;
                while i % 4 != 0 { i += 1; }
            } else if token == 9 {
                break;
            } else {
                while i % 4 != 0 { i += 1; }
            }
        }
    }
    Detected { pl011_base: pl011, gicd_base: gicd, gicc_base: gicc, other_mmio: others }
}

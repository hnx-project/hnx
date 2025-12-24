// 项目关联 hnx-toolchain => ohlink 工具链
#![allow(dead_code)]

use crate::arch::common::mmu::MmuFlags;
use crate::memory::physical::alloc_pages;
use crate::memory::virtual_::{create_user_l1, map_in_pt};

const OHLINK_MAGIC_64: u32 = 0x0f112234;
const INITRD_MAGIC: [u8; 4] = *b"IRDT";
const LC_SEGMENT_64: u32 = 0x19;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OhlinkHeader {
    pub magic: u32,
    pub cpu_type: u32,
    pub cpu_subtype: u32,
    pub file_type: u32,
    pub ncmds: u32,
    pub sizeofcmds: u32,
    pub flags: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy)]
pub struct Segment64 {
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub prot: u32,
}

pub struct UserProgram {
    pub entry: usize,
    pub pt_base: usize,
    pub user_sp: usize,
}

pub fn find_ohlink<'a>(data: &'a [u8], path: &str) -> Option<&'a [u8]> {
    if data.len() >= 8 && data[0..4] == INITRD_MAGIC {
        let count = u32::from_le_bytes(data[4..8].try_into().ok()?);
        let mut off = 8usize;
        for _ in 0..count {
            if off + 64 + 8 > data.len() {
                break;
            }
            let name_bytes = &data[off..off + 64];
            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
            let name = core::str::from_utf8(&name_bytes[..name_end]).ok()?;
            let offv = u32::from_le_bytes(data[off + 64..off + 68].try_into().ok()?);
            let sizev = u32::from_le_bytes(data[off + 68..off + 72].try_into().ok()?);
            if name == path {
                let s = offv as usize;
                let e = s.saturating_add(sizev as usize);
                if e <= data.len() {
                    return Some(&data[s..e]);
                }
            }
            off += 72;
        }
    }
    let magic = OHLINK_MAGIC_64.to_le_bytes();
    for i in 0..data.len().saturating_sub(4) {
        if data[i..i + 4] == magic {
            return Some(&data[i..]);
        }
    }
    None
}

pub fn load(binary: &[u8]) -> Result<UserProgram, ()> {
    if binary.len() < core::mem::size_of::<OhlinkHeader>() {
        return Err(());
    }
    let hdr: OhlinkHeader = unsafe { core::ptr::read_unaligned(binary.as_ptr() as *const _) };
    if hdr.magic != OHLINK_MAGIC_64 {
        return Err(());
    }
    let pt = create_user_l1().ok_or(())?;
    let mut off = core::mem::size_of::<OhlinkHeader>();
    let mut entry: usize = 0;
    for _ in 0..hdr.ncmds {
        if off + 8 > binary.len() {
            return Err(());
        }
        let cmd = u32::from_le_bytes([
            binary[off],
            binary[off + 1],
            binary[off + 2],
            binary[off + 3],
        ]);
        let cmdsize = u32::from_le_bytes([
            binary[off + 4],
            binary[off + 5],
            binary[off + 6],
            binary[off + 7],
        ]) as usize;
        if cmd == LC_SEGMENT_64 {
            if off + cmdsize > binary.len() {
                return Err(());
            }
            let (vmaddr, vmsize, fileoff, filesize, initprot, nsects, sec_base) = if cmdsize >= 72 {
                (
                    u64::from_le_bytes(binary[off + 24..off + 32].try_into().unwrap()),
                    u64::from_le_bytes(binary[off + 32..off + 40].try_into().unwrap()),
                    u64::from_le_bytes(binary[off + 40..off + 48].try_into().unwrap()),
                    u64::from_le_bytes(binary[off + 48..off + 56].try_into().unwrap()),
                    i32::from_le_bytes(binary[off + 64..off + 68].try_into().unwrap()) as u32,
                    u32::from_le_bytes(binary[off + 68..off + 72].try_into().unwrap()) as usize,
                    off + 72,
                )
            } else if cmdsize == 48 {
                (
                    u64::from_le_bytes(binary[off + 8..off + 16].try_into().unwrap()),
                    u64::from_le_bytes(binary[off + 16..off + 24].try_into().unwrap()),
                    u64::from_le_bytes(binary[off + 24..off + 32].try_into().unwrap()),
                    u64::from_le_bytes(binary[off + 32..off + 40].try_into().unwrap()),
                    u32::from_le_bytes(binary[off + 40..off + 44].try_into().unwrap()),
                    0usize,
                    0,
                )
            } else {
                return Err(());
            };
            let seg = Segment64 {
                vmaddr,
                vmsize,
                fileoff,
                filesize,
                prot: initprot,
            };
            map_segment(binary, pt, &seg)?;
            if entry == 0 {
                if nsects > 0 {
                    let mut sec_off = sec_base;
                    for _ in 0..nsects {
                        if sec_off + 80 > binary.len() {
                            break;
                        }
                        let addr = u64::from_le_bytes(
                            binary[sec_off + 32..sec_off + 40].try_into().unwrap(),
                        );
                        let sectname_bytes = &binary[sec_off..sec_off + 16];
                        let name_end = sectname_bytes.iter().position(|&b| b == 0).unwrap_or(16);
                        let name = core::str::from_utf8(&sectname_bytes[..name_end]).unwrap_or("");
                        if name == "__text" {
                            entry = (seg.vmaddr + addr) as usize;
                            break;
                        }
                        sec_off += 80;
                    }
                }
                if entry == 0 {
                    entry = seg.vmaddr as usize;
                }
            }
        }
        off += cmdsize;
    }
    let stack_top = 0x8000_0000usize;
    let stack_pages = 16usize;
    for i in 0..stack_pages {
        let va = stack_top - i * 4096;
        if let Some(p) = alloc_pages(1) {
            unsafe {
                core::ptr::write_bytes((p as *mut u8), 0, 4096);
            }
            let flags = MmuFlags::READ
                .combine(MmuFlags::WRITE)
                .combine(MmuFlags::USER);
            map_in_pt(pt, va, p, flags);
            crate::arch::mmu::clean_dcache_range(p, 4096);
        } else {
            return Err(());
        }
    }
    crate::arch::mmu::flush_icache_all();
    Ok(UserProgram {
        entry,
        pt_base: pt,
        user_sp: stack_top,
    })
}

fn map_segment(binary: &[u8], pt: usize, seg: &Segment64) -> Result<(), ()> {
    let mut mapped = 0u64;
    while mapped < seg.vmsize {
        let va = seg.vmaddr + mapped;
        let pa = if let Some(p) = alloc_pages(1) {
            if mapped < seg.filesize {
                let fo = (seg.fileoff + mapped) as usize;
                let remaining = (seg.filesize - mapped) as usize;
                let to_copy = core::cmp::min(remaining, 4096);
                let avail = if binary.len() > fo {
                    binary.len() - fo
                } else {
                    0
                };
                let n = core::cmp::min(to_copy, avail);
                unsafe {
                    if n > 0 {
                        core::ptr::copy_nonoverlapping(binary.as_ptr().add(fo), p as *mut u8, n);
                    }
                    if n < 4096 {
                        core::ptr::write_bytes((p as *mut u8).add(n), 0, 4096 - n);
                    }
                }
            } else {
                unsafe {
                    core::ptr::write_bytes(p as *mut u8, 0, 4096);
                }
            }
            p
        } else {
            return Err(());
        };
        crate::arch::mmu::clean_dcache_range(pa, 4096);
        let mut flags = MmuFlags::READ.combine(MmuFlags::USER);
        // ohlink-format uses RWX bits in initprot: 1=R, 2=W, 4=X
        if seg.prot & 2 != 0 {
            flags = flags.combine(MmuFlags::WRITE);
        }
        if seg.prot & 4 != 0 {
            flags = flags.combine(MmuFlags::EXECUTE);
        }
        map_in_pt(pt, va as usize, pa, flags);
        mapped += 4096;
    }
    Ok(())
}

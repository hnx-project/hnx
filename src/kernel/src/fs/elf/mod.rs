#![allow(dead_code)]

use crate::arch::common::mmu::MmuFlags;
use crate::memory::physical::alloc_pages;
use crate::memory::virtual_::{create_user_l1, map_in_pt};
use crate::{error, info};

pub struct UserProgram {
    pub entry: usize,
    pub pt_base: usize,
    pub user_sp: usize,
}

#[repr(C)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[repr(C)]
struct Elf64Shdr {
    sh_name: u32,
    sh_type: u32,
    sh_flags: u64,
    sh_addr: u64,
    sh_offset: u64,
    sh_size: u64,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u64,
    sh_entsize: u64,
}

const PT_LOAD: u32 = 1;
const PF_X: u32 = 1 << 0;
const PF_W: u32 = 1 << 1;
const PF_R: u32 = 1 << 2;

// Section types
const SHT_SYMTAB: u32 = 2;
const SHT_DYNSYM: u32 = 11;
const SHT_RELA: u32 = 4;
const SHT_DYNAMIC: u32 = 6;

/// Check if an ELF binary is dynamically linked by looking for SHT_DYNAMIC sections
fn check_dynamic_binary(binary: &[u8], shoff: usize, shentsize: usize, shnum: usize) -> bool {
    for i in 0..shnum {
        let offset = shoff + i * shentsize;
        if offset + core::mem::size_of::<Elf64Shdr>() <= binary.len() {
            let shdr = unsafe { &*(binary[offset..].as_ptr() as *const Elf64Shdr) };
            if shdr.sh_type == SHT_DYNAMIC {
                return true;
            }
        }
    }
    false
}

// AArch64 relocation types (subset)
const R_AARCH64_ADR_PREL_PG_HI21: u32 = 0x101;
const R_AARCH64_ADD_ABS_LO12_NC: u32 = 0x102;

// AArch64 dynamic relocation types
const R_AARCH64_ABS64: u32 = 257;
const R_AARCH64_GLOB_DAT: u32 = 1025;
const R_AARCH64_JUMP_SLOT: u32 = 1026;
const R_AARCH64_RELATIVE: u32 = 1027;

#[repr(C)]
struct Elf64Sym {
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
}

#[repr(C)]
struct Elf64Rela {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

/// Dynamic entry structure
#[repr(C)]
struct Elf64Dyn {
    d_tag: u64,
    d_val: u64,  // Union with d_ptr
}

pub fn find_elf(data: &[u8]) -> Option<&[u8]> {
    let m = [0x7F, b'E', b'L', b'F'];
    let end = data.len().saturating_sub(4);
    let mut i = 0;
    while i < end {
        let b0 = unsafe { *data.get_unchecked(i) };
        if b0 == m[0] {
            let b1 = unsafe { *data.get_unchecked(i + 1) };
            let b2 = unsafe { *data.get_unchecked(i + 2) };
            let b3 = unsafe { *data.get_unchecked(i + 3) };
            if b1 == m[1] && b2 == m[2] && b3 == m[3] {
                return Some(&data[i..]);
            }
        }
        i += 1;
    }
    None
}

pub fn load(binary: &[u8]) -> Result<UserProgram, ()> {
    if binary.len() < 64 {
        return Err(());
    }
    if binary[0..4] != [0x7F, b'E', b'L', b'F'] {
        return Err(());
    }
    let e_entry = u64::from_le_bytes(binary[24..32].try_into().unwrap()) as usize;
    let e_phoff =
        usize::try_from(u64::from_le_bytes(binary[32..40].try_into().unwrap())).unwrap_or(0);
    let e_phentsize = u16::from_le_bytes(binary[54..56].try_into().unwrap()) as usize;
    let e_phnum = u16::from_le_bytes(binary[56..58].try_into().unwrap()) as usize;
    let e_shoff =
        usize::try_from(u64::from_le_bytes(binary[40..48].try_into().unwrap())).unwrap_or(0);
    let e_shentsize = u16::from_le_bytes(binary[58..60].try_into().unwrap()) as usize;
    let e_shnum = u16::from_le_bytes(binary[60..62].try_into().unwrap()) as usize;
    info!(
        "hdr: entry=0x{:016X} phoff=0x{:X} phnum={} shoff=0x{:X} shnum={}",
        e_entry, e_phoff, e_phnum, e_shoff, e_shnum
    );
    let pt = create_user_l1().ok_or(())?;
    let mut mapped_any = false;
    let mut page_map: [(usize, usize); 128] = [(0, 0); 128];
    let mut page_map_len: usize = 0;

    fn find_page(page_map: &[(usize, usize); 128], len: usize, base: usize) -> Option<usize> {
        let mut i = 0;
        while i < len {
            if page_map[i].0 == base {
                return Some(page_map[i].1);
            }
            i += 1;
        }
        None
    }

    // Check if this is a dynamically linked executable
    let is_dynamic = check_dynamic_binary(binary, e_shoff, e_shentsize, e_shnum);

    // Iterate program headers and map PT_LOAD segments
    let mut i = 0;
    while i < e_phnum {
        let off = e_phoff + i * e_phentsize;
        if off + 56 > binary.len() {
            error!("phdr out of range: off=0x{:X}", off);
            break;
        }
        let p_type = u32::from_le_bytes(binary[off..off + 4].try_into().unwrap());
        let p_flags = u32::from_le_bytes(binary[off + 4..off + 8].try_into().unwrap());
        let p_offset = u64::from_le_bytes(binary[off + 8..off + 16].try_into().unwrap()) as usize;
        let p_vaddr = u64::from_le_bytes(binary[off + 16..off + 24].try_into().unwrap()) as usize;
        let _p_paddr = u64::from_le_bytes(binary[off + 24..off + 32].try_into().unwrap()) as usize;
        let p_filesz = u64::from_le_bytes(binary[off + 32..off + 40].try_into().unwrap()) as usize;
        let p_memsz = u64::from_le_bytes(binary[off + 40..off + 48].try_into().unwrap()) as usize;
        let _p_align = u64::from_le_bytes(binary[off + 48..off + 56].try_into().unwrap()) as usize;
        if p_type == PT_LOAD {
            mapped_any = true;
            info!(
                "PT_LOAD: vaddr=0x{:016X} off=0x{:X} filesz=0x{:X} memsz=0x{:X} flags=0x{:X}",
                p_vaddr, p_offset, p_filesz, p_memsz, p_flags
            );
            let seg_start = p_vaddr & !0xFFFusize;
            let seg_end = (p_vaddr + p_memsz + 0xFFF) & !0xFFFusize;
            let mut va = seg_start;
            while va < seg_end {
                // ensure page allocated and mapped once
                let pa = if let Some(existing) = find_page(&page_map, page_map_len, va) {
                    existing
                } else if let Some(p) = alloc_pages(1) {
                    // map into user PT
                    let mut flags = MmuFlags::READ.combine(MmuFlags::USER);
                    if (p_flags & PF_W) != 0 {
                        flags = flags.combine(MmuFlags::WRITE);
                    }
                    if (p_flags & PF_X) != 0 {
                        flags = flags.combine(MmuFlags::EXECUTE);
                    }
                    map_in_pt(pt, va, p, flags);
                    // record mapping
                    if page_map_len < page_map.len() {
                        page_map[page_map_len] = (va, p);
                        page_map_len += 1;
                    }
                    p
                } else {
                    error!("alloc page failed at va=0x{:016X}", va);
                    return Err(());
                };
                // mapping detail suppressed

                // fill page contents
                unsafe {
                    // map this page into kernel VA for coherent writes
                    let kva = crate::memory::virtual_::KERNEL_BASE + pa;
                    crate::memory::virtual_::map(kva, pa, MmuFlags::READ.combine(MmuFlags::WRITE));
                    let dst = kva as *mut u8;
                    // destination offset in page (first page may start at non-zero offset)
                    let dst_off = if va == seg_start {
                        p_vaddr & 0xFFFusize
                    } else {
                        0
                    };
                    // segment memory offset corresponding to this page
                    let seg_mem_off = va.saturating_sub(p_vaddr);
                    let file_off = p_offset + seg_mem_off;
                    if seg_mem_off < p_memsz {
                        let remain_in_seg = p_memsz - seg_mem_off;
                        let to_copy = core::cmp::min(4096 - dst_off, remain_in_seg);
                        let mut file_avail = if seg_mem_off < p_filesz {
                            core::cmp::min(to_copy, p_filesz - seg_mem_off)
                        } else {
                            0
                        };
                        if file_off >= binary.len() {
                            file_avail = 0;
                        }
                        if file_avail > 0 {
                            let max_avail = binary.len().saturating_sub(file_off);
                            let n = core::cmp::min(file_avail, max_avail);
                            core::ptr::copy_nonoverlapping(
                                binary.as_ptr().add(file_off),
                                dst.add(dst_off),
                                n,
                            );
                            file_avail = n;
                        }
                        if file_avail < to_copy {
                            core::ptr::write_bytes(
                                dst.add(dst_off + file_avail),
                                0,
                                to_copy - file_avail,
                            );
                        }
                        if dst_off + to_copy < 4096 {
                            core::ptr::write_bytes(
                                dst.add(dst_off + to_copy),
                                0,
                                4096 - (dst_off + to_copy),
                            );
                        }
                    }
                }
                crate::arch::mmu::clean_dcache_range(
                    crate::memory::virtual_::KERNEL_BASE + pa,
                    4096,
                );
                va += 4096;
            }
        }
        i += 1;
    }

    if !mapped_any {
        let mut smapped = false;
        let mut j = 0;
        while j < e_shnum {
            let off = e_shoff + j * e_shentsize;
            if off + 64 > binary.len() {
                break;
            }
            let sh_type = u32::from_le_bytes(binary[off + 4..off + 8].try_into().unwrap());
            let sh_flags = u64::from_le_bytes(binary[off + 8..off + 16].try_into().unwrap());
            let sh_addr =
                u64::from_le_bytes(binary[off + 24..off + 32].try_into().unwrap()) as usize;
            let sh_offset =
                u64::from_le_bytes(binary[off + 32..off + 40].try_into().unwrap()) as usize;
            let sh_size =
                u64::from_le_bytes(binary[off + 40..off + 48].try_into().unwrap()) as usize;
            if sh_size > 0 && sh_addr != 0 {
                let seg_start = sh_addr & !0xFFFusize;
                let seg_end = (sh_addr + sh_size + 0xFFF) & !0xFFFusize;
                let mut va = seg_start;
                while va < seg_end {
                    let pa = if let Some(existing) = find_page(&page_map, page_map_len, va) {
                        existing
                    } else if let Some(p) = alloc_pages(1) {
                        let mut flags = MmuFlags::READ.combine(MmuFlags::USER);
                        let x = (sh_flags & 0x4) != 0;
                        let w = (sh_flags & 0x1) != 0;
                        if w {
                            flags = flags.combine(MmuFlags::WRITE);
                        }
                        if x {
                            flags = flags.combine(MmuFlags::EXECUTE);
                        }
                        map_in_pt(pt, va, p, flags);
                        crate::memory::virtual_::vma_add(pt, seg_start, seg_end.saturating_sub(seg_start), flags);
                        if page_map_len < page_map.len() {
                            page_map[page_map_len] = (va, p);
                            page_map_len += 1;
                        }
                        p
                    } else {
                        error!("alloc page failed at va=0x{:016X}", va);
                        return Err(());
                    };
                    unsafe {
                        let kva = crate::memory::virtual_::KERNEL_BASE + pa;
                        crate::memory::virtual_::map(
                            kva,
                            pa,
                            MmuFlags::READ.combine(MmuFlags::WRITE),
                        );
                        let dst = kva as *mut u8;
                        let dst_off = if va == seg_start {
                            sh_addr & 0xFFFusize
                        } else {
                            0
                        };
                        let seg_mem_off = va.saturating_sub(sh_addr);
                        let file_off = sh_offset + seg_mem_off;
                        let to_copy =
                            core::cmp::min(4096 - dst_off, sh_size.saturating_sub(seg_mem_off));
                        if to_copy > 0 {
                            core::ptr::copy_nonoverlapping(
                                binary.as_ptr().add(file_off),
                                dst.add(dst_off),
                                to_copy,
                            );
                            if dst_off + to_copy < 4096 {
                                core::ptr::write_bytes(
                                    dst.add(dst_off + to_copy),
                                    0,
                                    4096 - (dst_off + to_copy),
                                );
                            }
                        } else {
                            core::ptr::write_bytes(dst.add(dst_off), 0, 4096 - dst_off);
                        }
                    }
                    crate::arch::mmu::clean_dcache_range(
                        crate::memory::virtual_::KERNEL_BASE + pa,
                        4096,
                    );
                    va += 4096;
                }
                smapped = true;
            }
            j += 1;
        }
        if !smapped {
            error!("no loadable sections");
            return Err(());
        }
    }

    // Apply RELA relocations (minimal subset) to fix address materialization
    // Helpers to read lists
    fn read_symtab<'a>(bin: &'a [u8], shdr: &Elf64Shdr) -> Option<&'a [Elf64Sym]> {
        if shdr.sh_type != SHT_SYMTAB && shdr.sh_type != SHT_DYNSYM {
            return None;
        }
        let off = shdr.sh_offset as usize;
        let size = shdr.sh_size as usize;
        if off + size > bin.len() {
            return None;
        }
        Some(unsafe {
            core::slice::from_raw_parts(
                bin.as_ptr().add(off) as *const Elf64Sym,
                size / core::mem::size_of::<Elf64Sym>(),
            )
        })
    }
    fn read_rela<'a>(bin: &'a [u8], shdr: &Elf64Shdr) -> Option<&'a [Elf64Rela]> {
        if shdr.sh_type != SHT_RELA {
            return None;
        }
        let off = shdr.sh_offset as usize;
        let size = shdr.sh_size as usize;
        if off + size > bin.len() {
            return None;
        }
        Some(unsafe {
            core::slice::from_raw_parts(
                bin.as_ptr().add(off) as *const Elf64Rela,
                size / core::mem::size_of::<Elf64Rela>(),
            )
        })
    }
    fn page_lookup(page_map: &[(usize, usize); 128], len: usize, va_page: usize) -> Option<usize> {
        let mut i = 0;
        while i < len {
            if page_map[i].0 == va_page {
                return Some(page_map[i].1);
            }
            i += 1;
        }
        None
    }
    let mut applied: usize = 0;
    let mut si = 0;
    while si < e_shnum {
        let shoff = e_shoff + si * e_shentsize;
        if shoff + core::mem::size_of::<Elf64Shdr>() > binary.len() {
            break;
        }
        let shdr = unsafe { core::ptr::read(binary[shoff..].as_ptr() as *const Elf64Shdr) };
        if shdr.sh_type == SHT_RELA {
            // symbol table referenced by sh_link
            let sym_shoff = e_shoff + (shdr.sh_link as usize) * e_shentsize;
            if sym_shoff + core::mem::size_of::<Elf64Shdr>() > binary.len() {
                si += 1;
                continue;
            }
            let sym_shdr =
                unsafe { core::ptr::read(binary[sym_shoff..].as_ptr() as *const Elf64Shdr) };
            let symtab = if let Some(s) = read_symtab(binary, &sym_shdr) {
                s
            } else {
                si += 1;
                continue;
            };
            let relas = if let Some(r) = read_rela(binary, &shdr) {
                r
            } else {
                si += 1;
                continue;
            };
            for r in relas {
                let rtype = (r.r_info & 0xFFFF_FFFF) as u32;
                let sym_idx = (r.r_info >> 32) as usize;
                if sym_idx >= symtab.len() {
                    continue;
                }
                let s_val = symtab[sym_idx].st_value as i64;
                let a = r.r_addend;
                let place_va = r.r_offset as usize;
                let va_page = place_va & !0xFFFusize;
                if let Some(pa_page) = page_lookup(&page_map, page_map_len, va_page) {
                    let off = place_va & 0xFFFusize;
                    unsafe {
                        let iptr = (pa_page + off) as *mut u32;
                        let insn = core::ptr::read_unaligned(iptr);
                        match rtype {
                            R_AARCH64_ADR_PREL_PG_HI21 => {
                                // delta in pages: ((S + A) - P) >> 12
                                let p = ((place_va as i64) & !0xFFFi64);
                                let delta_pg = (((s_val + a) - p) >> 12) as i32; // signed 21-bit
                                let immlo = (delta_pg as u32) & 0x3;
                                let immhi = ((delta_pg as u32) >> 2) & 0x7FFFF; // 19 bits
                                let mut new = insn;
                                // clear fields
                                new &= !(0x3 << 29);
                                new &= !(0x7FFFF << 5);
                                // set
                                new |= immlo << 29;
                                new |= immhi << 5;
                                core::ptr::write_unaligned(iptr, new);
                                applied += 1;
                            }
                            R_AARCH64_ADD_ABS_LO12_NC => {
                                let val = (s_val + a) as u64;
                                let imm12 = (val & 0xFFF) as u32;
                                let mut new = insn;
                                new &= !(0xFFF << 10);
                                new |= imm12 << 10;
                                core::ptr::write_unaligned(iptr, new);
                                applied += 1;
                            }
                            _ => { /* ignore other types for now */ }
                        }
                    }
                }
            }
        }
        si += 1;
    }
    info!("relocations applied: {}", applied);

    // If this is a dynamically linked executable, perform dynamic relocations
    // Note: Dynamic linking functionality has been moved to userspace
    // This is a placeholder for future implementation
    if is_dynamic {
        info!("Dynamic linking functionality has been moved to userspace");
    }

    // Map user stack
    let stack_top = 0x8000_0000usize;
    let stack_pages = 8usize;
    for i in 0..stack_pages {
        let va = stack_top - i * 4096;
        if let Some(p) = alloc_pages(1) {
            unsafe {
                core::ptr::write_bytes(p as *mut u8, 0, 4096);
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

    /// Check if an ELF binary is dynamically linked by looking for SHT_DYNAMIC sections
fn check_dynamic_binary(binary: &[u8], shoff: usize, shentsize: usize, shnum: usize) -> bool {
    for i in 0..shnum {
        let offset = shoff + i * shentsize;
        if offset + core::mem::size_of::<Elf64Shdr>() <= binary.len() {
            let shdr = unsafe { &*(binary[offset..].as_ptr() as *const Elf64Shdr) };
            if shdr.sh_type == SHT_DYNAMIC {
                return true;
            }
        }
    }
    false
}
    Ok(UserProgram {
        entry: e_entry,
        pt_base: pt,
        user_sp: stack_top,
    })
}
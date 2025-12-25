//! Minimal ELF loader - ONLY for bootstrapping init process
//!
//! This is a HARDCODED, MINIMAL ELF loader that ONLY loads the init binary.
//! It does NOT handle:
//! - Dynamic linking
//! - Relocations  
//! - Complex segments
//! - TLS
//! - Multi-threading
//!
//! All real ELF loading should be done by user space Loader Service.

use crate::arch::common::mmu::MmuFlags;
use crate::memory::virtual_::{create_user_l1, map_in_pt};
use crate::{info, error};

const ELF_MAGIC: &[u8] = &[0x7F, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const PT_LOAD: u32 = 1;

/// Minimal ELF loader - only for init process
pub struct BootstrapElfLoader<'a> {
    data: &'a [u8],
}

impl<'a> BootstrapElfLoader<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, &'static str> {
        if data.len() < 64 {
            return Err("ELF file too small");
        }
        
        if &data[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic");
        }
        
        if data[4] != ELF_CLASS_64 {
            return Err("Not 64-bit ELF");
        }
        
        Ok(Self { data })
    }
    
    /// Load init ELF and return (entry, sp, pt_base)
    pub fn load_init(&self) -> Result<(usize, usize, usize), &'static str> {
        let entry = self.read_u64(24) as usize;
        let phoff = self.read_u64(32) as usize;
        let phentsize = self.read_u16(54) as usize;
        let phnum = self.read_u16(56) as usize;
        
        info!("bootstrap: ELF entry=0x{:X}, {} program headers", entry, phnum);
        info!("bootstrap: phoff=0x{:X}, phentsize={}, phnum={}", phoff, phentsize, phnum);
        
        let pt_base = create_user_l1().ok_or("Failed to create page table")?;
        
        let mut max_vaddr = 0usize;
        
        for i in 0..phnum {
            let ph_offset = phoff + i * phentsize;
            let p_type = self.read_u32_at(ph_offset);
            
            if p_type != PT_LOAD {
                continue;
            }
            
            let p_flags = self.read_u32_at(ph_offset + 4);
            let p_offset = self.read_u64_at(ph_offset + 8) as usize;
            let p_vaddr = self.read_u64_at(ph_offset + 16) as usize;
            let p_filesz = self.read_u64_at(ph_offset + 32) as usize;
            let p_memsz = self.read_u64_at(ph_offset + 40) as usize;
            
            let r = if p_flags & 0x4 != 0 { 'R' } else { '-' };
            let w = if p_flags & 0x2 != 0 { 'W' } else { '-' };
            let x = if p_flags & 0x1 != 0 { 'X' } else { '-' };
            
            info!("bootstrap: LOAD segment[{}] vaddr=0x{:X}, offset=0x{:X}, filesz=0x{:X}, memsz=0x{:X}, flags={}{}{}", 
                  i, p_vaddr, p_offset, p_filesz, p_memsz, r, w, x);
            
            self.load_segment(pt_base, p_vaddr, p_offset, p_filesz, p_memsz, p_flags)?;
            
            let seg_end = p_vaddr + p_memsz;
            if seg_end > max_vaddr {
                max_vaddr = seg_end;
            }
        }
        
        let user_sp = (max_vaddr + 0x10000 + 0xFFF) & !0xFFF;
        
        for i in 0..16 {
            let stack_va = user_sp - ((i + 1) * 0x1000);
            if let Some(stack_pa) = crate::memory::physical::alloc_pages(1) {
                let stack_flags = MmuFlags::READ
                    .combine(MmuFlags::WRITE)
                    .combine(MmuFlags::USER);
                map_in_pt(pt_base, stack_va, stack_pa, stack_flags);
            } else {
                return Err("Failed to allocate stack");
            }
        }
        
        info!("bootstrap: init loaded - entry=0x{:X}, sp=0x{:X}, pt=0x{:X}", 
              entry, user_sp, pt_base);
        
        Ok((entry, user_sp, pt_base))
    }
    
    fn load_segment(
        &self,
        pt_base: usize,
        vaddr: usize,
        offset: usize,
        filesz: usize,
        memsz: usize,
        flags: u32,
    ) -> Result<(), &'static str> {
        let page_start = vaddr & !0xFFF;
        let page_end = (vaddr + memsz + 0xFFF) & !0xFFF;
        let num_pages = (page_end - page_start) / 0x1000;
        
        let mut mmu_flags = MmuFlags::USER;
        if flags & 0x4 != 0 { mmu_flags = mmu_flags.combine(MmuFlags::READ); }
        if flags & 0x2 != 0 { mmu_flags = mmu_flags.combine(MmuFlags::WRITE); }
        if flags & 0x1 != 0 { mmu_flags = mmu_flags.combine(MmuFlags::EXECUTE); }
        
        info!("bootstrap: loading segment: page_start=0x{:X}, page_end=0x{:X}, num_pages={}", 
              page_start, page_end, num_pages);
        
        for i in 0..num_pages {
            let page_va = page_start + i * 0x1000;
            let page_pa = crate::memory::physical::alloc_pages(1)
                .ok_or("Failed to allocate page")?;
            
            map_in_pt(pt_base, page_va, page_pa, mmu_flags);
            
            unsafe {
                core::ptr::write_bytes(page_pa as *mut u8, 0, 0x1000);
            }
            
            let page_offset_in_seg = if page_va >= vaddr { 
                page_va - vaddr 
            } else { 
                0 
            };
            let bytes_to_copy_in_page = if page_offset_in_seg < filesz {
                core::cmp::min(filesz - page_offset_in_seg, 0x1000)
            } else {
                0
            };
            
            if bytes_to_copy_in_page > 0 {
                let src_offset = offset + page_offset_in_seg;
                let dst_offset_in_page = if page_va < vaddr {
                    vaddr - page_va
                } else {
                    0
                };
                
                if src_offset + bytes_to_copy_in_page <= self.data.len() {
                    unsafe {
                        let dst = (page_pa + dst_offset_in_page) as *mut u8;
                        let src = self.data.as_ptr().add(src_offset);
                        core::ptr::copy_nonoverlapping(src, dst, bytes_to_copy_in_page);
                    }
                    
                    info!("bootstrap:   page[{}] va=0x{:X} pa=0x{:X} copied {} bytes from offset 0x{:X}", 
                          i, page_va, page_pa, bytes_to_copy_in_page, src_offset);
                }
            } else {
                info!("bootstrap:   page[{}] va=0x{:X} pa=0x{:X} (zero-filled)", 
                      i, page_va, page_pa);
            }
        }
        
        Ok(())
    }
    
    fn read_u16(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.data[offset], self.data[offset + 1]])
    }
    
    fn read_u32_at(&self, offset: usize) -> u32 {
        u32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ])
    }
    
    fn read_u64(&self, offset: usize) -> u64 {
        self.read_u64_at(offset)
    }
    
    fn read_u64_at(&self, offset: usize) -> u64 {
        u64::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
            self.data[offset + 4],
            self.data[offset + 5],
            self.data[offset + 6],
            self.data[offset + 7],
        ])
    }
}
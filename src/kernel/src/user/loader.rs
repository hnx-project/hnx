use crate::arch::common::mmu::MmuFlags;
use crate::memory::virtual_::{create_user_l1, map_in_pt};
use crate::{error, info};

pub struct UserProgram {
    pub entry: usize,
    pub user_sp: usize,
    pub pt_base: usize,
}

pub fn create_simple_user_program(code: &[u8]) -> Result<UserProgram, ()> {
    let pt_base = create_user_l1().ok_or(())?;
    let user_sp = 0x0010_0000usize;
    
    let code_pa = crate::memory::physical::alloc_pages(1).ok_or(())?;
    let data_pa = crate::memory::physical::alloc_pages(1).ok_or(())?;
    
    let code_va = 0x0000_1000usize;
    let data_va = 0x0000_2000usize;
    
    let code_flags = MmuFlags::READ
        .combine(MmuFlags::EXECUTE)
        .combine(MmuFlags::USER);
    let data_flags = MmuFlags::READ
        .combine(MmuFlags::WRITE)
        .combine(MmuFlags::USER);
    
    map_in_pt(pt_base, code_va, code_pa, code_flags);
    map_in_pt(pt_base, data_va, data_pa, data_flags);
    map_in_pt(pt_base, user_sp - 0x1000, data_pa, data_flags);
    
    unsafe {
        for (i, &b) in code.iter().enumerate().take(4096) {
            core::ptr::write((code_pa as *mut u8).add(i), b);
        }
        crate::arch::mmu::clean_dcache_range(code_pa, 4096);
    }
    crate::arch::mmu::flush_icache_all();
    
    Ok(UserProgram {
        entry: code_va,
        user_sp,
        pt_base,
    })
}
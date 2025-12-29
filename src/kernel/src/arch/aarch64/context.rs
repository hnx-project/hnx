//! AArch64 上下文切换实现

use crate::arch::common::traits::Context;

/// AArch64 上下文实现
pub struct AArch64Context;

impl Context for AArch64Context {
    fn exec_user(
        entry_point: usize,
        stack_pointer: usize,
        page_table_base: usize,
        asid: u16,
        args: (usize, usize, usize, usize),
    ) -> ! {
        // 编码 ASID 到 TTBR0_EL1[63:48]
        let ttbr0_with_asid = (page_table_base & 0x0000_FFFF_FFFF_FFFF) | ((asid as usize) << 48);
        
        crate::info!(
            "AArch64Context::exec_user: entry=0x{:016X} sp=0x{:016X} ttbr0=0x{:016X} asid={} args={:?}",
            entry_point, stack_pointer, ttbr0_with_asid, asid, args
        );
        
        // 调用汇编实现的 arch_do_exec
        unsafe {
            extern "C" {
                fn arch_do_exec(
                    elr: usize,
                    sp0: usize,
                    ttbr0: usize,
                    a0: usize,
                    a1: usize,
                    a2: usize,
                    a8: usize,
                ) -> !;
            }
            
            arch_do_exec(
                entry_point,
                stack_pointer,
                ttbr0_with_asid,
                args.0,
                args.1,
                args.2,
                args.3,
            );
        }
    }
    
    fn get_elr() -> usize {
        unsafe {
            let mut elr: u64;
            core::arch::asm!("mrs {}, elr_el1", out(reg) elr);
            elr as usize
        }
    }
    
    fn get_sp() -> usize {
        unsafe {
            let mut sp: u64;
            core::arch::asm!("mrs {}, sp_el0", out(reg) sp);
            sp as usize
        }
    }
    
    fn get_vbar() -> usize {
        unsafe {
            let mut vbar: u64;
            core::arch::asm!("mrs {}, vbar_el1", out(reg) vbar);
            vbar as usize
        }
    }
    
    fn get_current_el() -> u32 {
        unsafe {
            let mut cur_el: u64;
            core::arch::asm!("mrs {}, CurrentEL", out(reg) cur_el);
            ((cur_el >> 2) & 0x3) as u32  // CurrentEL[3:2] 包含异常级别
        }
    }

    fn get_spsr() -> usize {
        unsafe {
            let mut spsr: u64;
            core::arch::asm!("mrs {}, spsr_el1", out(reg) spsr);
            spsr as usize
        }
    }

    fn get_ttbr0() -> usize {
        unsafe {
            let mut ttbr0: u64;
            core::arch::asm!("mrs {}, ttbr0_el1", out(reg) ttbr0);
            ttbr0 as usize
        }
    }

    fn get_ttbr1() -> usize {
        unsafe {
            let mut ttbr1: u64;
            core::arch::asm!("mrs {}, ttbr1_el1", out(reg) ttbr1);
            ttbr1 as usize
        }
    }

    fn get_spsel() -> u32 {
        unsafe {
            let mut spsel: u64;
            core::arch::asm!("mrs {}, SPSel", out(reg) spsel);
            (spsel & 1) as u32  // SPSel[0] 表示当前使用的栈指针
        }
    }
}

// 模块级函数包装
pub fn exec_user(
    entry_point: usize,
    stack_pointer: usize,
    page_table_base: usize,
    asid: u16,
    args: (usize, usize, usize, usize),
) -> ! {
    AArch64Context::exec_user(entry_point, stack_pointer, page_table_base, asid, args)
}

pub fn get_elr() -> usize {
    AArch64Context::get_elr()
}

pub fn get_sp() -> usize {
    AArch64Context::get_sp()
}

pub fn get_vbar() -> usize {
    AArch64Context::get_vbar()
}

pub fn get_current_el() -> u32 {
    AArch64Context::get_current_el()
}

pub fn get_spsr() -> usize {
    AArch64Context::get_spsr()
}

pub fn get_ttbr0() -> usize {
    AArch64Context::get_ttbr0()
}

pub fn get_ttbr1() -> usize {
    AArch64Context::get_ttbr1()
}

pub fn get_spsel() -> u32 {
    AArch64Context::get_spsel()
}
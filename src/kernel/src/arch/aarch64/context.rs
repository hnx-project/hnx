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

    fn get_saved_gpr(saved_sp: usize, reg: usize) -> usize {
        // AArch64 异常栈布局 (从低地址向高地址增长):
        // sp+0:   x18, xzr
        // sp+16:  x16, x17
        // sp+32:  x14, x15
        // sp+48:  x12, x13
        // sp+64:  x10, x11
        // sp+80:  x8, x9
        // sp+96:  x6, x7
        // sp+112: x4, x5
        // sp+128: x2, x3
        // sp+144: x0, x1
        // sp+160: x25, xzr
        // sp+176: x23, x24
        // sp+192: x21, x22
        // sp+208: x19, x20
        //
        // 注意：栈从高地址向低地址增长，但保存的寄存器区域是连续的

        if reg > 30 {
            return 0; // 无效寄存器索引
        }

        unsafe {
            let base = saved_sp as *const usize;

            match reg {
                0 => *base.add(144 / 8),  // x0 at sp+144
                1 => *base.add(152 / 8),  // x1 at sp+152
                2 => *base.add(128 / 8),  // x2 at sp+128
                3 => *base.add(136 / 8),  // x3 at sp+136
                4 => *base.add(112 / 8),  // x4 at sp+112
                5 => *base.add(120 / 8),  // x5 at sp+120
                6 => *base.add(96 / 8),   // x6 at sp+96
                7 => *base.add(104 / 8),  // x7 at sp+104
                8 => *base.add(80 / 8),   // x8 at sp+80
                9 => *base.add(88 / 8),   // x9 at sp+88
                10 => *base.add(64 / 8),  // x10 at sp+64
                11 => *base.add(72 / 8),  // x11 at sp+72
                12 => *base.add(48 / 8),  // x12 at sp+48
                13 => *base.add(56 / 8),  // x13 at sp+56
                14 => *base.add(32 / 8),  // x14 at sp+32
                15 => *base.add(40 / 8),  // x15 at sp+40
                16 => *base.add(16 / 8),  // x16 at sp+16
                17 => *base.add(24 / 8),  // x17 at sp+24
                18 => *base.add(0 / 8),   // x18 at sp+0
                19 => *base.add(208 / 8), // x19 at sp+208
                20 => *base.add(216 / 8), // x20 at sp+216
                21 => *base.add(192 / 8), // x21 at sp+192
                22 => *base.add(200 / 8), // x22 at sp+200
                23 => *base.add(176 / 8), // x23 at sp+176
                24 => *base.add(184 / 8), // x24 at sp+184
                25 => *base.add(160 / 8), // x25 at sp+160
                // 26-30: 这些寄存器在异常处理中没有保存，返回0
                _ => 0,
            }
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

pub fn get_saved_gpr(saved_sp: usize, reg: usize) -> usize {
    AArch64Context::get_saved_gpr(saved_sp, reg)
}
// arch/common/mmu.rs
#[derive(Clone, Copy)]
pub struct MmuFlags(u64);

impl core::fmt::Debug for MmuFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}{}{}{}{}", 
            if self.contains(Self::READ) { "R" } else { "" },
            if self.contains(Self::WRITE) { "W" } else { "" },
            if self.contains(Self::EXECUTE) { "X" } else { "" },
            if self.contains(Self::USER) { "U" } else { "" },
            if self.contains(Self::DEVICE) { "D" } else { "" }
        )
    }
}

impl MmuFlags {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);
    pub const DEVICE: Self = Self(1 << 4);

    pub fn combine(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    // 转换为架构特定的标志位
    pub fn to_arch(&self, arch: ArchType) -> u64 {
        match arch {
            ArchType::AArch64 => {
                let mut bits = 0u64;
                // AttrIndx: 0 = Normal (MAIR idx0 = 0xFF), 1 = Device nGnRnE (MAIR idx1 = 0x00)
                if self.0 & Self::DEVICE.0 != 0 {
                    bits |= 1 << 2;
                }
                // AP[7:6] permissions
                // 00: EL1 RW, EL0 NA
                // 01: EL1 RO, EL0 NA
                // 10: EL1 RW, EL0 RW
                // 11: EL1 RO, EL0 RO
                let user = (self.0 & Self::USER.0) != 0;
                let write = (self.0 & Self::WRITE.0) != 0;
                let execute = (self.0 & Self::EXECUTE.0) != 0;
                
                let ap = if user {
                    if write {
                        0b10
                    } else {
                        0b11
                    }
                } else if write {
                    0b00
                } else {
                    0b01
                };
                // AP[2] at bit 6, AP[1] at bit 7
                // ap[0] is LSB of ap value, ap[1] is MSB of ap value
                // ap=0b10 means AP[2]=1 (user access), AP[1]=0 (read-write)
                // So: AP[2] (bit 6) = (ap >> 1) & 1, AP[1] (bit 7) = ap & 1
                bits |= (((ap >> 1) & 1) << 6) | ((ap & 1) << 7);
                
                // CRITICAL SECURITY: Set PXN/UXN bits for execute protection
                // PXN (Privileged eXecute Never) - bit 53: prevents EL1 from executing this page
                // UXN (User eXecute Never) - bit 54: prevents EL0 from executing this page
                
                if user {
                    // User page:
                    // - Always set PXN (kernel should NEVER execute user code)
                    bits |= 1u64 << 53; // PXN = 1 (prevent kernel execution)
                    
                    // - Set UXN if page is not executable for user
                    if !execute {
                        bits |= 1u64 << 54; // UXN = 1 (prevent user execution)
                    }
                } else {
                    // Kernel page:
                    // - Always set UXN (user should NEVER execute kernel code)
                    bits |= 1u64 << 54; // UXN = 1 (prevent user execution)
                    
                    // - Set PXN if page is not executable for kernel
                    if !execute {
                        bits |= 1u64 << 53; // PXN = 1 (prevent kernel execution)
                    }
                }
                
                bits
            }
            ArchType::Riscv64 => 0,
        }
    }

    // 从架构特定的标志位转换
    pub fn from_arch(bits: u64, arch: ArchType) -> Self {
        match arch {
            ArchType::AArch64 => {
                let mut flags = Self(0);
                // Check AttrIndx for device memory
                if (bits >> 2) & 0x7 == 1 {
                    flags = flags.combine(Self::DEVICE);
                }
                // Extract AP[7:6] permissions
                let ap = (bits >> 6) & 0x3;
                match ap {
                    0b00 | 0b10 => {
                        // Read-write permissions
                        flags = flags.combine(Self::READ).combine(Self::WRITE);
                        if ap == 0b10 {
                            flags = flags.combine(Self::USER);
                        }
                    }
                    0b01 | 0b11 => {
                        // Read-only permissions
                        flags = flags.combine(Self::READ);
                        if ap == 0b11 {
                            flags = flags.combine(Self::USER);
                        }
                    }
                    _ => {}
                }
                // Check UXN/PXN for execute permissions
                if (bits & (1u64 << 54)) == 0 && (bits & (1u64 << 53)) == 0 {
                    flags = flags.combine(Self::EXECUTE);
                }
                flags
            }
            ArchType::Riscv64 => Self(0),
        }
    }

    // 公共方法来检查标志位
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    // 获取原始值
    pub fn bits(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub enum ArchType {
    AArch64,
    Riscv64,
}

// 页表项 trait
pub trait PageTableEntry {
    fn new(paddr: usize, flags: MmuFlags) -> Self;
    fn is_valid(&self) -> bool;
    fn paddr(&self) -> usize;
    fn flags(&self) -> MmuFlags;
}
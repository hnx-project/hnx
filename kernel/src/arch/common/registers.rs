// 寄存器访问通用工具
//
// 提供安全的寄存器访问抽象。

use core::ptr::{read_volatile, write_volatile};

/// 内存映射IO地址
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct MmioAddress(usize);

impl MmioAddress {
    /// 创建新的MMIO地址
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }
    
    /// 读取8位值
    pub unsafe fn read_u8(&self) -> u8 {
        read_volatile(self.0 as *const u8)
    }
    
    /// 写入8位值
    pub unsafe fn write_u8(&self, value: u8) {
        write_volatile(self.0 as *mut u8, value);
    }
    
    /// 读取16位值
    pub unsafe fn read_u16(&self) -> u16 {
        read_volatile(self.0 as *const u16)
    }
    
    /// 写入16位值
    pub unsafe fn write_u16(&self, value: u16) {
        write_volatile(self.0 as *mut u16, value);
    }
    
    /// 读取32位值
    pub unsafe fn read_u32(&self) -> u32 {
        read_volatile(self.0 as *const u32)
    }
    
    /// 写入32位值
    pub unsafe fn write_u32(&self, value: u32) {
        write_volatile(self.0 as *mut u32, value);
    }
    
    /// 读取64位值
    pub unsafe fn read_u64(&self) -> u64 {
        read_volatile(self.0 as *const u64)
    }
    
    /// 写入64位值
    pub unsafe fn write_u64(&self, value: u64) {
        write_volatile(self.0 as *mut u64, value);
    }
    
    /// 读取结构体
    pub unsafe fn read<T: Copy>(&self) -> T {
        read_volatile(self.0 as *const T)
    }
    
    /// 写入结构体
    pub unsafe fn write<T>(&self, value: T) {
        write_volatile(self.0 as *mut T, value);
    }
    
    /// 偏移地址
    pub fn offset(&self, offset: usize) -> Self {
        Self(self.0 + offset)
    }

    pub const fn address(&self) -> usize {
        self.0
    }
}

/// 寄存器访问器
pub trait RegisterAccess {
    type Value;
    
    /// 读取寄存器值
    fn read(&self) -> Self::Value;
    
    /// 写入寄存器值
    fn write(&mut self, value: Self::Value);
    
    /// 修改寄存器值
    fn modify<F>(&mut self, f: F)
    where
        F: FnOnce(Self::Value) -> Self::Value;
}

/// 位字段操作
pub mod bitfield {
    /// 提取位字段
    pub const fn extract(value: u64, offset: u32, width: u32) -> u64 {
        (value >> offset) & ((1 << width) - 1)
    }
    
    /// 插入位字段
    pub const fn insert(value: u64, field: u64, offset: u32, width: u32) -> u64 {
        let mask = ((1 << width) - 1) << offset;
        (value & !mask) | ((field << offset) & mask)
    }
    
    /// 测试位
    pub const fn test(value: u64, bit: u32) -> bool {
        (value >> bit) & 1 == 1
    }
    
    /// 设置位
    pub const fn set(value: u64, bit: u32) -> u64 {
        value | (1 << bit)
    }
    
    /// 清除位
    pub const fn clear(value: u64, bit: u32) -> u64 {
        value & !(1 << bit)
    }
}

// ARMv8-A boot模块实现
//
// 要求实现：
// - 定义具体类型：
//   Aarch64boot 等
// - 实现架构抽象特征对应的具体方法
// - 使用tock-registers定义寄存器
// - 使用aarch64-cpu访问CPU功能
// - 遵循"一切皆对象"原则
//
// 主要API：
// - init(): 初始化本模块
// - 实现对应trait的所有方法
// - 提供对象构造和销毁接口
//
// 对象类型：
// - Aarch64boot: 主对象类型
// - Aarch64bootInfo: 信息对象
// - Aarch64bootConfig: 配置对象
// - Aarch64bootState: 状态对象
//
// 关键功能：
// - 异常向量表设置
// - 栈指针初始化
// - MMU早期设置
// - 设备树解析
// - 多核启动协调
// - 启动参数传递

// 需要实现的具体类型（示例）：
// pub struct Aarch64Module;
// pub struct Aarch64ModuleInfo;
// pub struct Aarch64ModuleConfig;
//
// 需要实现的方法（示例）：
// pub fn init() -> ArchResult<()>;
// pub fn create(config: &Aarch64ModuleConfig) -> ArchResult<Aarch64Module>;
// pub fn info(&self) -> Aarch64ModuleInfo;
//
// 寄存器定义（示例）：
// use tock_registers::{register_bitfields, register_structs, registers::*};
//
// 对象管理（示例）：
// use crate::object::{KernelObject, Handle, ObjectRights};
//
// 遵循"一切皆对象"原则，所有资源都封装为对象。

core::arch::global_asm!(include_str!("boot.S"));

use crate::arch::traits::boot::{BootCpuInfo, BootInfo, DeviceTree, DeviceTreeNode, MemoryRegion};
use crate::arch::traits::mmu::MemoryType;
use crate::arch::ArchResult;

const PHYS_BASE: usize = 0x4000_0000;
const DEFAULT_PHYS_MEM_SIZE: usize = 512 * 1024 * 1024;

const UART_BASE: usize = 0x0900_0000;
const UART_SIZE: usize = 0x1000;

const GIC_DIST_BASE: usize = 0x0800_0000;
const GIC_CPU_BASE: usize = 0x0801_0000;
const GIC_SIZE: usize = 0x20000;

static DEVICE_MEMORY_REGIONS: [MemoryRegion; 3] = [
    MemoryRegion::new(UART_BASE, UART_SIZE, MemoryType::Device),
    MemoryRegion::new(GIC_DIST_BASE, GIC_SIZE, MemoryType::Device),
    MemoryRegion::new(GIC_CPU_BASE, GIC_SIZE, MemoryType::Device),
];

static MEMORY_MAP: [MemoryRegion; 4] = [
    MemoryRegion::new(PHYS_BASE, DEFAULT_PHYS_MEM_SIZE, MemoryType::Normal),
    MemoryRegion::new(UART_BASE, UART_SIZE, MemoryType::Device),
    MemoryRegion::new(GIC_DIST_BASE, GIC_SIZE, MemoryType::Device),
    MemoryRegion::new(GIC_CPU_BASE, GIC_SIZE, MemoryType::Device),
];

#[derive(Debug)]
pub struct Aarch64Boot;

#[derive(Debug)]
pub struct Aarch64BootInfo;

impl BootInfo for Aarch64BootInfo {
    fn total_physical_memory(&self) -> u64 {
        DEFAULT_PHYS_MEM_SIZE as u64
    }

    fn available_physical_memory(&self) -> u64 {
        DEFAULT_PHYS_MEM_SIZE as u64
    }

    fn reserved_memory_regions(&self) -> &[MemoryRegion] {
        &[]
    }

    fn device_memory_regions(&self) -> &[MemoryRegion] {
        &DEVICE_MEMORY_REGIONS
    }

    fn kernel_image(&self) -> crate::arch::traits::boot::KernelImageInfo {
        extern "C" {
            fn _start();
            static __kernel_end: u8;
            static __text_vma_start: u8;
            static __text_vma_end: u8;
            static __rodata_vma_start: u8;
            static __rodata_vma_end: u8;
            static __data_vma_start: u8;
            static __data_vma_end: u8;
            static __bss_vma_start: u8;
            static __bss_vma_end: u8;
        }

        let base = _start as *const () as usize;
        let end = unsafe { &__kernel_end as *const u8 as usize };

        let sections = [
            (
                ".text",
                unsafe { &__text_vma_start as *const u8 as usize },
                unsafe { &__text_vma_end as *const u8 as usize },
                crate::arch::traits::boot::SectionFlags {
                    executable: true,
                    writable: false,
                    readable: true,
                },
            ),
            (
                ".rodata",
                unsafe { &__rodata_vma_start as *const u8 as usize },
                unsafe { &__rodata_vma_end as *const u8 as usize },
                crate::arch::traits::boot::SectionFlags {
                    executable: false,
                    writable: false,
                    readable: true,
                },
            ),
            (
                ".data",
                unsafe { &__data_vma_start as *const u8 as usize },
                unsafe { &__data_vma_end as *const u8 as usize },
                crate::arch::traits::boot::SectionFlags {
                    executable: false,
                    writable: true,
                    readable: true,
                },
            ),
            (
                ".bss",
                unsafe { &__bss_vma_start as *const u8 as usize },
                unsafe { &__bss_vma_end as *const u8 as usize },
                crate::arch::traits::boot::SectionFlags {
                    executable: false,
                    writable: true,
                    readable: true,
                },
            ),
        ];

        crate::arch::traits::boot::KernelImageInfo {
            base,
            size: end.saturating_sub(base),
            entry_point: base,
            sections: sections
                .into_iter()
                .map(
                    |(name, start, end, flags)| crate::arch::traits::boot::SectionInfo {
                        name: alloc::string::String::from(name),
                        address: start,
                        size: end.saturating_sub(start),
                        flags,
                    },
                )
                .collect(),
        }
    }

    fn bootloader(&self) -> crate::arch::traits::boot::BootloaderInfo {
        crate::arch::traits::boot::BootloaderInfo {
            name: alloc::string::String::new(),
            version: alloc::string::String::new(),
            boot_method: crate::arch::traits::boot::BootMethod::DirectKernelBoot,
        }
    }

    fn platform(&self) -> crate::arch::traits::boot::PlatformInfo {
        crate::arch::traits::boot::PlatformInfo {
            manufacturer: alloc::string::String::new(),
            product: alloc::string::String::new(),
            version: alloc::string::String::new(),
            serial: None,
            uuid: None,
        }
    }

    fn has_acpi(&self) -> bool {
        false
    }

    fn has_device_tree(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct Aarch64DeviceTree;

impl DeviceTree for Aarch64DeviceTree {
    fn root(&self) -> DeviceTreeNode {
        DeviceTreeNode::new(0)
    }

    fn find_node(&self, _path: &str) -> Option<DeviceTreeNode> {
        None
    }

    fn find_compatible(&self, _compatible: &str) -> alloc::vec::Vec<DeviceTreeNode> {
        alloc::vec::Vec::new()
    }

    fn property(
        &self,
        _node: DeviceTreeNode,
        _name: &str,
    ) -> Option<crate::arch::traits::boot::DeviceTreeProperty> {
        None
    }

    fn memory_regions(&self) -> alloc::vec::Vec<MemoryRegion> {
        alloc::vec::Vec::new()
    }

    fn interrupt_controllers(
        &self,
    ) -> alloc::vec::Vec<crate::arch::traits::boot::InterruptControllerInfo> {
        alloc::vec::Vec::new()
    }

    fn cpus(&self) -> alloc::vec::Vec<BootCpuInfo> {
        alloc::vec::Vec::new()
    }
}

static mut BOOT_INFO: Aarch64BootInfo = Aarch64BootInfo;

pub unsafe fn early_init() {
    crate::debug::init();
}

pub fn init() -> ArchResult<Aarch64BootInfo> {
    unsafe {
        BOOT_INFO = Aarch64BootInfo;
    }
    Ok(Aarch64BootInfo)
}

pub fn boot_info() -> &'static Aarch64BootInfo {
    unsafe { &*(&raw const BOOT_INFO) }
}

pub fn init_memory(_info: &Aarch64BootInfo) -> ArchResult<()> {
    Ok(())
}

pub fn init_console() -> ArchResult<()> {
    Ok(())
}

pub fn init_smp() -> ArchResult<()> {
    Ok(())
}

pub fn boot_secondary_cpus() -> ArchResult<()> {
    Ok(())
}

pub fn platform_init() -> ArchResult<()> {
    Ok(())
}

pub fn device_tree() -> Option<&'static Aarch64DeviceTree> {
    None
}

pub fn command_line() -> &'static str {
    ""
}

pub fn memory_map() -> &'static [MemoryRegion] {
    &MEMORY_MAP
}

pub fn boot_timestamp() -> u64 {
    use aarch64_cpu::registers::{Readable, CNTVCT_EL0};
    CNTVCT_EL0.get()
}

pub fn cleanup_boot_resources() -> ArchResult<()> {
    Ok(())
}

#[no_mangle]
pub extern "C" fn rust_exc_mark(_ec: u64, _esr: u64, _elr: u64, _far: u64) {}

#[no_mangle]
pub extern "C" fn rust_svc_handler(
    _esr: u64,
    _elr: u64,
    _far: u64,
    _saved_x8: u64,
    saved_sp: usize,
) {
    unsafe {
        let x0_ptr = (saved_sp as *mut i64).add(18);
        *x0_ptr = shared::abi::errors::ZX_ERR_NOT_SUPPORTED as i64;
    }
}

#[no_mangle]
pub extern "C" fn rust_irq_handler() {}

#[no_mangle]
pub extern "C" fn rust_sync_try_handle(
    _esr: u64,
    _elr: u64,
    _far: u64,
    _tcr: u64,
    _sctlr: u64,
    _spsr: u64,
) -> u64 {
    1
}

#[no_mangle]
pub extern "C" fn rust_sync_panic() {
    loop {
        core::hint::spin_loop();
    }
}

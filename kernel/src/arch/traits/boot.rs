// 启动引导抽象特征
//
// 要求实现：
// - 早期启动代码
// - CPU初始化
// - 内存探测
// - 设备树/ACPI解析
// - 多核启动
// - 栈设置
// - 内核重定位

use super::super::ArchResult;
use alloc::{string::String, vec::Vec};

use super::cpu::CacheInfo;
use super::cpu::CpuInfo as CpuInfoTrait;
use super::mmu::{MemoryRegion as MmuMemoryRegion, MemoryType};

/// 启动架构抽象特征
pub trait BootArch: Send + Sync {
    /// 引导信息对象类型
    type BootInfo: BootInfo;

    /// 设备树对象类型
    type DeviceTree: DeviceTree;

    /// 早期启动初始化
    unsafe fn early_init();

    /// 主启动初始化
    fn init() -> ArchResult<Self::BootInfo>;

    /// 获取引导信息
    fn boot_info() -> &'static Self::BootInfo;

    /// 初始化内存管理
    fn init_memory(info: &Self::BootInfo) -> ArchResult<()>;

    /// 初始化控制台
    fn init_console() -> ArchResult<()>;

    /// 初始化多核
    fn init_smp() -> ArchResult<()>;

    /// 启动从核
    fn boot_secondary_cpus() -> ArchResult<()>;

    /// 平台特定初始化
    fn platform_init() -> ArchResult<()>;

    /// 获取设备树
    fn device_tree() -> Option<&'static Self::DeviceTree>;

    /// 获取命令行参数
    fn command_line() -> &'static str;

    /// 获取内存映射
    fn memory_map() -> &'static [MemoryRegion];

    /// 获取启动时间戳
    fn boot_timestamp() -> u64;

    /// 清理引导资源
    fn cleanup_boot_resources() -> ArchResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub base: usize,
    pub size: usize,
    pub memory_type: MemoryType,
    pub cacheable: bool,
    pub executable: bool,
    pub writable: bool,
    pub device: bool,
}

impl MemoryRegion {
    pub const fn new(base: usize, size: usize, memory_type: MemoryType) -> Self {
        let device = matches!(memory_type, MemoryType::Device);
        Self {
            base,
            size,
            memory_type,
            cacheable: !device,
            executable: !device,
            writable: true,
            device,
        }
    }
}

impl MmuMemoryRegion for MemoryRegion {
    fn base(&self) -> usize {
        self.base
    }

    fn size(&self) -> usize {
        self.size
    }

    fn memory_type(&self) -> MemoryType {
        self.memory_type
    }

    fn is_cacheable(&self) -> bool {
        self.cacheable
    }

    fn is_executable(&self) -> bool {
        self.executable
    }

    fn is_writable(&self) -> bool {
        self.writable
    }

    fn is_device(&self) -> bool {
        self.device
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BootCpuInfo {
    pub id: usize,
    pub arch_version: u32,
    pub has_virtualization: bool,
    pub has_simd: bool,
    pub vendor: &'static str,
}

impl BootCpuInfo {
    pub const fn new(id: usize) -> Self {
        Self {
            id,
            arch_version: 8,
            has_virtualization: false,
            has_simd: false,
            vendor: "unknown",
        }
    }

    pub const fn cache_info(&self) -> CacheInfo {
        CacheInfo {
            l1_data: super::cpu::CacheLevel {
                size: 0,
                line_size: 0,
                associativity: 0,
            },
            l1_instruction: super::cpu::CacheLevel {
                size: 0,
                line_size: 0,
                associativity: 0,
            },
            l2: None,
            l3: None,
        }
    }
}

impl CpuInfoTrait for BootCpuInfo {
    fn id(&self) -> usize {
        self.id
    }

    fn vendor(&self) -> &str {
        self.vendor
    }

    fn arch_version(&self) -> u32 {
        self.arch_version
    }

    fn has_virtualization(&self) -> bool {
        self.has_virtualization
    }

    fn has_simd(&self) -> bool {
        self.has_simd
    }

    fn cache_info(&self) -> CacheInfo {
        self.cache_info()
    }
}

/// 引导信息特征
pub trait BootInfo: Send + Sync {
    /// 物理内存总量
    fn total_physical_memory(&self) -> u64;

    /// 可用物理内存
    fn available_physical_memory(&self) -> u64;

    /// 保留内存区域
    fn reserved_memory_regions(&self) -> &[MemoryRegion];

    /// 设备内存区域
    fn device_memory_regions(&self) -> &[MemoryRegion];

    /// 内核映像信息
    fn kernel_image(&self) -> KernelImageInfo;

    /// 引导加载器信息
    fn bootloader(&self) -> BootloaderInfo;

    /// 平台信息
    fn platform(&self) -> PlatformInfo;

    /// 是否支持ACPI
    fn has_acpi(&self) -> bool;

    /// 是否支持设备树
    fn has_device_tree(&self) -> bool;
}

/// 设备树特征
pub trait DeviceTree: Send + Sync {
    /// 根节点
    fn root(&self) -> DeviceTreeNode;

    /// 根据路径查找节点
    fn find_node(&self, path: &str) -> Option<DeviceTreeNode>;

    /// 根据兼容性查找节点
    fn find_compatible(&self, compatible: &str) -> Vec<DeviceTreeNode>;

    /// 获取属性值
    fn property(&self, node: DeviceTreeNode, name: &str) -> Option<DeviceTreeProperty>;

    /// 获取内存映射
    fn memory_regions(&self) -> Vec<MemoryRegion>;

    /// 获取中断控制器信息
    fn interrupt_controllers(&self) -> Vec<InterruptControllerInfo>;

    /// 获取CPU信息
    fn cpus(&self) -> Vec<BootCpuInfo>;
}

/// 设备树节点句柄
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceTreeNode(usize);

impl DeviceTreeNode {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn value(&self) -> usize {
        self.0
    }
}

/// 设备树属性
#[derive(Debug, Clone)]
pub struct DeviceTreeProperty {
    pub name: String,
    pub value: Vec<u8>,
}

/// 内核映像信息
#[derive(Debug, Clone)]
pub struct KernelImageInfo {
    pub base: usize,
    pub size: usize,
    pub entry_point: usize,
    pub sections: Vec<SectionInfo>,
}

/// 节信息
#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub name: String,
    pub address: usize,
    pub size: usize,
    pub flags: SectionFlags,
}

/// 节标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionFlags {
    pub executable: bool,
    pub writable: bool,
    pub readable: bool,
}

/// 引导加载器信息
#[derive(Debug, Clone)]
pub struct BootloaderInfo {
    pub name: String,
    pub version: String,
    pub boot_method: BootMethod,
}

/// 引导方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMethod {
    Uefi,
    LegacyBios,
    Coreboot,
    LinuxBoot,
    DirectKernelBoot,
}

/// 平台信息
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub manufacturer: String,
    pub product: String,
    pub version: String,
    pub serial: Option<String>,
    pub uuid: Option<[u8; 16]>,
}

/// 中断控制器信息
#[derive(Debug, Clone)]
pub struct InterruptControllerInfo {
    pub node: DeviceTreeNode,
    pub compatible: Vec<String>,
    pub base_address: usize,
    pub interrupt_parent: Option<DeviceTreeNode>,
    pub phandle: u32,
}

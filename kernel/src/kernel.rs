
//! 内核核心对象
//!
//! 该模块定义了 HNX 内核的顶层 `Kernel` 结构体，它将持有所有核心
//! 内核服务的实例，例如内存管理器、设备管理器等。

use crate::memory::mmap_manager::MemoryMapManager;
use shared::sync::mutex::Mutex;

/// HNX 内核的顶层结构体
pub struct Kernel {
    /// 内存映射管理器
    pub memory_manager: Mutex<MemoryMapManager>,
}

impl Kernel {
    /// 创建一个新的 Kernel 实例
    ///
    /// 这个函数会在内核初始化时被调用，按顺序创建所有内核管理器。
    pub fn new() -> Self {
        let memory_manager = Mutex::new(MemoryMapManager::new());

        Self {
            memory_manager,
        }
    }
}

/// 全局内核实例
///
/// # 安全性
///
/// `static mut` 是不安全的，但我们在内核初始化时只对其进行一次写操作，
/// 并且之后的所有访问都通过安全的 `get_kernel()` 函数进行，因此这种用法是可控的。
static mut KERNEL: Option<Kernel> = None;

/// 初始化全局内核实例
pub fn init() {
    let kernel = Kernel::new();
    unsafe {
        KERNEL = Some(kernel);
    }
    crate::info!("Global kernel object initialized");
}

/// 获取对全局内核实例的安全引用
pub fn get_kernel() -> &'static Kernel {
    unsafe {
        KERNEL.as_ref().expect("Kernel has not been initialized")
    }
}
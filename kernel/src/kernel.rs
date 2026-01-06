use crate::console::ConsoleManager;
use crate::drivers::device_manager::DeviceManager;
use crate::loader::LoaderManager;
use crate::memory::MemoryManager;
use crate::process::ProcessManager;
use crate::security::capability::CapabilityManager;
use shared::sync::mutex::Mutex;

/// HNX 内核的顶层结构体
pub struct Kernel {
    /// 内存映射管理器
    pub memory_manager: Mutex<MemoryManager>,
    /// 设备管理器
    pub device_manager: Mutex<DeviceManager>,
    /// 能力管理器
    pub capability_manager: Mutex<CapabilityManager>,
    /// 进程管理器
    pub process_manager: Mutex<ProcessManager>,
    /// 加载器管理器
    pub loader_manager: Mutex<LoaderManager>,
    /// 控制台管理器
    pub console_manager: Mutex<ConsoleManager>,
}

impl Kernel {
    /// 创建一个新的 Kernel 实例
    ///
    /// 这个函数会在内核初始化时被调用，按顺序创建所有内核管理器。
    pub fn new() -> Self {
        let memory_manager = Mutex::new(MemoryManager::new());
        let device_manager = Mutex::new(DeviceManager::new());
        let capability_manager = Mutex::new(CapabilityManager::new());
        let process_manager = Mutex::new(ProcessManager::new());
        let loader_manager = Mutex::new(LoaderManager::new());
        let console_manager = Mutex::new(ConsoleManager::new());

        Self {
            memory_manager,
            device_manager,
            capability_manager,
            process_manager,
            loader_manager,
            console_manager,
        }
    }
}
/// 全局内核实例
///
/// # 安全性
///
/// `static mut` 是不安全的，但我们在内核初始化时只对其进行一次写操作，
/// 并且之后的所有访问都通过安全的 `get_kernel()` 函数进行，因此这种用法是可控的。
#[used]
static mut KERNEL: Option<Kernel> = None;

/// 初始化全局内核实例
pub fn init() {
    crate::info!("Creating kernel object...");
    let kernel = Kernel::new();
    unsafe {
        KERNEL = Some(kernel);
        crate::info!("KERNEL pointer = {:p}", &KERNEL as *const _);
        // 编译器屏障，确保存储不会被重新排序或优化掉
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
    crate::info!("Kernel object initialized");

    // ======== 初始化子系统管理器 ========
    // 控制台管理器初始化
    unsafe { KERNEL.as_mut().unwrap().console_manager.lock().init() };

    // 架构初始化 TODO
    crate::info!("Initializing architecture...");
    crate::arch::init();
    crate::info!("Architecture initialized");

    // 内存映射管理器初始化
    unsafe { KERNEL.as_mut().unwrap().memory_manager.lock().init() };

}

/// 获取对全局内核实例的安全引用
pub fn get_kernel() -> &'static Kernel {
    unsafe {
        crate::println!("[get_kernel] KERNEL.is_some() = {}", KERNEL.is_some());
        crate::println!("[get_kernel] KERNEL pointer = {:p}", &KERNEL as *const _);
        KERNEL.as_ref().expect("Kernel has not been initialized")
    }
}
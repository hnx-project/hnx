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
    pub memory_manager: &'static Mutex<MemoryManager>,
    /// 设备管理器
    pub device_manager: &'static Mutex<DeviceManager>,
    /// 能力管理器
    pub capability_manager: &'static Mutex<CapabilityManager>,
    /// 进程管理器
    pub process_manager: &'static Mutex<ProcessManager>,
    /// 加载器管理器
    pub loader_manager: &'static Mutex<LoaderManager>,
    /// 控制台管理器
    pub console_manager: &'static Mutex<ConsoleManager>,
}

impl Kernel {
    /// 创建一个新的 Kernel 实例
    ///
    /// 这个函数会在内核初始化时被调用，按顺序创建所有内核管理器。
    /// 注意：内存管理器使用全局单例，必须在调用此函数之前初始化。
    pub fn new() -> Self {
        let memory_manager = crate::memory::manager::get_memory_manager();
        let device_manager = crate::drivers::device_manager::get_device_manager();
        let capability_manager = crate::security::capability::get_capability_manager();
        let process_manager = crate::process::get_process_manager();
        let loader_manager = crate::loader::get_loader_manager();
        let console_manager = crate::console::get_console_manager();

        Self {
            memory_manager,
            device_manager,
            capability_manager,
            process_manager,
            loader_manager,
            console_manager,
        }
    }

    /// 初始化所有内核管理器
    pub fn init(&self) {
        // 控制台管理器初始化
        self.console_manager.lock().init();
        // 内存管理器初始化
        self.memory_manager.lock().init();
        // 其他管理器初始化可以按需添加
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
    crate::info!("Initializing kernel subsystems...");

    // 第一步：初始化分配器单例
    crate::info!("Initializing DMA allocator...");
    crate::memory::allocator::init_dma_allocator();

    // 第二步：初始化管理器单例
    crate::info!("Initializing memory manager...");
    crate::memory::manager::init_memory_manager();
    crate::info!("Initializing device manager...");
    crate::drivers::device_manager::init_device_manager();
    crate::info!("Initializing capability manager...");
    crate::security::capability::init_capability_manager();
    crate::info!("Initializing process manager...");
    crate::process::init_process_manager();
    crate::info!("Initializing loader manager...");
    crate::loader::init_loader_manager();
    // 控制台管理器已静态初始化，无需显式初始化

    // 第三步：创建内核实例（引用上述单例）
    crate::info!("Creating kernel object...");

    let kernel = Kernel::new();
    unsafe {
        KERNEL = Some(kernel);
        crate::info!("KERNEL pointer = {:p}", &KERNEL as *const _);
        // 编译器屏障，确保存储不会被重新排序或优化掉
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
    crate::info!("Kernel object initialized");

    // 第四步：初始化子系统（按依赖顺序）
    crate::info!("Initializing architecture...");
    crate::arch::init();
    crate::info!("Architecture initialized");

    // 控制台管理器初始化
    crate::console::init();
    // 内存管理器初始化
    crate::memory::manager::get_memory_manager().lock().init();
    // 其他管理器初始化（如果有需要）
    // ...

    crate::info!("Kernel initialization complete");
}

/// 获取对全局内核实例的安全引用
pub fn get_kernel() -> &'static Kernel {
    unsafe {
        crate::println!("[get_kernel] KERNEL.is_some() = {}", KERNEL.is_some());
        crate::println!("[get_kernel] KERNEL pointer = {:p}", &KERNEL as *const _);
        KERNEL.as_ref().expect("Kernel has not been initialized")
    }
}
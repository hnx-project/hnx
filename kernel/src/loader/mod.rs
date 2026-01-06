//! Minimal Bootstrap Loader
//!
//! **CRITICAL**: This module is ONLY for loading the first user space process (init).
//! All subsequent program loading MUST be done by the user space Loader Service.
//!
//! ## Architecture
//!
//! ```
//! Kernel Bootstrap:
//!   1. Discover initrd in memory
//!   2. Load ONLY the init process from initrd
//!   3. Start init process
//!
//! User Space (init process):
//!   1. Start Loader Service (endpoint 3)
//!   2. Start VFS Service (endpoint 1)  
//!   3. Start other services
//!
//! All Future Loads:
//!   User → Loader Service → VFS Service → Kernel syscalls
//! ```
//!
//! ## Why This Exists
//!
//! Chicken-and-egg problem: We need a Loader service to load programs,
//! but we need to load the Loader service first. This module solves that
//! by providing a minimal bootstrap that only loads the init binary.

pub mod initrd;
pub mod bootstrap_elf;

pub use initrd::{get_initrd_base, get_initrd_size, find_file_in_initrd};

use core::sync::atomic::{AtomicUsize, Ordering};
use shared::sync::mutex::Mutex;

/// 加载器管理器
///
/// 管理 initrd 状态和引导加载相关功能
pub struct LoaderManager {
    /// initrd 内存基地址
    initrd_base: AtomicUsize,
    /// initrd 大小（字节）
    initrd_size: AtomicUsize,
}

/// 全局加载器管理器实例（临时，迁移期间使用）
// static mut LOADER_MANAGER: LoaderManager = LoaderManager::new();

/// 初始化加载器管理器
pub fn init(dtb_ptr: usize) {
    // 使用全局加载器管理器单例
    crate::println!("[loader::init] Calling get_loader_manager()...");
    get_loader_manager().lock().init(dtb_ptr);
    crate::println!("[loader::init] Done");
}

impl LoaderManager {
    /// 创建新的加载器管理器
    pub const fn new() -> Self {
        Self {
            initrd_base: AtomicUsize::new(0),
            initrd_size: AtomicUsize::new(0),
        }
    }

    /// 初始化 initrd（从 DTB 中发现）
    pub fn init(&self, dtb_ptr: usize) {
        let (base, size) = self::initrd::discover_initrd(dtb_ptr);
        self.initrd_base.store(base, Ordering::SeqCst);
        self.initrd_size.store(size, Ordering::SeqCst);
        crate::info!("loader: initrd discovered at 0x{:X}, size {} bytes", base, size);
    }

    /// 获取 initrd 基地址
    pub fn get_initrd_base(&self) -> usize {
        self.initrd_base.load(Ordering::SeqCst)
    }

    /// 获取 initrd 大小
    pub fn get_initrd_size(&self) -> usize {
        self.initrd_size.load(Ordering::SeqCst)
    }

    /// 获取 initrd 数据切片
    pub fn get_initrd_slice(&self) -> &'static [u8] {
        let base = self.get_initrd_base();
        let size = self.get_initrd_size();

        if base == 0 || size == 0 {
            return &[];
        }

        unsafe {
            let slice = core::slice::from_raw_parts(base as *const u8, size);

            // 检查 gzip 魔数 (0x1F 0x8B)
            if slice.len() >= 2 && slice[0] == 0x1F && slice[1] == 0x8B {
                crate::error!("loader: initrd is gzip compressed, kernel cannot decompress");
                crate::error!("loader: please use uncompressed initrd or implement gzip decompression");
                return &[];
            }

            slice
        }
    }
}

/// Bootstrap the first user process from initrd
///
/// This function ONLY loads the init process. It's hardcoded and minimal.
/// Returns (entry_point, stack_pointer, page_table_base) or error.
pub fn bootstrap_init_process() -> Result<(usize, usize, usize), ()> {
    crate::info!("loader: bootstrapping init process from initrd");

    let loader = crate::kernel::get_kernel().loader_manager.lock();
    let initrd = loader.get_initrd_slice();
    if initrd.is_empty() {
        crate::error!("loader: no initrd found - cannot boot");
        return Err(());
    }

    crate::debug!("loader: initrd at 0x{:X}, size {} bytes",
                 loader.get_initrd_base(), loader.get_initrd_size());

    let init_elf = find_file_in_cpio(initrd, "init")?;

    crate::debug!("loader: found init ELF, size {} bytes", init_elf.len());

    let loader = bootstrap_elf::BootstrapElfLoader::new(init_elf)
        .map_err(|e| {
            crate::error!("loader: invalid init ELF: {}", e);
        })?;

    loader.load_init().map_err(|e| {
        crate::error!("loader: failed to load init: {}", e);
    })
}

/// Load a service from initrd (minimal loader - only for bootstrap)
///
/// This is a MINIMAL loader for bootstrapping core services before
/// the user space Loader service is available.
///
/// # Arguments
/// * `path` - Path to service ELF in initrd (e.g., "/services/vfs-service")
///
/// # Returns
/// * (entry_point, stack_pointer, page_table_base) or error
pub fn spawn_service_from_initrd(path: &str) -> Result<(usize, usize, usize), ()> {
    crate::info!("loader: spawning service from initrd: {}", path);

    let loader = crate::kernel::get_kernel().loader_manager.lock();
    let initrd = loader.get_initrd_slice();
    if initrd.is_empty() {
        crate::error!("loader: no initrd found");
        return Err(());
    }

    let elf_data = find_file_in_cpio(initrd, path)?;
    crate::debug!("loader: found service ELF '{}', size {} bytes", path, elf_data.len());

    let loader = bootstrap_elf::BootstrapElfLoader::new(elf_data)
        .map_err(|e| {
            crate::error!("loader: invalid service ELF '{}': {}", path, e);
        })?;

    loader.load_init().map_err(|e| {
        crate::error!("loader: failed to load service '{}': {}", path, e);
    })
}

fn find_file_in_cpio<'a>(cpio_data: &'a [u8], target_name: &str) -> Result<&'a [u8], ()> {
    let mut offset = 0;
    let cpio_len = cpio_data.len();

    while offset < cpio_len {
        // 确保有足够的空间容纳CPIO头部
        if offset + 110 > cpio_len {
            crate::debug!("loader: insufficient space for CPIO header at offset {}", offset);
            break;
        }

        // 检查CPIO魔数
        let magic = &cpio_data[offset..offset + 6];
        if magic != b"070701" && magic != b"070702" {
            crate::debug!("loader: invalid CPIO magic at offset {}: {:?}", offset, magic);
            break;
        }

        // 读取namesize和filesize
        let namesize = match read_hex(&cpio_data, offset + 94, 8) {
            Ok(n) => n as usize,
            Err(_) => {
                crate::debug!("loader: failed to read namesize at offset {}", offset);
                break;
            }
        };

        let filesize = match read_hex(&cpio_data, offset + 54, 8) {
            Ok(f) => f as usize,
            Err(_) => {
                crate::debug!("loader: failed to read filesize at offset {}", offset);
                break;
            }
        };

        // 检查合理性
        if namesize == 0 || namesize > 4096 {
            crate::debug!("loader: invalid namesize {} at offset {}", namesize, offset);
            break;
        }

        // filesize合理性检查（最大10MB）
        if filesize > 10 * 1024 * 1024 {
            crate::debug!("loader: suspicious filesize {} at offset {}", filesize, offset);
            // 继续尝试，可能是设备节点或特殊文件
        }

        let name_offset = offset + 110;
        if name_offset + namesize > cpio_len {
            crate::debug!("loader: name extends beyond initrd at offset {}", offset);
            break;
        }

        // 读取文件名（跳过最后的null终止符）
        let name_end = name_offset + namesize - 1;
        if name_end >= cpio_len {
            crate::debug!("loader: name extends beyond initrd (adjusted) at offset {}", offset);
            break;
        }

        let name_bytes = &cpio_data[name_offset..name_end];
        let name = match core::str::from_utf8(name_bytes) {
            Ok(n) => n,
            Err(_) => {
                crate::debug!("loader: invalid UTF-8 in name at offset {}", offset);
                break;
            }
        };

        // 调试日志：记录每个CPIO条目
        crate::debug!("loader: CPIO entry at offset {}: name='{}', namesize={}, filesize={}", offset, name, namesize, filesize);

        if name == "TRAILER!!!" {
            break;
        }

        // 计算对齐的偏移量
        let header_and_name = 110 + namesize;
        let aligned_offset = (offset + header_and_name + 3) & !3;
        let data_end = aligned_offset + filesize;

        // 检查边界
        if aligned_offset > cpio_len {
            crate::debug!("loader: aligned_offset {} > cpio_len {} at offset {}",
                         aligned_offset, cpio_len, offset);
            break;
        }

        // 检查是否为目标文件
        let normalized_target = if target_name.starts_with("./") {
            &target_name[2..]
        } else if target_name.starts_with('/') {
            &target_name[1..]
        } else {
            target_name
        };

        let normalized_name = if name.starts_with("./") {
            &name[2..]
        } else if name.starts_with('/') {
            &name[1..]
        } else {
            name
        };

        crate::debug!("loader: comparing: normalized_name='{}', normalized_target='{}'", normalized_name, normalized_target);
        if normalized_name == normalized_target {
            crate::info!("loader: found '{}' in initrd, data_end={}, cpio_len={}, filesize={}",
                         name, data_end, cpio_len, filesize);

            if data_end <= cpio_len {
                return Ok(&cpio_data[aligned_offset..data_end]);
            } else {
                // 找到文件但data_end无效，这表示CPIO条目损坏
                crate::error!("loader: '{}' found but data_end {} exceeds initrd size {}",
                            name, data_end, cpio_len);
                // 尝试从aligned_offset读取到cpio_len的剩余部分
                // 这可能是最后的文件，filesize可能包含填充
                if aligned_offset < cpio_len {
                    crate::warn!("loader: using truncated data for '{}'", name);
                    return Ok(&cpio_data[aligned_offset..cpio_len]);
                }
                return Err(());
            }
        }

        // 更新偏移到下一个条目
        let next_offset = if data_end <= cpio_len {
            (data_end + 3) & !3
        } else {
            // data_end无效，尝试基于名称和基本对齐前进
            crate::debug!("loader: invalid data_end at offset {}, advancing cautiously", offset);
            let min_advance = (header_and_name + 3) & !3;
            offset + min_advance
        };

        if next_offset <= offset {
            crate::debug!("loader: offset not advancing ({} -> {}), breaking", offset, next_offset);
            break;
        }

        offset = next_offset;
    }

    crate::error!("loader: '{}' not found in initrd CPIO archive", target_name);
    Err(())
}

fn find_init_in_cpio(cpio_data: &[u8]) -> Result<&[u8], ()> {
    find_file_in_cpio(cpio_data, "init")
}

fn read_hex(data: &[u8], offset: usize, len: usize) -> Result<u32, ()> {
    if data.len() < offset + len {
        return Err(());
    }

    let slice = &data[offset..offset + len];

    // 尝试直接解析
    if let Ok(hex_str) = core::str::from_utf8(slice) {
        if let Ok(value) = u32::from_str_radix(hex_str, 16) {
            return Ok(value);
        }

        // 如果失败，尝试修剪空白字符
        let trimmed = hex_str.trim_matches(|c: char| c.is_whitespace() || c == '\0');
        if let Ok(value) = u32::from_str_radix(trimmed, 16) {
            return Ok(value);
        }

        // 如果仍然失败，尝试只提取十六进制字符
        // 使用固定大小的数组，因为CPIO头部字段长度固定（最多8个字符）
        let mut hex_chars = [0u8; 8];
        let mut hex_len = 0;

        for &b in slice {
            let c = b as char;
            if c.is_ascii_hexdigit() && hex_len < hex_chars.len() {
                hex_chars[hex_len] = b;
                hex_len += 1;
            } else if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\0' {
                // 跳过空白和空字符
                continue;
            } else if hex_len > 0 {
                // 非十六进制字符，停止提取
                break;
            }
        }

        if hex_len > 0 {
            if let Ok(hex_str) = core::str::from_utf8(&hex_chars[..hex_len]) {
                if let Ok(value) = u32::from_str_radix(hex_str, 16) {
                    return Ok(value);
                }
            }
        }
    }

    // 所有尝试都失败
    Err(())
}

/// 全局加载器管理器单例实例
///
/// # 安全性
///
/// `static mut` 是不安全的，但我们在初始化时只对其进行一次写操作，
/// 并且之后的所有访问都通过安全的 `get_loader_manager()` 函数进行，因此这种用法是可控的。
#[used]
static mut LOADER_MANAGER: Option<Mutex<LoaderManager>> = None;

/// 初始化全局加载器管理器单例实例
pub fn init_loader_manager() {
    crate::info!("loader: initializing global loader manager singleton");
    let manager = Mutex::new(LoaderManager::new());
    unsafe {
        LOADER_MANAGER = Some(manager);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
    crate::info!("loader: global loader manager singleton initialized");
}

/// 获取对全局加载器管理器单例实例的安全引用
pub fn get_loader_manager() -> &'static Mutex<LoaderManager> {
    unsafe {
        LOADER_MANAGER.as_ref().expect("Loader manager has not been initialized")
    }
}

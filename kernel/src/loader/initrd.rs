use crate::arch;

// 静态变量已迁移到 LoaderManager 中，这些函数现在委托给 LOADER_MANAGER
// 为了保持向后兼容，保留函数签名但更改实现

pub fn init(dtb_ptr: usize) {
    // 委托给父模块中的全局 LoaderManager
    super::init(dtb_ptr);
}

pub fn get_initrd_base() -> usize {
    // 委托给父模块中的全局 LoaderManager
    crate::kernel::get_kernel().loader_manager.lock().get_initrd_base()
}

pub fn get_initrd_size() -> usize {
    // 委托给父模块中的全局 LoaderManager
    crate::kernel::get_kernel().loader_manager.lock().get_initrd_size()
}

pub fn get_initrd_slice() -> &'static [u8] {
    // 委托给父模块中的全局 LoaderManager
    crate::kernel::get_kernel().loader_manager.lock().get_initrd_slice()
}

pub fn find_file_in_initrd(_path: &str) -> Option<&'static [u8]> {
    // TODO: Implement CPIO parser in user space Loader service
    // Kernel should not parse file systems
    None
}

pub fn discover_initrd(_dtb_ptr: usize) -> (usize, usize) {
    // 方案：使用 QEMU -device loader 将 initrd 加载到固定地址
    const INITRD_FIXED_ADDR: usize = 0x4200_0000;
    
    crate::debug!("loader/initrd: checking fixed address 0x{:X} (QEMU -device loader)", INITRD_FIXED_ADDR);
    
    // 检查固定地址是否有 CPIO 魔数
    let probe = unsafe {
        core::slice::from_raw_parts(INITRD_FIXED_ADDR as *const u8, 256)
    };
    
    if probe.len() >= 6 && (&probe[0..6] == b"070701" || &probe[0..6] == b"070702") {
        let size = detect_cpio_size(INITRD_FIXED_ADDR);
        crate::debug!("loader/initrd: ✓ found CPIO at fixed address 0x{:X}, size {} bytes", INITRD_FIXED_ADDR, size);
        return (INITRD_FIXED_ADDR, size);
    }
    
    crate::debug!("loader/initrd: no CPIO at fixed address 0x{:X}", INITRD_FIXED_ADDR);
    crate::debug!("loader/initrd: expected magic bytes b\"070701\" or b\"070702\", got: {:02X?}", &probe[0..6.min(probe.len())]);
    
    // 后备：暴力扫描（以防 QEMU 用了不同的地址）
    try_extensive_memory_scan()
}

/// 检测 CPIO 归档的大小
/// 
/// 从给定的内存基地址开始扫描，查找 CPIO 归档的结束位置。
/// 
/// # 参数
/// 
/// * `base` - CPIO 归档的内存基地址
/// 
/// # 返回值
/// 
/// 如果找到 CPIO 归档的结束位置，则返回该位置的偏移量；否则返回 0。
fn detect_cpio_size(base: usize) -> usize {
    // 最大 20MB 限制 ？ 防止无限循环
    // TODO 是否合理？如果加入更多的服务，可能需要增加这个限制？
    const MAX_SIZE: usize = 30 * 1024 * 1024;  // 最大 30MB
    let data = unsafe {
        core::slice::from_raw_parts(base as *const u8, MAX_SIZE)
    };
    
    let mut offset = 0;
    while offset < data.len() {
        if offset + 110 > data.len() {
            break;
        }
        
        let magic = &data[offset..offset + 6];
        if magic != b"070701" && magic != b"070702" {
            break;
        }
        
        // 读取文件名大小和文件大小
        let namesize = match read_hex(&data, offset + 94, 8) {
            Ok(n) => n as usize,
            Err(_) => break,
        };
        crate::debug!("loader/initrd: detect: offset={}, namesize={}", offset, namesize);
        
        let name_offset = offset + 110;
        if name_offset + namesize > data.len() {
            break;
        }
        
        let name_bytes = &data[name_offset..name_offset + namesize - 1];
        if let Ok(name) = core::str::from_utf8(name_bytes) {
            if name == "TRAILER!!!" {
                // 找到结尾标记
                let header_and_name = 110 + namesize;
                let aligned_offset = (offset + header_and_name + 3) & !3;
                crate::debug!("loader/initrd: detect_cpio_size: found TRAILER!!! at offset {}, returning size {}", offset, aligned_offset);
                return aligned_offset;
            }
        }
        
        // 移动到下一个条目
        let filesize = match read_hex(&data, offset + 54, 8) {
            Ok(n) => n as usize,
            Err(_) => break,
        };
        crate::debug!("loader/initrd: detect: offset={}, filesize={}", offset, filesize);

        let header_and_name = 110 + namesize;
        let aligned_offset = (offset + header_and_name + 3) & !3;
        let data_end = aligned_offset + filesize;

        // 计算下一个偏移量，处理无效的filesize
        let next_offset = if data_end <= data.len() {
            (data_end + 3) & !3
        } else {
            // data_end无效，使用最小前进量
            let min_advance = (header_and_name + 3) & !3;
            offset + min_advance
        };
        crate::debug!("loader/initrd: detect: aligned_offset={}, data_end={}, next_offset={}", aligned_offset, data_end, next_offset);
        offset = next_offset;
    }
    
    // 如果没找到 TRAILER!!!，返回默认大小
    crate::debug!("loader/initrd: detect_cpio_size: TRAILER!!! not found, returning default size 131072");
    131072
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

fn try_extensive_memory_scan() -> (usize, usize) {
    // 暴力扫描整个 512MB RAM，每 64KB 一次
    crate::debug!("loader/initrd: brute-force scanning 512MB RAM for CPIO magic...");
    
    let ram_start = 0x4000_0000usize;
    let ram_end = 0x6000_0000usize;
    let step = 0x10000; // 64KB
    
    let total_locs = (ram_end - ram_start) / step;
    let mut scanned = 0;
    
    for addr in (ram_start..ram_end).step_by(step) {
        scanned += 1;
        
        let probe = unsafe {
            core::slice::from_raw_parts(addr as *const u8, 256)
        };
        
        // 检查 CPIO newc ASCII 魔数
        if probe.len() >= 6 && (&probe[0..6] == b"070701" || &probe[0..6] == b"070702") {
            crate::debug!("loader/initrd: ✓ CPIO found at 0x{:X} (scanned {}/{})", addr, scanned, total_locs);
            return (addr, 131072);
        }
        
        // 每扫描 1024 个位置报告一次进度
        if scanned % 1024 == 0 {
            crate::debug!("loader/initrd: ...scanned {}/{} locations (current: 0x{:X})", scanned, total_locs, addr);
        }
    }
    
    crate::warn!("loader/initrd: ✗ no CPIO found after scanning {} locations in 512MB RAM", scanned);
    (0, 0)
}
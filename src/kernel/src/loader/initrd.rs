use crate::arch;

static mut INITRD_BASE: usize = 0;
static mut INITRD_SIZE: usize = 0;

pub fn init(dtb_ptr: usize) {
    unsafe {
        let (base, size) = discover_initrd(dtb_ptr);
        INITRD_BASE = base;
        INITRD_SIZE = size;
        crate::info!("loader/initrd: discovered at 0x{:X}, size {} bytes", base, size);
    }
}

pub fn get_initrd_base() -> usize {
    unsafe { INITRD_BASE }
}

pub fn get_initrd_size() -> usize {
    unsafe { INITRD_SIZE }
}

pub fn get_initrd_slice() -> &'static [u8] {
    unsafe {
        if INITRD_BASE == 0 || INITRD_SIZE == 0 {
            return &[];
        }
        core::slice::from_raw_parts(INITRD_BASE as *const u8, INITRD_SIZE)
    }
}

pub fn find_file_in_initrd(_path: &str) -> Option<&'static [u8]> {
    // TODO: Implement CPIO parser in user space Loader service
    // Kernel should not parse file systems
    None
}

fn discover_initrd(_dtb_ptr: usize) -> (usize, usize) {
    // 方案：使用 QEMU -device loader 将 initrd 加载到固定地址
    const INITRD_FIXED_ADDR: usize = 0x4200_0000;
    
    crate::info!("loader/initrd: checking fixed address 0x{:X} (QEMU -device loader)", INITRD_FIXED_ADDR);
    
    // 检查固定地址是否有 CPIO 魔数
    let probe = unsafe {
        core::slice::from_raw_parts(INITRD_FIXED_ADDR as *const u8, 256)
    };
    
    if probe.len() >= 6 && (&probe[0..6] == b"070701" || &probe[0..6] == b"070702") {
        let size = detect_cpio_size(INITRD_FIXED_ADDR);
        crate::info!("loader/initrd: ✓ found CPIO at fixed address 0x{:X}, size {} bytes", INITRD_FIXED_ADDR, size);
        return (INITRD_FIXED_ADDR, size);
    }
    
    crate::warn!("loader/initrd: no CPIO at fixed address 0x{:X}", INITRD_FIXED_ADDR);
    crate::info!("loader/initrd: expected magic bytes b\"070701\" or b\"070702\", got: {:02X?}", &probe[0..6.min(probe.len())]);
    
    // 后备：暴力扫描（以防 QEMU 用了不同的地址）
    try_extensive_memory_scan()
}

fn detect_cpio_size(base: usize) -> usize {
    const MAX_SIZE: usize = 10 * 1024 * 1024;  // 最大 10MB
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
                return aligned_offset;
            }
        }
        
        // 移动到下一个条目
        let filesize = match read_hex(&data, offset + 54, 8) {
            Ok(n) => n as usize,
            Err(_) => break,
        };
        
        let header_and_name = 110 + namesize;
        let aligned_offset = (offset + header_and_name + 3) & !3;
        let data_end = aligned_offset + filesize;
        offset = (data_end + 3) & !3;
    }
    
    // 如果没找到 TRAILER!!!，返回默认大小
    131072
}

fn read_hex(data: &[u8], offset: usize, len: usize) -> Result<u32, ()> {
    if data.len() < offset + len {
        return Err(());
    }
    
    let hex_str = core::str::from_utf8(&data[offset..offset + len]).map_err(|_| ())?;
    u32::from_str_radix(hex_str, 16).map_err(|_| ())
}

fn try_extensive_memory_scan() -> (usize, usize) {
    // 暴力扫描整个 512MB RAM，每 64KB 一次
    crate::info!("loader/initrd: brute-force scanning 512MB RAM for CPIO magic...");
    
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
            crate::info!("loader/initrd: ✓ CPIO found at 0x{:X} (scanned {}/{})", addr, scanned, total_locs);
            return (addr, 131072);
        }
        
        // 每扫描 1024 个位置报告一次进度
        if scanned % 1024 == 0 {
            crate::info!("loader/initrd: ...scanned {}/{} locations (current: 0x{:X})", scanned, total_locs, addr);
        }
    }
    
    crate::warn!("loader/initrd: ✗ no CPIO found after scanning {} locations in 512MB RAM", scanned);
    (0, 0)
}
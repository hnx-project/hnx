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

fn discover_initrd(dtb_ptr: usize) -> (usize, usize) {
    if dtb_ptr != 0 {
        if let Some((base, size)) = parse_initrd_from_dtb(dtb_ptr) {
            return (base, size);
        }
    }
    
    try_extensive_memory_scan()
}

fn parse_initrd_from_dtb(_dtb_ptr: usize) -> Option<(usize, usize)> {
    // TODO: Parse DTB to find /chosen/linux,initrd-start and linux,initrd-end
    None
}

fn try_extensive_memory_scan() -> (usize, usize) {
    const SCAN_BASES: [usize; 20] = [
        0x4800_0000, 0x4700_0000, 0x4600_0000, 0x4500_0000,
        0x4400_0000, 0x4300_0000, 0x4200_0000, 0x4100_0000,
        0x4000_0000, 0x4080_0000, 0x4040_0000, 0x4020_0000,
        0x4010_0000, 0x4008_0000, 0x4004_0000, 0x4002_0000,
        0x4001_0000, 0x4000_8000, 0x4000_4000, 0x4000_2000,
    ];
    
    crate::info!("loader/initrd: scanning {} memory locations...", SCAN_BASES.len());
    
    for &base in &SCAN_BASES {
        let probe = unsafe {
            core::slice::from_raw_parts(base as *const u8, 256)
        };
        
        if probe.len() >= 6 && (&probe[0..6] == b"070701" || &probe[0..6] == b"070702") {
            crate::info!("loader/initrd: ✓ found CPIO newc at 0x{:X}", base);
            return (base, 131072);
        }
        
        if probe.len() >= 2 && &probe[0..2] == b"\x1f\x8b" {
            crate::info!("loader/initrd: ✓ found gzip at 0x{:X}", base);
            return (base, 65536);
        }
    }
    
    crate::warn!("loader/initrd: ✗ no initrd found after scanning {} locations", SCAN_BASES.len());
    crate::warn!("loader/initrd: QEMU may not support -initrd, or initrd wasn't loaded");
    crate::info!("loader/initrd: try: qemu-system-aarch64 ... -device loader,file=initrd.cpio,addr=0x42000000");
    
    (0, 0)
}
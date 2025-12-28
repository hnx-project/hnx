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

/// Bootstrap the first user process from initrd
///
/// This function ONLY loads the init process. It's hardcoded and minimal.
/// Returns (entry_point, stack_pointer, page_table_base) or error.
pub fn bootstrap_init_process() -> Result<(usize, usize, usize), ()> {
    crate::info!("loader: bootstrapping init process from initrd");

    let initrd = initrd::get_initrd_slice();
    if initrd.is_empty() {
        crate::error!("loader: no initrd found - cannot boot");
        return Err(());
    }

    crate::debug!("loader: initrd at 0x{:X}, size {} bytes",
                 initrd::get_initrd_base(), initrd::get_initrd_size());

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

    let initrd = initrd::get_initrd_slice();
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

    while offset < cpio_data.len() {
        if offset + 110 > cpio_data.len() {
            break;
        }

        let magic = &cpio_data[offset..offset + 6];
        if magic != b"070701" && magic != b"070702" {
            break;
        }

        let namesize = read_hex(&cpio_data, offset + 94, 8)? as usize;
        let filesize = read_hex(&cpio_data, offset + 54, 8)? as usize;

        let name_offset = offset + 110;
        if name_offset + namesize > cpio_data.len() {
            break;
        }

        let name_bytes = &cpio_data[name_offset..name_offset + namesize - 1];
        let name = core::str::from_utf8(name_bytes).map_err(|_| ())?;

        if name == "TRAILER!!!" {
            break;
        }

        let header_and_name = 110 + namesize;
        let aligned_offset = (offset + header_and_name + 3) & !3;
        let data_end = aligned_offset + filesize;

        // Check if this is the file we're looking for
        // CPIO paths can be "./init", "init", "/services/vfs-service", etc.
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

        if normalized_name == normalized_target {
            crate::debug!("loader: found '{}' in initrd", name);
            if data_end <= cpio_data.len() {
                return Ok(&cpio_data[aligned_offset..data_end]);
            }
        }

        offset = (data_end + 3) & !3;
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
    
    let hex_str = core::str::from_utf8(&data[offset..offset + len]).map_err(|_| ())?;
    u32::from_str_radix(hex_str, 16).map_err(|_| ())
}

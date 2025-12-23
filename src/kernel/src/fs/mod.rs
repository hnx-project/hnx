extern crate alloc;

pub mod cpio;
pub mod elf;
pub mod ohlink;
pub mod vfs;
pub mod ramfs;
pub mod dirent;
pub mod path;
pub mod symlink;
pub mod acl;
pub mod xattr;
pub mod snapshot;

use crate::arch::boot::get_boot_info;
use crate::arch::common::mmu::MmuFlags;
use crate::console;
use crate::memory::physical::{alloc_pages, PhysAddr};
use crate::memory::virtual_::map;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use alloc::string::String;
use crate::fs::vfs::{FileSystem, WritableFileSystem};

pub fn init() {
    crate::info!("fs init");
    let boot = crate::arch::boot::get_boot_info();
    crate::info!("fs mapping initrd at 0x{:016X}", boot.initrd_start as usize);
    crate::memory::virtual_::map(
        boot.initrd_start as usize,
        boot.initrd_start as usize,
        MmuFlags::READ,
    );
    crate::memory::virtual_::map(
        crate::memory::virtual_::KERNEL_BASE + boot.initrd_start as usize,
        boot.initrd_start as usize,
        MmuFlags::READ,
    );
    crate::info!(
        "fs initrd: using identity low-VA access at 0x{:016X}",
        boot.initrd_start as usize
    );
    // unsafe { initrd_build_minimal(); }
    crate::info!("fs ok");
    let _ = crate::kernel_core::ipc::register(crate::kernel_core::ipc::SERVICE_VFS, ipc_handler);
}

const INITRD_MAGIC: [u8; 4] = *b"IRDT";
const NAME_MAX: usize = 64;
const INITRD_VA_BASE: usize = 0xFFFF_8000_1000_0000;
// Window size to scan for externally loaded initrd/ELF when size is unknown
pub const INITRD_SCAN_SIZE: usize = 0x0020_0000; // 2MB

#[repr(C)]
struct InitrdHeader {
    magic: [u8; 4],
    count: u32,
}

#[repr(C)]
struct DirEntry {
    name: [u8; NAME_MAX],
    offset: u32,
    size: u32,
}

unsafe fn write_bytes(dst: *mut u8, src: &[u8]) {
    core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
}

unsafe fn initrd_build_minimal() {
    crate::info!("fs/initrd building internal initrd...");
    let p = alloc_pages(2);
    if p.is_none() {
        return;
    }
    let paddr = p.unwrap();
    // map two pages into kernel VA
    map(
        INITRD_VA_BASE,
        paddr,
        MmuFlags::READ.combine(MmuFlags::WRITE),
    );
    map(
        INITRD_VA_BASE + 0x1000,
        paddr + 0x1000,
        MmuFlags::READ.combine(MmuFlags::WRITE),
    );
    INITRD_FAKE_BASE.store(INITRD_VA_BASE, Ordering::Relaxed);
    INITRD_FAKE_SIZE.store(0x2000, Ordering::Relaxed);
    let base = INITRD_FAKE_BASE.load(Ordering::Relaxed) as *mut u8;

    // Layout:
    // [0x0000] InitrdHeader
    // [0x0008] DirEntry[1]
    // [0x0100] hello.exe.ohlink (80 + 4096 bytes)
    let dir_offset = 0x0008usize;
    let file_offset = 0x0100usize;

    // Header
    let hdr = InitrdHeader {
        magic: INITRD_MAGIC,
        count: 1,
    };
    core::ptr::write_unaligned(base as *mut InitrdHeader, hdr);

    // Directory entry
    let mut entry = DirEntry {
        name: [0; NAME_MAX],
        offset: file_offset as u32,
        size: 0,
    };
    let path = b"/bin/hello.exe.ohlink";
    let n = core::cmp::min(NAME_MAX, path.len());
    core::ptr::copy_nonoverlapping(path.as_ptr(), entry.name.as_mut_ptr(), n);

    // Build minimal Ohlink file in-place
    // Header (32 bytes)
    let oh_magic: u32 = 0x0f112234;
    let ncmds: u32 = 1;
    let sizeofcmds: u32 = 48; // 8 (cmd+cmdsize) + 40 (segment fields)
    let fileoff: u64 = 1024; // keep within 2 pages total (0x2000)
    let vmaddr: u64 = 0x4000_0000;
    let vmsize: u64 = 4096;
    let filesize: u64 = 4096;

    // Write Ohlink header
    {
        let p = base.add(file_offset);
        let mut tmp = [0u8; 32];
        tmp[0..4].copy_from_slice(&oh_magic.to_le_bytes());
        tmp[4..8].copy_from_slice(&0u32.to_le_bytes()); // cpu_type
        tmp[8..12].copy_from_slice(&0u32.to_le_bytes()); // cpu_subtype
        tmp[12..16].copy_from_slice(&0u32.to_le_bytes()); // file_type
        tmp[16..20].copy_from_slice(&ncmds.to_le_bytes());
        tmp[20..24].copy_from_slice(&sizeofcmds.to_le_bytes());
        tmp[24..28].copy_from_slice(&0u32.to_le_bytes()); // flags
        tmp[28..32].copy_from_slice(&0u32.to_le_bytes()); // reserved
        write_bytes(p, &tmp);
    }

    // Write LC_SEGMENT_64 command
    {
        let p = base.add(file_offset + 32);
        let mut tmp = [0u8; 48];
        tmp[0..4].copy_from_slice(&0x19u32.to_le_bytes()); // cmd = LC_SEGMENT_64
        tmp[4..8].copy_from_slice(&sizeofcmds.to_le_bytes()); // cmdsize
        tmp[8..16].copy_from_slice(&vmaddr.to_le_bytes()); // vmaddr
        tmp[16..24].copy_from_slice(&vmsize.to_le_bytes()); // vmsize
        tmp[24..32].copy_from_slice(&fileoff.to_le_bytes()); // fileoff
        tmp[32..40].copy_from_slice(&filesize.to_le_bytes()); // filesize
        tmp[40..44].copy_from_slice(&5u32.to_le_bytes());
        write_bytes(p, &tmp);
    }

    // Code page: print via svc #1, then exit via svc #2
    {
        let p = base.add(file_offset + fileoff as usize);
        let mut page = [0u8; 4096];
        // adrp x1, page-base; add x1, x1, #0x94 => &msg
        page[0..4].copy_from_slice(&0x90000001u32.to_le_bytes());
        page[4..8].copy_from_slice(&0x91025021u32.to_le_bytes());
        // movz x2, #13
        page[8..12].copy_from_slice(&0xD28001A2u32.to_le_bytes());
        // movz x0, #1 (fd=1)
        page[12..16].copy_from_slice(&0xD2800020u32.to_le_bytes());
        // svc #1 (write)
        page[16..20].copy_from_slice(&0xD4000021u32.to_le_bytes());
        // movz x0, #0
        page[20..24].copy_from_slice(&0xD2800000u32.to_le_bytes());
        // svc #2 (exit)
        page[24..28].copy_from_slice(&0xD4000041u32.to_le_bytes());
        // b . (stay in place)
        page[28..32].copy_from_slice(&0x14000000u32.to_le_bytes());
        // message (13 bytes)
        let msg = b"Hello EL0!\nOK\n";
        let mlen = core::cmp::min(13, msg.len());
        page[0x94..0x94 + mlen].copy_from_slice(&msg[..mlen]);
        write_bytes(p, &page);
    }

    // Finalize dir entry size
    entry.size = (fileoff as usize + 4096) as u32;
    core::ptr::write_unaligned(base.add(dir_offset) as *mut DirEntry, entry);
    let b = INITRD_FAKE_BASE.load(Ordering::Relaxed);
    let s = INITRD_FAKE_SIZE.load(Ordering::Relaxed);
    crate::info!(
        "fs/initrd internal initrd ready at 0x{:016X} size=0x{:X}",
        b,
        s
    );
}

#[no_mangle]
pub static INITRD_FAKE_BASE: AtomicUsize = AtomicUsize::new(0);
#[no_mangle]
pub static INITRD_FAKE_SIZE: AtomicUsize = AtomicUsize::new(0);

pub fn initrd_bytes<'a>() -> &'a [u8] {
    unsafe {
        let boot = crate::arch::boot::get_boot_info();
        let base = INITRD_FAKE_BASE.load(Ordering::Relaxed);
        let size = INITRD_FAKE_SIZE.load(Ordering::Relaxed);
        if base != 0 {
            core::slice::from_raw_parts(base as *const u8, size)
        } else {
            let sz = if boot.initrd_size == 0 {
                INITRD_SCAN_SIZE
            } else {
                boot.initrd_size as usize
            };
            core::slice::from_raw_parts(
                (crate::memory::virtual_::KERNEL_BASE + boot.initrd_start as usize) as *const u8,
                sz,
            )
        }
    }
}

/// List directory contents
pub fn list_directory(bytes: &[u8], path: &str) -> Vec<String> {
    let mut entries = Vec::new();
    
    // Make sure path ends with a slash for consistent matching
    let normalized_path = if path.ends_with('/') || path.is_empty() {
        String::from(path)
    } else {
        let mut s = String::from(path);
        s.push('/');
        s
    };
    
    // IRDT format
    if bytes.len() >= 8 && &bytes[0..4] == b"IRDT" {
        let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0u8; 4]));
        let mut off = 8usize;
        for _ in 0..count {
            if off + 72 > bytes.len() {
                break;
            }
            let name_bytes = &bytes[off..off + 64];
            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
            let name = core::str::from_utf8(&name_bytes[..name_end]).unwrap_or("");
            
            // Check if the file is in the requested directory
            if name.starts_with(&normalized_path) {
                // Extract the filename or directory name
                let relative_path = &name[normalized_path.len()..];
                if let Some(first_slash) = relative_path.find('/') {
                    // It's a subdirectory
                    let dir_name = &relative_path[..first_slash];
                    // Add directory name if not already present
                    if !entries.contains(&String::from(dir_name)) {
                        entries.push(String::from(dir_name));
                    }
                } else if !relative_path.is_empty() {
                    // It's a file
                    entries.push(String::from(relative_path));
                }
            }
            
            off += 72;
        }
        return entries;
    }
    
    // For other formats, return empty list for now
    entries
}

pub fn ipc_handler(msg: &crate::kernel_core::ipc::Msg) -> crate::kernel_core::ipc::Resp {
    match msg.op {
        4 => {
            // Read operation
            let key = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(key).unwrap_or("");
            let mut path = s;
            let mut off: usize = 0;
            if let Some(sp) = s.find(' ') {
                path = &s[..sp];
                let offs = &s[(sp + 1)..];
                let mut v: usize = 0;
                for b in offs.as_bytes() {
                    if *b >= b'0' && *b <= b'9' {
                        v = v * 10 + ((*b - b'0') as usize);
                    } else {
                        break;
                    }
                }
                off = v;
            }
            let bytes = initrd_bytes();
            
            // Check permissions before reading
            let fs = crate::fs::vfs::InitrdFs;
            if let Some(metadata) = fs.stat(bytes, path) {
                if !fs.check_permission(&metadata, "read") {
                    return crate::kernel_core::ipc::Resp { code: -3, len: 0 }; // Permission denied
                }
            }
            
            if let Some((start, size)) = crate::fs::vfs::find_file(bytes, path) {
                let remaining = size.saturating_sub(off);
                let n = core::cmp::min(remaining, msg.l2);
                if n == 0 {
                    return crate::kernel_core::ipc::Resp { code: 0, len: 0 };
                }
                unsafe {
                    core::ptr::copy_nonoverlapping((start as *const u8).add(off), msg.p2, n);
                }
                crate::kernel_core::ipc::Resp { code: 0, len: n }
            } else {
                crate::kernel_core::ipc::Resp { code: -2, len: 0 }
            }
        }
        5 => {
            // Check if file exists
            let path = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(path).unwrap_or("");
            let bytes = initrd_bytes();
            
            // Check permissions before checking existence
            let fs = crate::fs::vfs::InitrdFs;
            if let Some(metadata) = fs.stat(bytes, s) {
                if !fs.check_permission(&metadata, "read") {
                    return crate::kernel_core::ipc::Resp { code: -3, len: 0 }; // Permission denied
                }
            }
            
            let ex = crate::fs::vfs::find_file(bytes, s).is_some();
            crate::kernel_core::ipc::Resp {
                code: if ex { 1 } else { 0 },
                len: 0,
            }
        }
        6 => {
            // List directory contents
            let path = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(path).unwrap_or("");
            let bytes = initrd_bytes();
            
            // Check permissions before listing directory
            let fs = crate::fs::vfs::InitrdFs;
            if let Some(metadata) = fs.stat(bytes, s) {
                if !fs.check_permission(&metadata, "read") {
                    return crate::kernel_core::ipc::Resp { code: -3, len: 0 }; // Permission denied
                }
            }
            
            let entries = list_directory(bytes, s);
            if entries.is_empty() {
                crate::kernel_core::ipc::Resp { code: -1, len: 0 }
            } else {
                // Copy directory entries to output buffer
                let output_buffer = unsafe { 
                    core::slice::from_raw_parts_mut(msg.p2, msg.l2) 
                };
                let mut offset = 0;
                for entry in entries.iter() {
                    if offset + entry.len() + 1 <= output_buffer.len() {
                        output_buffer[offset..offset + entry.len()].copy_from_slice(entry.as_bytes());
                        output_buffer[offset + entry.len()] = b'\n';
                        offset += entry.len() + 1;
                    } else {
                        break;
                    }
                }
                crate::kernel_core::ipc::Resp { code: 0, len: offset }
            }
        }
        7 => {
            // Get file size
            let path = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(path).unwrap_or("");
            let bytes = initrd_bytes();
            
            // Check permissions before getting file size
            let fs = crate::fs::vfs::InitrdFs;
            if let Some(metadata) = fs.stat(bytes, s) {
                if !fs.check_permission(&metadata, "read") {
                    return crate::kernel_core::ipc::Resp { code: -3, len: 0 }; // Permission denied
                }
            }
            
            if let Some((_start, size)) = crate::fs::vfs::find_file(bytes, s) {
                crate::kernel_core::ipc::Resp { code: 0, len: size }
            } else {
                crate::kernel_core::ipc::Resp { code: -1, len: 0 }
            }
        }
        8 => {
            // Write operation (not supported on initrd, return error)
            crate::debug!("vfs: write operation not supported on read-only initrd");
            crate::kernel_core::ipc::Resp { code: -1, len: 0 }  // Read-only filesystem
        }
        9 => {
            // Create operation (not supported on initrd)
            crate::debug!("vfs: create operation not supported on read-only initrd");
            crate::kernel_core::ipc::Resp { code: -1, len: 0 }  // Read-only filesystem
        }
        10 => {
            // Truncate operation (not supported on initrd)
            crate::debug!("vfs: truncate operation not supported on read-only initrd");
            crate::kernel_core::ipc::Resp { code: -1, len: 0 }  // Read-only filesystem
        }
        11 => {
            // Delete operation (not supported on initrd)
            crate::debug!("vfs: delete operation not supported on read-only initrd");
            crate::kernel_core::ipc::Resp { code: -1, len: 0 }  // Read-only filesystem
        }
        12 => {
            // Mkdir operation
            let path = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(path).unwrap_or("");
            let mode_ptr = msg.p2 as *const u32;
            let mode = unsafe { *mode_ptr };
            
            // For now, we'll create a temporary RamFs instance for mkdir operations
            // In a real implementation, this would use the mounted filesystem
            let mut ramfs = crate::fs::ramfs::RamFs::new();
            
            // Try to create directory in RamFs
            match ramfs.mkdir(s, mode) {
                Ok(()) => {
                    crate::debug!("vfs: mkdir successful for {}", s);
                    crate::kernel_core::ipc::Resp { code: 0, len: 0 }
                }
                Err(e) => {
                    crate::debug!("vfs: mkdir failed for {}: {:?}", s, e);
                    crate::kernel_core::ipc::Resp { code: -1, len: 0 }
                }
            }
        }
        13 => {
            // Rmdir operation
            let path = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(path).unwrap_or("");
            
            // For now, we'll create a temporary RamFs instance for rmdir operations
            // In a real implementation, this would use the mounted filesystem
            let mut ramfs = crate::fs::ramfs::RamFs::new();
            
            // Try to remove directory from RamFs
            match ramfs.rmdir(s) {
                Ok(()) => {
                    crate::debug!("vfs: rmdir successful for {}", s);
                    crate::kernel_core::ipc::Resp { code: 0, len: 0 }
                }
                Err(e) => {
                    crate::debug!("vfs: rmdir failed for {}: {:?}", s, e);
                    crate::kernel_core::ipc::Resp { code: -1, len: 0 }
                }
            }
        }
        14 => {
            // Readdir operation
            let path = unsafe { core::slice::from_raw_parts(msg.p1, msg.l1) };
            let s = core::str::from_utf8(path).unwrap_or("");
            
            // For now, we'll create a temporary RamFs instance for readdir operations
            // In a real implementation, this would use the mounted filesystem
            let ramfs = crate::fs::ramfs::RamFs::new();
            
            // Try to read directory entries from RamFs
            match ramfs.readdir(s) {
                Ok(entries) => {
                    // Serialize directory entries to output buffer
                    let output_buffer = unsafe { 
                        core::slice::from_raw_parts_mut(msg.p2, msg.l2) 
                    };
                    
                    let mut offset = 0;
                    for entry in entries.iter() {
                        // Simple serialization: name\0type\0size\0
                        let name_bytes = entry.name.as_bytes();
                        let type_byte = entry.entry_type as u8;
                        let size_bytes = (entry.size as u64).to_le_bytes();
                        
                        // Check if we have enough space
                        let needed = name_bytes.len() + 1 + 1 + 1 + 8;
                        if offset + needed > output_buffer.len() {
                            break;
                        }
                        
                        // Copy name
                        output_buffer[offset..offset + name_bytes.len()].copy_from_slice(name_bytes);
                        offset += name_bytes.len();
                        output_buffer[offset] = 0; // null terminator
                        offset += 1;
                        
                        // Copy type
                        output_buffer[offset] = type_byte;
                        offset += 1;
                        output_buffer[offset] = 0; // null terminator
                        offset += 1;
                        
                        // Copy size
                        output_buffer[offset..offset + 8].copy_from_slice(&size_bytes);
                        offset += 8;
                    }
                    
                    crate::debug!("vfs: readdir successful for {}, returned {} bytes", s, offset);
                    crate::kernel_core::ipc::Resp { code: 0, len: offset }
                }
                Err(e) => {
                    crate::debug!("vfs: readdir failed for {}: {:?}", s, e);
                    crate::kernel_core::ipc::Resp { code: -1, len: 0 }
                }
            }
        }
        _ => crate::kernel_core::ipc::Resp { code: -1, len: 0 },
    }
}
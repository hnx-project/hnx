//! User-space Filesystem Server Implementation
//!
//! This module implements a user-space filesystem server that communicates with the kernel
//! through IPC to handle filesystem operations.
//!
//! The server handles:
//! - File operations (open, read, write, close)
//! - Directory operations (mkdir, rmdir, readdir)
//! - Filesystem operations (create, delete, truncate)
//! - Path resolution and symbolic links
//! - Access control lists and extended attributes
//! - Snapshots and backups
//!
//! Communication with kernel is through standardized IPC protocol.

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use spin::Mutex;
use alloc::vec::Vec;

// Import our local modules
mod filesystems;
mod managers;

// Import system call interfaces
use hnx_libc::syscalls::{sys_channel_create, sys_channel_read, sys_channel_write};
use alloc::string::String;

// Define endpoint ID type for compatibility
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndpointId(pub u32);

/// Filesystem operation opcodes
pub const FS_OP_READ: u64 = 4;
pub const FS_OP_EXISTS: u64 = 5;
pub const FS_OP_LIST_DIR: u64 = 6;
pub const FS_OP_FILE_SIZE: u64 = 7;
pub const FS_OP_WRITE: u64 = 8;
pub const FS_OP_CREATE: u64 = 9;
pub const FS_OP_TRUNCATE: u64 = 10;
pub const FS_OP_DELETE: u64 = 11;
pub const FS_OP_MKDIR: u64 = 12;
pub const FS_OP_RMDIR: u64 = 13;
pub const FS_OP_READDIR: u64 = 14;

/// Filesystem error codes
pub const FS_ERR_NOT_FOUND: u32 = 1;
pub const FS_ERR_PERMISSION_DENIED: u32 = 2;
pub const FS_ERR_ALREADY_EXISTS: u32 = 3;
pub const FS_ERR_IS_DIRECTORY: u32 = 4;
pub const FS_ERR_NOT_DIRECTORY: u32 = 5;
pub const FS_ERR_READ_ONLY_FS: u32 = 6;
pub const FS_ERR_NO_SPACE: u32 = 7;
pub const FS_ERR_INVALID_OFFSET: u32 = 8;
pub const FS_ERR_INVALID_HANDLE: u32 = 9;
pub const FS_ERR_INVALID_PATH: u32 = 10;
pub const FS_ERR_IO_ERROR: u32 = 11;
pub const FS_ERR_NOT_EMPTY: u32 = 12;
pub const FS_ERR_NAME_TOO_LONG: u32 = 13;

/// File handle type
pub type FileHandle = usize;

/// Directory entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryType {
    File,
    Directory,
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub inode: u64,
    pub name: alloc::string::String,
    pub entry_type: DirEntryType,
    pub size: usize,
}

impl DirEntry {
    pub fn new(inode: u64, name: alloc::string::String, entry_type: DirEntryType, size: usize) -> Self {
        Self {
            inode,
            name,
            entry_type,
            size,
        }
    }
}

/// User-space filesystem server
pub struct FilesystemServer {
    /// Server endpoint ID for communication with kernel
    server_epid: EndpointId,
    
    /// RAM filesystem instance
    ramfs: filesystems::ramfs::RamFs,
}

impl FilesystemServer {
    /// Create a new filesystem server
    pub fn new() -> Result<Self, ()> {
        // Create endpoint for communication with kernel
        let server_epid = endpoint_create().map_err(|_| ())?;
        
        // Initialize RAM filesystem
        let ramfs = filesystems::ramfs::RamFs::new();
        
        Ok(Self {
            server_epid,
            ramfs,
        })
    }
    
    /// Register with kernel VFS service
    pub fn register_with_kernel(&self) -> Result<(), ()> {
        // In a real implementation, this would register with the kernel VFS service
        // For now, we'll just log that registration would happen
        // log_message(&format!("Filesystem server registered with kernel using EPID {}", self.server_epid.0));
        Ok(())
    }
    
    /// Initialize the filesystem server
    pub fn init(&mut self) {
        // log_message(&format!("Filesystem server initializing..."));
        
        // Initialize filesystems
        filesystems::init();
        managers::init();
        
        // log_message(&format!("Filesystem server initialized"));
    }
    
    /// Main server loop - process incoming IPC messages
    pub fn run(&mut self) -> ! {
        // log_message(&format!("Filesystem server starting with EPID {}", self.server_epid.0));
        
        loop {
            // Receive IPC message from kernel
            match endpoint_recv_sync(self.server_epid, None) {
                Ok(msg) => {
                    self.handle_ipc_message(msg);
                }
                Err(e) => {
                    // log_message(&format!("Error receiving IPC message: {:?}", e));
                    // Handle error or continue
                    continue;
                }
            }
        }
    }
    
    /// Handle incoming IPC message
    fn handle_ipc_message(&mut self, msg: IpcMessage) {
        match msg.op as u64 {
            FS_OP_READ => {
                self.handle_read_request(msg);
            }
            FS_OP_EXISTS => {
                self.handle_exists_request(msg);
            }
            FS_OP_LIST_DIR => {
                self.handle_list_dir_request(msg);
            }
            FS_OP_FILE_SIZE => {
                self.handle_file_size_request(msg);
            }
            FS_OP_WRITE => {
                self.handle_write_request(msg)); */ 
            }
            FS_OP_CREATE => {
                self.handle_create_request(msg)); */ 
            }
            FS_OP_TRUNCATE => {
                self.handle_truncate_request(msg)); */ 
            }
            FS_OP_DELETE => {
                self.handle_delete_request(msg)); */ 
            }
            FS_OP_MKDIR => {
                self.handle_mkdir_request(msg)); */ 
            }
            FS_OP_RMDIR => {
                self.handle_rmdir_request(msg)); */ 
            }
            FS_OP_READDIR => {
                self.handle_readdir_request(msg)); */ 
            }
            _ => {
                /* log_message(crate::println!(format!(*/ "Filesystem server received unknown opcode: {}", msg.op)); */ 
                
                // Send error response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = FS_ERR_IO_ERROR as u8;
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send error response: {:?}", e)); */ 
                }
            }
        }
    }
    
    /// Handle read request
    fn handle_read_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling read request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes..., offset: u64, buffer_size: u32]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len + 8 + 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        // Extract offset (8 bytes after path)
        let offset_bytes = &msg.data[4 + path_len..4 + path_len + 8];
        let offset = u64::from_le_bytes([
            offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3],
            offset_bytes[4], offset_bytes[5], offset_bytes[6], offset_bytes[7]
        ]) as usize;
        
        // Extract buffer size (4 bytes after offset)
        let buf_size_bytes = &msg.data[4 + path_len + 8..4 + path_len + 8 + 4];
        let buf_size = u32::from_le_bytes([
            buf_size_bytes[0], buf_size_bytes[1], buf_size_bytes[2], buf_size_bytes[3]
        ]) as usize;
        
        /* log_message(crate::println!(format!(*/ "Reading from path: {}, offset: {}, buffer size: {}", path, offset, buf_size)); */ 
        
        // Open file
        match self.ramfs.open(path, 0) {
            Ok(handle) => {
                // Create buffer for reading
                let mut buffer = vec![0u8; buf_size];
                
                // Read data
                match self.ramfs.read(handle, &mut buffer, offset) {
                    Ok(bytes_read) => {
                        // Close file handle
                        let _ = self.ramfs.close(handle)); */ 
                        
                        // Prepare response with data
                        let mut response_data = [0u8; 256];
                        
                        // First 4 bytes: number of bytes read
                        response_data[0..4].copy_from_slice(&(bytes_read as u32).to_le_bytes())); */ 
                        
                        // Next bytes: actual data read
                        let data_end = 4 + bytes_read;
                        if data_end <= 256 {
                            response_data[4..data_end].copy_from_slice(&buffer[..bytes_read])); */ 
                            
                            // Send success response
                            let response_msg = IpcMessage {
                                src_pid: 0,
                                dst_epid: msg.src_pid,
                                op: msg.op,
                                priority: Priority::Normal,
                                data_len: data_end,
                                data: response_data,
                                timestamp: crate::arch::timer::now_us(),
                            };
                            
                            if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                                /* log_message(crate::println!(format!(*/ "Failed to send read response: {:?}", e)); */ 
                            }
                        } else {
                            // Data too large for response buffer
                            self.send_error_response(msg.src_pid, msg.op, FS_ERR_IO_ERROR)); */ 
                        }
                    }
                    Err(_) => {
                        // Close file handle
                        let _ = self.ramfs.close(handle)); */ 
                        
                        self.send_error_response(msg.src_pid, msg.op, FS_ERR_IO_ERROR)); */ 
                    }
                }
            }
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_NOT_FOUND)); */ 
            }
        }
    }
    
    /// Handle file exists request
    fn handle_exists_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling file exists request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        /* log_message(crate::println!(format!(*/ "Checking if file exists: {}", path)); */ 
        
        // Check if file exists
        match self.ramfs.lookup(path) {
            Ok(_) => {
                // File exists - send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 0; // Success (file exists)
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send exists response: {:?}", e)); */ 
                }
            }
            Err(_) => {
                // File does not exist - send not found response
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_NOT_FOUND)); */ 
            }
        }
    }
    
    /// Handle list directory request
    fn handle_list_dir_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling list directory request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        /* log_message(crate::println!(format!(*/ "Listing directory: {}", path)); */ 
        
        // List directory contents
        match self.ramfs.readdir(path) {
            Ok(entries) => {
                // Serialize directory entries
                let mut response_data = [0u8; 256];
                let mut offset = 0;
                
                // First 4 bytes: number of entries
                let num_entries = entries.len() as u32;
                response_data[offset..offset + 4].copy_from_slice(&num_entries.to_le_bytes())); */ 
                offset += 4;
                
                // For each entry, serialize: name_len (u32), name_bytes, entry_type (u8), size (u64)
                for entry in entries {
                    if offset + 4 + entry.name.len() + 1 + 8 > 256 {
                        // Not enough space in response buffer
                        self.send_error_response(msg.src_pid, msg.op, FS_ERR_IO_ERROR)); */ 
                        return;
                    }
                    
                    // Name length
                    let name_len = entry.name.len() as u32;
                    response_data[offset..offset + 4].copy_from_slice(&name_len.to_le_bytes())); */ 
                    offset += 4;
                    
                    // Name bytes
                    response_data[offset..offset + entry.name.len()].copy_from_slice(entry.name.as_bytes())); */ 
                    offset += entry.name.len()); */ 
                    
                    // Entry type
                    let entry_type_byte = match entry.entry_type {
                        DirEntryType::File => 0u8,
                        DirEntryType::Directory => 1u8,
                    };
                    response_data[offset] = entry_type_byte;
                    offset += 1;
                    
                    // Size
                    response_data[offset..offset + 8].copy_from_slice(&(entry.size as u64).to_le_bytes())); */ 
                    offset += 8;
                }
                
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: offset,
                    data: response_data,
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send list directory response: {:?}", e)); */ 
                }
            }
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_NOT_FOUND)); */ 
            }
        }
    }
    
    /// Handle file size request
    fn handle_file_size_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling file size request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        /* log_message(crate::println!(format!(*/ "Getting file size for: {}", path)); */ 
        
        // Get file size
        match self.ramfs.lookup(path) {
            Ok(node) => {
                let n = node.lock()); */ 
                let size = n.data.len() as u64;
                
                // Send success response with file size
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 8,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0..8].copy_from_slice(&size.to_le_bytes())); */ 
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send file size response: {:?}", e)); */ 
                }
            }
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_NOT_FOUND)); */ 
            }
        }
    }
    
    /// Handle write request
    fn handle_write_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling write request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes..., offset: u64, data_len: u32, data_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len + 8 + 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        // Extract offset (8 bytes after path)
        let offset_bytes = &msg.data[4 + path_len..4 + path_len + 8];
        let offset = u64::from_le_bytes([
            offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3],
            offset_bytes[4], offset_bytes[5], offset_bytes[6], offset_bytes[7]
        ]) as usize;
        
        // Extract data length (4 bytes after offset)
        let data_len_bytes = &msg.data[4 + path_len + 8..4 + path_len + 8 + 4];
        let data_len = u32::from_le_bytes([
            data_len_bytes[0], data_len_bytes[1], data_len_bytes[2], data_len_bytes[3]
        ]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len + 8 + 4 + data_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract data
        let data_bytes = &msg.data[4 + path_len + 8 + 4..4 + path_len + 8 + 4 + data_len];
        
        /* log_message(crate::println!(format!(*/ "Writing to path: {}, offset: {}, data length: {}", path, offset, data_len)); */ 
        
        // Open file (create if it doesn't exist)
        let handle = match self.ramfs.open(path, 0) {
            Ok(h) => h,
            Err(_) => {
                // Try to create the file
                match self.ramfs.create(path, 0o644) {
                    Ok(h) => h,
                    Err(_) => {
                        self.send_error_response(msg.src_pid, msg.op, FS_ERR_IO_ERROR)); */ 
                        return;
                    }
                }
            }
        };
        
        // Write data
        match self.ramfs.write(handle, data_bytes, offset) {
            Ok(bytes_written) => {
                // Close file handle
                let _ = self.ramfs.close(handle)); */ 
                
                // Send success response with bytes written
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0..4].copy_from_slice(&(bytes_written as u32).to_le_bytes())); */ 
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send write response: {:?}", e)); */ 
                }
            }
            Err(_) => {
                // Close file handle
                let _ = self.ramfs.close(handle)); */ 
                
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_IO_ERROR)); */ 
            }
        }
    }
    
    /// Handle create request
    fn handle_create_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling create request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes..., mode: u32]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len + 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        // Extract mode (4 bytes after path)
        let mode_bytes = &msg.data[4 + path_len..4 + path_len + 4];
        let mode = u32::from_le_bytes([
            mode_bytes[0], mode_bytes[1], mode_bytes[2], mode_bytes[3]
        ])); */ 
        
        /* log_message(crate::println!(format!(*/ "Creating file: {} with mode: {:o}", path, mode)); */ 
        
        // Create file
        match self.ramfs.create(path, mode) {
            Ok(_) => {
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 0; // Success
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send create response: {:?}", e)); */ 
                }
            }
            Err(fs_err) => {
                let err_code = match fs_err {
                    filesystems::ramfs::FsError::AlreadyExists => FS_ERR_ALREADY_EXISTS,
                    filesystems::ramfs::FsError::PermissionDenied => FS_ERR_PERMISSION_DENIED,
                    filesystems::ramfs::FsError::InvalidPath => FS_ERR_INVALID_PATH,
                    _ => FS_ERR_IO_ERROR,
                };
                self.send_error_response(msg.src_pid, msg.op, err_code)); */ 
            }
        }
    }
    
    /// Handle truncate request
    fn handle_truncate_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling truncate request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes..., size: u64]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len + 8 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        // Extract size (8 bytes after path)
        let size_bytes = &msg.data[4 + path_len..4 + path_len + 8];
        let size = u64::from_le_bytes([
            size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3],
            size_bytes[4], size_bytes[5], size_bytes[6], size_bytes[7]
        ]) as usize;
        
        /* log_message(crate::println!(format!(*/ "Truncating file: {} to size: {}", path, size)); */ 
        
        // Truncate file
        match self.ramfs.truncate(path, size) {
            Ok(_) => {
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 0; // Success
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send truncate response: {:?}", e)); */ 
                }
            }
            Err(fs_err) => {
                let err_code = match fs_err {
                    filesystems::ramfs::FsError::NotFound => FS_ERR_NOT_FOUND,
                    filesystems::ramfs::FsError::PermissionDenied => FS_ERR_PERMISSION_DENIED,
                    filesystems::ramfs::FsError::InvalidPath => FS_ERR_INVALID_PATH,
                    _ => FS_ERR_IO_ERROR,
                };
                self.send_error_response(msg.src_pid, msg.op, err_code)); */ 
            }
        }
    }
    
    /// Handle delete request
    fn handle_delete_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling delete request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        /* log_message(crate::println!(format!(*/ "Deleting file: {}", path)); */ 
        
        // Delete file
        match self.ramfs.delete(path) {
            Ok(_) => {
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 0; // Success
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send delete response: {:?}", e)); */ 
                }
            }
            Err(fs_err) => {
                let err_code = match fs_err {
                    filesystems::ramfs::FsError::NotFound => FS_ERR_NOT_FOUND,
                    filesystems::ramfs::FsError::PermissionDenied => FS_ERR_PERMISSION_DENIED,
                    filesystems::ramfs::FsError::InvalidPath => FS_ERR_INVALID_PATH,
                    _ => FS_ERR_IO_ERROR,
                };
                self.send_error_response(msg.src_pid, msg.op, err_code)); */ 
            }
        }
    }
    
    /// Handle mkdir request
    fn handle_mkdir_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling mkdir request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes..., mode: u32]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len + 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        // Extract mode (4 bytes after path)
        let mode_bytes = &msg.data[4 + path_len..4 + path_len + 4];
        let mode = u32::from_le_bytes([
            mode_bytes[0], mode_bytes[1], mode_bytes[2], mode_bytes[3]
        ])); */ 
        
        /* log_message(crate::println!(format!(*/ "Creating directory: {} with mode: {:o}", path, mode)); */ 
        
        // Create directory
        match self.ramfs.mkdir(path, mode) {
            Ok(_) => {
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 0; // Success
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send mkdir response: {:?}", e)); */ 
                }
            }
            Err(fs_err) => {
                let err_code = match fs_err {
                    filesystems::ramfs::FsError::AlreadyExists => FS_ERR_ALREADY_EXISTS,
                    filesystems::ramfs::FsError::PermissionDenied => FS_ERR_PERMISSION_DENIED,
                    filesystems::ramfs::FsError::InvalidPath => FS_ERR_INVALID_PATH,
                    _ => FS_ERR_IO_ERROR,
                };
                self.send_error_response(msg.src_pid, msg.op, err_code)); */ 
            }
        }
    }
    
    /// Handle rmdir request
    fn handle_rmdir_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling rmdir request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        /* log_message(crate::println!(format!(*/ "Removing directory: {}", path)); */ 
        
        // Remove directory
        match self.ramfs.rmdir(path) {
            Ok(_) => {
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 0; // Success
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send rmdir response: {:?}", e)); */ 
                }
            }
            Err(fs_err) => {
                let err_code = match fs_err {
                    filesystems::ramfs::FsError::NotFound => FS_ERR_NOT_FOUND,
                    filesystems::ramfs::FsError::PermissionDenied => FS_ERR_PERMISSION_DENIED,
                    filesystems::ramfs::FsError::InvalidPath => FS_ERR_INVALID_PATH,
                    filesystems::ramfs::FsError::NotEmpty => FS_ERR_NOT_EMPTY,
                    _ => FS_ERR_IO_ERROR,
                };
                self.send_error_response(msg.src_pid, msg.op, err_code)); */ 
            }
        }
    }
    
    /// Handle readdir request
    fn handle_readdir_request(&mut self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*/ "Filesystem server handling readdir request")); */ 
        
        // Parse request data
        // Expected format: [path_len: u32, path_bytes...]
        if msg.data_len < 4 {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path length
        let path_len = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]) as usize;
        
        // Check if we have enough data
        if msg.data_len < 4 + path_len {
            self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
            return;
        }
        
        // Extract path
        let path_bytes = &msg.data[4..4 + path_len];
        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                self.send_error_response(msg.src_pid, msg.op, FS_ERR_INVALID_PATH)); */ 
                return;
            }
        };
        
        /* log_message(crate::println!(format!(*/ "Reading directory: {}", path)); */ 
        
        // Read directory entries
        match self.ramfs.readdir(path) {
            Ok(entries) => {
                // Serialize directory entries
                let mut response_data = [0u8; 256];
                let mut offset = 0;
                
                // First 4 bytes: number of entries
                let num_entries = entries.len() as u32;
                response_data[offset..offset + 4].copy_from_slice(&num_entries.to_le_bytes())); */ 
                offset += 4;
                
                // For each entry, serialize: name_len (u32), name_bytes, entry_type (u8), size (u64)
                for entry in entries {
                    if offset + 4 + entry.name.len() + 1 + 8 > 256 {
                        // Not enough space in response buffer
                        self.send_error_response(msg.src_pid, msg.op, FS_ERR_IO_ERROR)); */ 
                        return;
                    }
                    
                    // Name length
                    let name_len = entry.name.len() as u32;
                    response_data[offset..offset + 4].copy_from_slice(&name_len.to_le_bytes())); */ 
                    offset += 4;
                    
                    // Name bytes
                    response_data[offset..offset + entry.name.len()].copy_from_slice(entry.name.as_bytes())); */ 
                    offset += entry.name.len()); */ 
                    
                    // Entry type
                    let entry_type_byte = match entry.entry_type {
                        DirEntryType::File => 0u8,
                        DirEntryType::Directory => 1u8,
                    };
                    response_data[offset] = entry_type_byte;
                    offset += 1;
                    
                    // Size
                    response_data[offset..offset + 8].copy_from_slice(&(entry.size as u64).to_le_bytes())); */ 
                    offset += 8;
                }
                
                // Send success response
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: offset,
                    data: response_data,
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!(*/ "Failed to send readdir response: {:?}", e)); */ 
                }
            }
            Err(fs_err) => {
                let err_code = match fs_err {
                    filesystems::ramfs::FsError::NotFound => FS_ERR_NOT_FOUND,
                    filesystems::ramfs::FsError::PermissionDenied => FS_ERR_PERMISSION_DENIED,
                    filesystems::ramfs::FsError::InvalidPath => FS_ERR_INVALID_PATH,
                    filesystems::ramfs::FsError::NotDirectory => FS_ERR_NOT_DIRECTORY,
                    _ => FS_ERR_IO_ERROR,
                };
                self.send_error_response(msg.src_pid, msg.op, err_code)); */ 
            }
        }
    }
    
    /// Send error response
    fn send_error_response(&self, dst_epid: u32, op: u16, error_code: u32) {
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid,
            op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0..4].copy_from_slice(&error_code.to_le_bytes())); */ 
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(dst_epid, response_msg, None) {
            /* log_message(crate::println!(format!(*/ "Failed to send error response: {:?}", e)); */ 
        }
    }
}

/// Initialize and start the filesystem server
pub fn start_filesystem_server() -> ! {
    /* log_message(crate::println!(format!(*/ "Starting user-space filesystem server...")); */ 
    
    // Create filesystem server instance
    let mut server = FilesystemServer::new().expect("Failed to create filesystem server")); */ 
    
    // Register with kernel
    server.register_with_kernel().expect("Failed to register with kernel")); */ 
    
    // Initialize the server
    server.init()); */ 
    
    // Start server loop
    server.run()); */ 
}
extern crate alloc;

use alloc::vec::Vec;
use crate::fs::dirent::DirEntry;
use crate::fs::acl::AclPermissions;

/// File permissions
#[derive(Debug, Clone, Copy)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub owner: u32,  // User ID
    pub group: u32,  // Group ID
}

impl Permissions {
    /// Convert to ACL permissions
    pub fn to_acl_permissions(&self) -> AclPermissions {
        AclPermissions::new(self.read, self.write, self.execute)
    }
    
    /// Create Permissions from ACL permissions
    pub fn from_acl_permissions(acl_perm: &AclPermissions, owner: u32, group: u32) -> Self {
        Self {
            read: acl_perm.read,
            write: acl_perm.write,
            execute: acl_perm.execute,
            owner,
            group,
        }
    }
}

/// File metadata
#[derive(Debug, Clone)]
pub struct Metadata {
    pub permissions: Permissions,
    pub size: usize,
    pub created: u64,  // Timestamp
    pub modified: u64, // Timestamp
}

/// File handle type
pub type FileHandle = usize;

/// Filesystem error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    IsDirectory,
    NotDirectory,
    ReadOnlyFs,
    NoSpace,
    InvalidOffset,
    InvalidHandle,
    InvalidPath,
    IoError,
    NotEmpty,
    NameTooLong,
}

/// Filesystem trait with read and write operations
pub trait FileSystem {
    // Read-only operations (existing)
    fn find(&self, bytes: &[u8], path: &str) -> Option<(usize, usize)>;
    
    /// Get file metadata
    fn stat(&self, bytes: &[u8], path: &str) -> Option<Metadata>;
    
    /// Check if the current process has permission to access the file
    fn check_permission(&self, metadata: &Metadata, required_perm: &str) -> bool;
    
    /// Check if a path refers to a symbolic link
    fn is_symlink(&self, bytes: &[u8], path: &str) -> bool;
}

/// Writable filesystem trait - extends FileSystem with write operations
pub trait WritableFileSystem {
    /// Write data to file at given offset
    /// Returns number of bytes written
    fn write(&mut self, handle: FileHandle, buf: &[u8], offset: usize) -> Result<usize, FsError>;
    
    /// Create a new file
    /// Returns file handle on success
    fn create(&mut self, path: &str, mode: u32) -> Result<FileHandle, FsError>;
    
    /// Delete a file
    fn delete(&mut self, path: &str) -> Result<(), FsError>;
    
    /// Truncate file to specified size
    fn truncate(&mut self, path: &str, size: usize) -> Result<(), FsError>;
    
    /// Create directory
    fn mkdir(&mut self, path: &str, mode: u32) -> Result<(), FsError>;
    
    /// Remove empty directory
    fn rmdir(&mut self, path: &str) -> Result<(), FsError>;
    
    /// Read directory entries
    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, FsError>;
    
    /// Open file for reading/writing
    fn open(&mut self, path: &str, flags: u32) -> Result<FileHandle, FsError>;
    
    /// Read from file at given offset
    fn read(&self, handle: FileHandle, buf: &mut [u8], offset: usize) -> Result<usize, FsError>;
    
    /// Close file handle
    fn close(&mut self, handle: FileHandle) -> Result<(), FsError>;
}

/// Helper function to check write permissions
pub fn has_write_permission(permissions: &Permissions, uid: u32, gid: u32) -> bool {
    // Owner write permission
    if uid == permissions.owner && permissions.write {
        return true;
    }
    // Group write permission (simplified - always allow for now)
    if gid == permissions.group && permissions.write {
        return true;
    }
    // Other write permission
    if permissions.write {
        return true;
    }
    false
}

pub struct InitrdFs;

impl FileSystem for InitrdFs {
    fn find(&self, bytes: &[u8], path: &str) -> Option<(usize, usize)> {
        find_file(bytes, path)
    }
    
    fn stat(&self, bytes: &[u8], path: &str) -> Option<Metadata> {
        // For initrd, we'll return default permissions
        // In a real implementation, this would come from the file system metadata
        if self.find(bytes, path).is_some() {
            Some(Metadata {
                permissions: Permissions {
                    read: true,
                    write: false, // Initrd is typically read-only
                    execute: true,
                    owner: 0,     // Root user
                    group: 0,     // Root group
                },
                size: self.find(bytes, path).map(|(_, size)| size).unwrap_or(0),
                created: 0,   // Not available in initrd
                modified: 0,  // Not available in initrd
            })
        } else {
            None
        }
    }
    
    fn check_permission(&self, metadata: &Metadata, required_perm: &str) -> bool {
        match required_perm {
            "read" => metadata.permissions.read,
            "write" => metadata.permissions.write,
            "execute" => metadata.permissions.execute,
            _ => false,
        }
    }
    
    fn is_symlink(&self, _bytes: &[u8], _path: &str) -> bool {
        // Initrd doesn't support symbolic links
        false
    }
}

pub fn find_file(bytes: &[u8], path: &str) -> Option<(usize, usize)> {
    // IRDT format
    if bytes.len() >= 8 && &bytes[0..4] == b"IRDT" {
        let count = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
        let mut off = 8usize;
        for _ in 0..count {
            if off + 72 > bytes.len() {
                break;
            }
            let name_bytes = &bytes[off..off + 64];
            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
            let name = core::str::from_utf8(&name_bytes[..name_end]).ok()?;
            let offv = u32::from_le_bytes(bytes[off + 64..off + 68].try_into().ok()?);
            let sizev = u32::from_le_bytes(bytes[off + 68..off + 72].try_into().ok()?);
            if name == path {
                let s = offv as usize;
                let e = s.saturating_add(sizev as usize);
                if e <= bytes.len() {
                    return Some((bytes.as_ptr() as usize + s, sizev as usize));
                }
            }
            off += 72;
        }
        return None;
    }
    // CPIO newc format
    if bytes.len() >= 6 && &bytes[0..6] == b"070701" {
        return crate::fs::cpio::find_file(bytes, path);
    }
    // Raw ELF fallback (used when loader puts a single file at initrd_start)
    if bytes.len() >= 4 && bytes[0..4] == [0x7F, b'E', b'L', b'F']
        && path == "/bin/hello.elf" {
            return Some((bytes.as_ptr() as usize, bytes.len()));
        }
    None
}
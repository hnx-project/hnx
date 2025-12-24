/// RamFs - In-memory filesystem implementation
/// 
/// This module implements a simple in-memory filesystem that supports
/// full read/write operations. It's primarily used for testing VFS write
/// operations and as a temporary filesystem (tmpfs).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::RwLock;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::fs::vfs::{WritableFileSystem, FileHandle, FsError, Permissions, Metadata};
use crate::fs::dirent::{DirEntry, DirEntryType};
use crate::fs::symlink::{SymlinkOps, SymlinkFileSystem, Symlink};
use crate::fs::acl::{Acl, AclOps, DefaultAclOps, AclType, AclPermissions};
use crate::fs::xattr::{XattrOps, XattrFlags, XattrCollection};
use crate::fs::snapshot::{SnapshotOps, Snapshot, SnapshotManager};
use alloc::collections::BTreeSet;

/// Node type in the filesystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
}

/// A node in the ramfs tree
pub struct RamNode {
    inode: u64,
    name: String,
    node_type: NodeType,
    permissions: Permissions,
    created: u64,
    modified: u64,
    children: BTreeMap<String, Arc<RwLock<RamNode>>>,  // For directories
    data: Vec<u8>,  // For files
    // Optional ACL for this node
    acl: Option<Acl>,
    // Default ACL for directories
    default_acl: Option<Acl>,
    // Extended attributes
    xattrs: XattrCollection,
}

impl RamNode {
    /// Create a new file node
    fn new_file(inode: u64, name: String, mode: u32) -> Self {
        let now = crate::arch::timer::now_us();
        Self {
            inode,
            name,
            node_type: NodeType::File,
            permissions: Permissions {
                read: (mode & 0o400) != 0,
                write: (mode & 0o200) != 0,
                execute: (mode & 0o100) != 0,
                owner: 0,  // Root user for now
                group: 0,  // Root group for now
            },
            created: now,
            modified: now,
            children: BTreeMap::new(),
            data: Vec::new(),
            acl: None,
            default_acl: None,
            xattrs: XattrCollection::new(),
        }
    }
    
    /// Create a new directory node
    fn new_directory(inode: u64, name: String, mode: u32) -> Self {
        let now = crate::arch::timer::now_us();
        Self {
            inode,
            name,
            node_type: NodeType::Directory,
            permissions: Permissions {
                read: (mode & 0o400) != 0,
                write: (mode & 0o200) != 0,
                execute: (mode & 0o100) != 0,
                owner: 0,
                group: 0,
            },
            created: now,
            modified: now,
            children: BTreeMap::new(),
            data: Vec::new(),
            acl: None,
            default_acl: None,
            xattrs: XattrCollection::new(),
        }
    }
}

/// RamFs filesystem
pub struct RamFs {
    root: Arc<RwLock<RamNode>>,
    next_inode: AtomicU64,
    open_handles: Arc<RwLock<BTreeMap<FileHandle, Arc<RwLock<RamNode>>>>>,
    next_handle: AtomicU64,
    snapshot_manager: SnapshotManager,
}

impl RamFs {
    /// Create a new RamFs instance
    pub fn new() -> Self {
        let root = RamNode::new_directory(1, String::from("/"), 0o755);
        Self {
            root: Arc::new(RwLock::new(root)),
            next_inode: AtomicU64::new(2),
            open_handles: Arc::new(RwLock::new(BTreeMap::new())),
            next_handle: AtomicU64::new(1),
            snapshot_manager: SnapshotManager::new(),
        }
    }
    
    /// Allocate next inode number
    fn next_inode(&self) -> u64 {
        self.next_inode.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Allocate next file handle
    fn next_handle(&self) -> FileHandle {
        self.next_handle.fetch_add(1, Ordering::SeqCst) as FileHandle
    }
    
    /// Lookup a node by path
    /// Returns (parent_node, child_name) for the last component
    fn lookup(&self, path: &str) -> Result<Arc<RwLock<RamNode>>, FsError> {
        if path.is_empty() || path == "/" {
            return Ok(Arc::clone(&self.root));
        }
        
        // Split path into components
        let components: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        
        if components.is_empty() {
            return Ok(Arc::clone(&self.root));
        }
        
        let mut current = Arc::clone(&self.root);
        
        for component in components {
            let child_opt = {
                let node = current.read();
                
                if node.node_type != NodeType::Directory {
                    return Err(FsError::NotDirectory);
                }
                
                node.children.get(component).map(Arc::clone)
            };
            
            if let Some(child) = child_opt {
                current = child;
            } else {
                return Err(FsError::NotFound);
            }
        }
        
        Ok(current)
    }
    
    /// Lookup parent directory and child name
    fn lookup_parent(&self, path: &str) -> Result<(Arc<RwLock<RamNode>>, String), FsError> {
        if path.is_empty() || path == "/" {
            return Err(FsError::InvalidPath);
        }
        
        let trimmed = path.trim_start_matches('/');
        let components: Vec<&str> = trimmed
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        
        if components.is_empty() {
            return Err(FsError::InvalidPath);
        }
        
        let child_name = components[components.len() - 1].to_string();
        
        if components.len() == 1 {
            return Ok((Arc::clone(&self.root), child_name));
        }
        
        let parent_path = components[..components.len() - 1].join("/");
        let parent = self.lookup(&parent_path)?;
        
        Ok((parent, child_name))
    }
    
    /// Check permissions for a file or directory using ACLs if available
    pub fn check_acl_permission(&self, path: &str, uid: u32, gid: u32, required: &str) -> bool {
        // Try to get the node
        let node = match self.lookup(path) {
            Ok(node) => node,
            Err(_) => return false,
        };
        
        let n = node.read();
        
        // Convert required permission string to ACL permissions
        let required_perm = match required {
            "read" => AclPermissions::new(true, false, false),
            "write" => AclPermissions::new(false, true, false),
            "execute" => AclPermissions::new(false, false, true),
            _ => return false,
        };
        
        // If there's an ACL, use it for permission checking
        if let Some(acl) = &n.acl {
            acl.check_permission(uid, gid, &required_perm)
        } else {
            // Fall back to traditional permissions
            match required {
                "read" => n.permissions.read,
                "write" => n.permissions.write,
                "execute" => n.permissions.execute,
                _ => false,
            }
        }
    }
}

impl AclOps for RamFs {
    fn set_acl(&mut self, path: &str, acl: &Acl) -> Result<(), FsError> {
        // Validate the ACL
        acl.validate().map_err(|_| FsError::InvalidPath)?;
        
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        // Set the ACL
        n.acl = Some(acl.clone());
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    fn get_acl(&self, path: &str) -> Result<Acl, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        // Return the ACL if it exists, otherwise create one from traditional permissions
        if let Some(acl) = &n.acl {
            Ok(acl.clone())
        } else {
            let acl = Acl::from_permissions(&n.permissions, AclType::Access);
            Ok(acl)
        }
    }
    
    fn remove_acl(&mut self, path: &str) -> Result<(), FsError> {
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        // Remove the ACL
        n.acl = None;
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    fn has_acl(&self, path: &str) -> bool {
        match self.lookup(path) {
            Ok(node) => {
                let n = node.read();
                n.acl.is_some()
            }
            Err(_) => false
        }
    }
}

impl DefaultAclOps for RamFs {
    fn set_default_acl(&mut self, path: &str, acl: &Acl) -> Result<(), FsError> {
        // Validate the ACL
        acl.validate().map_err(|_| FsError::InvalidPath)?;
        
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        // Check that this is a directory
        if n.node_type != NodeType::Directory {
            return Err(FsError::NotDirectory);
        }
        
        // Set the default ACL
        n.default_acl = Some(acl.clone());
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    fn get_default_acl(&self, path: &str) -> Result<Acl, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        // Check that this is a directory
        if n.node_type != NodeType::Directory {
            return Err(FsError::NotDirectory);
        }
        
        // Return the default ACL if it exists
        if let Some(acl) = &n.default_acl {
            Ok(acl.clone())
        } else {
            Err(FsError::NotFound)
        }
    }
    
    fn remove_default_acl(&mut self, path: &str) -> Result<(), FsError> {
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        // Check that this is a directory
        if n.node_type != NodeType::Directory {
            return Err(FsError::NotDirectory);
        }
        
        // Remove the default ACL
        n.default_acl = None;
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
}

impl SymlinkOps for RamFs {
    fn create_symlink(&mut self, path: &str, target: &str) -> Result<(), FsError> {
        // Validate target path length
        if target.len() > crate::fs::symlink::MAX_SYMLINK_TARGET_LEN {
            return Err(FsError::NameTooLong);
        }
        
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Check if symlink already exists
        {
            let p = parent.read();
            if p.children.contains_key(&child_name) {
                return Err(FsError::AlreadyExists);
            }
        }
        
        // Create new symlink node
        let inode = self.next_inode();
        let symlink = Symlink::new(inode, child_name.clone(), target.to_string(), 0o777);
        
        // For now, we'll store symlinks as special files in RamFs
        // In a more complete implementation, we would have a separate node type
        let mut node = RamNode::new_file(inode, child_name.clone(), 0o777);
        node.data = target.as_bytes().to_vec();
        
        let node_arc = alloc::sync::Arc::new(spin::RwLock::new(node));
        
        // Add to parent
        {
            let mut p = parent.write();
            p.children.insert(child_name, node_arc);
            p.modified = crate::arch::timer::now_us();
        }
        
        Ok(())
    }
    
    fn read_symlink(&self, path: &str) -> Result<alloc::string::String, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        // Check if it's actually a symlink
        // For now, we'll treat any file as a potential symlink
        // In a more complete implementation, we would have a specific symlink flag
        if n.node_type != NodeType::File {
            return Err(FsError::InvalidPath);
        }
        
        // Return the target path stored in the file data
        let target = alloc::string::String::from_utf8(n.data.clone()).map_err(|_| FsError::IoError)?;
        Ok(target)
    }
    
    fn resolve_symlink(&self, path: &str) -> Result<alloc::string::String, FsError> {
        let target = self.read_symlink(path)?;
        
        // If target is absolute, return as-is
        if target.starts_with('/') {
            Ok(target)
        } else {
            // If target is relative, join with parent directory of symlink
            let parent = crate::fs::path::get_parent_path(path);
            let resolved = crate::fs::path::join_paths(&parent, &target);
            crate::fs::path::canonicalize_path(&resolved)
        }
    }
    
    fn is_symlink(&self, path: &str) -> bool {
        // For now, we'll assume any file could be a symlink
        // In a more complete implementation, we would check a specific symlink flag
        match self.lookup(path) {
            Ok(node) => {
                let n = node.read();
                n.node_type == NodeType::File
            }
            Err(_) => false
        }
    }
}

impl SymlinkFileSystem for RamFs {
    fn follow_symlink(&self, path: &str, max_depth: usize) -> Result<alloc::string::String, FsError> {
        let mut current_path = path.to_string();
        let mut depth = 0;
        
        loop {
            // Check recursion depth
            if depth >= max_depth {
                return Err(FsError::IoError); // Too many levels of symbolic links
            }
            
            // If current path is not a symlink, we're done
            if !self.is_symlink(&current_path) {
                return Ok(current_path);
            }
            
            // Resolve the symlink
            let target = self.resolve_symlink(&current_path)?;
            current_path = target;
            depth += 1;
        }
    }
    
    fn detect_symlink_cycle(&self, path: &str) -> Result<bool, FsError> {
        let mut visited_paths = BTreeSet::new();
        let mut current_path = path.to_string();
        let max_depth = 8; // Limit recursion depth for cycle detection
        let mut depth = 0;
        
        loop {
            // Check recursion depth
            if depth >= max_depth {
                return Ok(true); // Assume cycle if we reach max depth
            }
            
            // If we've seen this path before, we have a cycle
            if visited_paths.contains(&current_path) {
                return Ok(true);
            }
            
            // Add current path to visited set
            visited_paths.insert(current_path.clone());
            
            // If current path is not a symlink, we're done
            if !self.is_symlink(&current_path) {
                return Ok(false);
            }
            
            // Resolve the symlink
            let target = self.resolve_symlink(&current_path)?;
            current_path = target;
            depth += 1;
        }
    }
}

impl XattrOps for RamFs {
    fn set_xattr(&mut self, path: &str, name: &str, value: &[u8], flags: XattrFlags) -> Result<(), FsError> {
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        // Set the extended attribute
        n.xattrs.set(name.to_string(), value.to_vec(), flags)?;
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    fn get_xattr(&self, path: &str, name: &str) -> Result<Vec<u8>, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        // Get the extended attribute
        if let Some(attr) = n.xattrs.get(name) {
            Ok(attr.value.clone())
        } else {
            Err(FsError::NotFound)
        }
    }
    
    fn list_xattrs(&self, path: &str) -> Result<Vec<String>, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        // List all extended attributes
        Ok(n.xattrs.list_names())
    }
    
    fn remove_xattr(&mut self, path: &str, name: &str) -> Result<(), FsError> {
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        // Remove the extended attribute
        n.xattrs.remove(name)?;
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    fn list_xattrs_in_namespace(&self, path: &str, namespace: &str) -> Result<Vec<String>, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        // List extended attributes in the specified namespace
        Ok(n.xattrs.list_names_in_namespace(namespace))
    }
}

impl SnapshotOps for RamFs {
    fn create_snapshot(&mut self, name: &str, path: &str) -> Result<String, FsError> {
        // For RamFs, we'll create a snapshot of the entire filesystem
        // In a more sophisticated implementation, we might snapshot just a subtree
        
        let root_inode = 1; // Root inode is always 1 in RamFs
        let snapshot = self.snapshot_manager.create_snapshot(
            name.to_string(),
            None, // No parent snapshot for now
            root_inode,
        )?;
        
        Ok(snapshot.id)
    }
    
    fn delete_snapshot(&mut self, snapshot_id: &str) -> Result<(), FsError> {
        self.snapshot_manager.delete_snapshot(snapshot_id)
    }
    
    fn list_snapshots(&self, _path: &str) -> Result<Vec<Snapshot>, FsError> {
        // For RamFs, we list all snapshots since it's a simple in-memory filesystem
        Ok(self.snapshot_manager.list_snapshots())
    }
    
    fn restore_snapshot(&mut self, snapshot_id: &str, _target_path: &str) -> Result<(), FsError> {
        // Restoration in RamFs would involve replacing the current filesystem state
        // For now, we'll just delegate to the snapshot manager
        self.snapshot_manager.restore_snapshot(snapshot_id)
    }
    
    fn clone_snapshot(&mut self, snapshot_id: &str, new_name: &str) -> Result<String, FsError> {
        let snapshot = self.snapshot_manager.clone_snapshot(snapshot_id, new_name.to_string())?;
        Ok(snapshot.id)
    }
}

impl WritableFileSystem for RamFs {
    fn open(&mut self, path: &str, _flags: u32) -> Result<FileHandle, FsError> {
        let node = self.lookup(path)?;
        
        // Check if it's a file
        {
            let n = node.read();
            if n.node_type != NodeType::File {
                return Err(FsError::IsDirectory);
            }
        }
        
        // Allocate handle
        let handle = self.next_handle();
        self.open_handles.write().insert(handle, node);
        
        Ok(handle)
    }
    
    fn read(&self, handle: FileHandle, buf: &mut [u8], offset: usize) -> Result<usize, FsError> {
        let handles = self.open_handles.read();
        let node = handles.get(&handle).ok_or(FsError::InvalidHandle)?;
        let n = node.read();
        
        if offset >= n.data.len() {
            return Ok(0);
        }
        
        let remaining = n.data.len() - offset;
        let to_read = core::cmp::min(buf.len(), remaining);
        
        buf[..to_read].copy_from_slice(&n.data[offset..offset + to_read]);
        
        Ok(to_read)
    }
    
    fn write(&mut self, handle: FileHandle, buf: &[u8], offset: usize) -> Result<usize, FsError> {
        let handles = self.open_handles.read();
        let node = handles.get(&handle).ok_or(FsError::InvalidHandle)?;
        let mut n = node.write();
        
        // Check permissions
        if !n.permissions.write {
            return Err(FsError::PermissionDenied);
        }
        
        // Expand file if necessary
        let required_size = offset + buf.len();
        if n.data.len() < required_size {
            n.data.resize(required_size, 0);
        }
        
        // Write data
        n.data[offset..offset + buf.len()].copy_from_slice(buf);
        
        // Update metadata
        n.modified = crate::arch::timer::now_us();
        
        Ok(buf.len())
    }
    
    fn create(&mut self, path: &str, mode: u32) -> Result<FileHandle, FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Check if file already exists
        {
            let p = parent.read();
            if p.children.contains_key(&child_name) {
                return Err(FsError::AlreadyExists);
            }
        }
        
        // Create new file node
        let inode = self.next_inode();
        let node = RamNode::new_file(inode, child_name.clone(), mode);
        let node_arc = Arc::new(RwLock::new(node));
        
        // Add to parent
        {
            let mut p = parent.write();
            p.children.insert(child_name, Arc::clone(&node_arc));
            p.modified = crate::arch::timer::now_us();
        }
        
        // Allocate handle
        let handle = self.next_handle();
        self.open_handles.write().insert(handle, node_arc);
        
        Ok(handle)
    }
    
    fn delete(&mut self, path: &str) -> Result<(), FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Remove from parent
        {
            let mut p = parent.write();
            let child = p.children.get(&child_name).ok_or(FsError::NotFound)?;
            
            // Check if it's a file
            let c = child.read();
            if c.node_type != NodeType::File {
                return Err(FsError::IsDirectory);
            }
            drop(c);
            
            p.children.remove(&child_name);
            p.modified = crate::arch::timer::now_us();
        }
        
        Ok(())
    }
    
    fn truncate(&mut self, path: &str, size: usize) -> Result<(), FsError> {
        let node = self.lookup(path)?;
        let mut n = node.write();
        
        if n.node_type != NodeType::File {
            return Err(FsError::IsDirectory);
        }
        
        n.data.resize(size, 0);
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    fn mkdir(&mut self, path: &str, mode: u32) -> Result<(), FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Check if directory already exists
        {
            let p = parent.read();
            if p.children.contains_key(&child_name) {
                return Err(FsError::AlreadyExists);
            }
        }
        
        // Create new directory node
        let inode = self.next_inode();
        let node = RamNode::new_directory(inode, child_name.clone(), mode);
        let node_arc = Arc::new(RwLock::new(node));
        
        // Add to parent
        {
            let mut p = parent.write();
            p.children.insert(child_name, node_arc);
            p.modified = crate::arch::timer::now_us();
        }
        
        Ok(())
    }
    
    fn rmdir(&mut self, path: &str) -> Result<(), FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Remove from parent
        {
            let mut p = parent.write();
            let child = p.children.get(&child_name).ok_or(FsError::NotFound)?;
            
            // Check if it's a directory
            let c = child.read();
            if c.node_type != NodeType::Directory {
                return Err(FsError::NotDirectory);
            }
            
            // Check if directory is empty
            if !c.children.is_empty() {
                return Err(FsError::NotEmpty);  // Directory not empty
            }
            drop(c);
            
            p.children.remove(&child_name);
            p.modified = crate::arch::timer::now_us();
        }
        
        Ok(())
    }
    
    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let node = self.lookup(path)?;
        let n = node.read();
        
        if n.node_type != NodeType::Directory {
            return Err(FsError::NotDirectory);
        }
        
        let mut entries = Vec::new();
        
        // Add "." entry (current directory)
        entries.push(DirEntry::new(
            n.inode,
            ".".to_string(),
            DirEntryType::Directory,
            0,
        ));
        
        // Add ".." entry (parent directory)
        // For root directory, parent is itself
        let parent_inode = if path == "/" || path.is_empty() {
            n.inode
        } else {
            // For simplicity, we'll use the root inode as parent for now
            // In a more complete implementation, we'd track parent inodes
            1
        };
        
        entries.push(DirEntry::new(
            parent_inode,
            "..".to_string(),
            DirEntryType::Directory,
            0,
        ));
        
        // Add child entries
        for (name, child_node) in &n.children {
            let child = child_node.read();
            let entry_type = match child.node_type {
                NodeType::File => DirEntryType::File,
                NodeType::Directory => DirEntryType::Directory,
            };
            
            entries.push(DirEntry::new(
                child.inode,
                name.clone(),
                entry_type,
                child.data.len(),
            ));
        }
        
        Ok(entries)
    }
    
    fn close(&mut self, handle: FileHandle) -> Result<(), FsError> {
        self.open_handles.write().remove(&handle);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn test_create_and_write() {
        let mut fs = RamFs::new();
        
        // Create a file
        let handle = fs.create("/test.txt", 0o644).unwrap();
        
        // Write data
        let data = b"Hello, RamFs!";
        let written = fs.write(handle, data, 0).unwrap();
        assert_eq!(written, data.len());
        
        // Read data back
        let mut buf = [0u8; 32];
        let read = fs.read(handle, &mut buf, 0).unwrap();
        assert_eq!(read, data.len());
        assert_eq!(&buf[..read], data);
        
        fs.close(handle).unwrap();
    }
    
    #[test]
    fn test_mkdir_and_create() {
        let mut fs = RamFs::new();
        
        // Create directory
        fs.mkdir("/tmp", 0o755).unwrap();
        
        // Create file in directory
        let handle = fs.create("/tmp/test.txt", 0o644).unwrap();
        
        // Write data
        let data = b"Test data";
        fs.write(handle, data, 0).unwrap();
        
        fs.close(handle).unwrap();
        
        // Try to open again
        let handle2 = fs.open("/tmp/test.txt", 0).unwrap();
        let mut buf = [0u8; 16];
        let read = fs.read(handle2, &mut buf, 0).unwrap();
        assert_eq!(&buf[..read], data);
        
        fs.close(handle2).unwrap();
    }
    
    #[test]
    fn test_delete() {
        let mut fs = RamFs::new();
        
        // Create and write
        let handle = fs.create("/temp.txt", 0o644).unwrap();
        fs.write(handle, b"temp", 0).unwrap();
        fs.close(handle).unwrap();
        
        // Delete
        fs.delete("/temp.txt").unwrap();
        
        // Try to open - should fail
        assert!(fs.open("/temp.txt", 0).is_err());
    }
}
//! RAM filesystem implementation for user-space filesystem server

use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

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

/// File handle type
pub type FileHandle = usize;

/// Node type in the filesystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
}

/// File permissions
#[derive(Debug, Clone, Copy)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub owner: u32,  // User ID
    pub group: u32,  // Group ID
}

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
    pub name: String,
    pub entry_type: DirEntryType,
    pub size: usize,
}

impl DirEntry {
    pub fn new(inode: u64, name: String, entry_type: DirEntryType, size: usize) -> Self {
        Self {
            inode,
            name,
            entry_type,
            size,
        }
    }
}

/// A node in the ramfs tree
pub struct RamNode {
    inode: u64,
    name: String,
    node_type: NodeType,
    permissions: Permissions,
    created: u64,
    modified: u64,
    children: BTreeMap<String, Arc<Mutex<RamNode>>>,  // For directories
    data: Vec<u8>,  // For files
}

impl RamNode {
    /// Create a new file node
    pub fn new_file(inode: u64, name: String, mode: u32) -> Self {
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
        }
    }
    
    /// Create a new directory node
    pub fn new_directory(inode: u64, name: String, mode: u32) -> Self {
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
        }
    }
}

/// RamFs filesystem
pub struct RamFs {
    root: Arc<Mutex<RamNode>>,
    next_inode: AtomicU64,
    open_handles: Arc<Mutex<BTreeMap<FileHandle, Arc<Mutex<RamNode>>>>>,
    next_handle: AtomicU64,
}

impl RamFs {
    /// Create a new RamFs instance
    pub fn new() -> Self {
        let root = RamNode::new_directory(1, String::from("/"), 0o755);
        Self {
            root: Arc::new(Mutex::new(root)),
            next_inode: AtomicU64::new(2),
            open_handles: Arc::new(Mutex::new(BTreeMap::new())),
            next_handle: AtomicU64::new(1),
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
    fn lookup(&self, path: &str) -> Result<Arc<Mutex<RamNode>>, FsError> {
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
                let node = current.lock();
                
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
    fn lookup_parent(&self, path: &str) -> Result<(Arc<Mutex<RamNode>>, String), FsError> {
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
}

impl RamFs {
    pub fn open(&mut self, path: &str, _flags: u32) -> Result<FileHandle, FsError> {
        let node = self.lookup(path)?;
        
        // Check if it's a file
        {
            let n = node.lock();
            if n.node_type != NodeType::File {
                return Err(FsError::IsDirectory);
            }
        }
        
        // Allocate handle
        let handle = self.next_handle();
        self.open_handles.lock().insert(handle, node);
        
        Ok(handle)
    }
    
    pub fn read(&self, handle: FileHandle, buf: &mut [u8], offset: usize) -> Result<usize, FsError> {
        let handles = self.open_handles.lock();
        let node = handles.get(&handle).ok_or(FsError::InvalidHandle)?;
        let n = node.lock();
        
        if offset >= n.data.len() {
            return Ok(0);
        }
        
        let remaining = n.data.len() - offset;
        let to_read = core::cmp::min(buf.len(), remaining);
        
        buf[..to_read].copy_from_slice(&n.data[offset..offset + to_read]);
        
        Ok(to_read)
    }
    
    pub fn write(&mut self, handle: FileHandle, buf: &[u8], offset: usize) -> Result<usize, FsError> {
        let handles = self.open_handles.lock();
        let node = handles.get(&handle).ok_or(FsError::InvalidHandle)?;
        let mut n = node.lock();
        
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
    
    pub fn create(&mut self, path: &str, mode: u32) -> Result<FileHandle, FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Check if file already exists
        {
            let p = parent.lock();
            if p.children.contains_key(&child_name) {
                return Err(FsError::AlreadyExists);
            }
        }
        
        // Create new file node
        let inode = self.next_inode();
        let node = RamNode::new_file(inode, child_name.clone(), mode);
        let node_arc = Arc::new(Mutex::new(node));
        
        // Add to parent
        {
            let mut p = parent.lock();
            p.children.insert(child_name, Arc::clone(&node_arc));
            p.modified = crate::arch::timer::now_us();
        }
        
        // Allocate handle
        let handle = self.next_handle();
        self.open_handles.lock().insert(handle, node_arc);
        
        Ok(handle)
    }
    
    pub fn delete(&mut self, path: &str) -> Result<(), FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Remove from parent
        {
            let mut p = parent.lock();
            let child = p.children.get(&child_name).ok_or(FsError::NotFound)?;
            
            // Check if it's a file
            let c = child.lock();
            if c.node_type != NodeType::File {
                return Err(FsError::IsDirectory);
            }
            drop(c);
            
            p.children.remove(&child_name);
            p.modified = crate::arch::timer::now_us();
        }
        
        Ok(())
    }
    
    pub fn truncate(&mut self, path: &str, size: usize) -> Result<(), FsError> {
        let node = self.lookup(path)?;
        let mut n = node.lock();
        
        if n.node_type != NodeType::File {
            return Err(FsError::IsDirectory);
        }
        
        n.data.resize(size, 0);
        n.modified = crate::arch::timer::now_us();
        
        Ok(())
    }
    
    pub fn mkdir(&mut self, path: &str, mode: u32) -> Result<(), FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Check if directory already exists
        {
            let p = parent.lock();
            if p.children.contains_key(&child_name) {
                return Err(FsError::AlreadyExists);
            }
        }
        
        // Create new directory node
        let inode = self.next_inode();
        let node = RamNode::new_directory(inode, child_name.clone(), mode);
        let node_arc = Arc::new(Mutex::new(node));
        
        // Add to parent
        {
            let mut p = parent.lock();
            p.children.insert(child_name, node_arc);
            p.modified = crate::arch::timer::now_us();
        }
        
        Ok(())
    }
    
    pub fn rmdir(&mut self, path: &str) -> Result<(), FsError> {
        let (parent, child_name) = self.lookup_parent(path)?;
        
        // Remove from parent
        {
            let mut p = parent.lock();
            let child = p.children.get(&child_name).ok_or(FsError::NotFound)?;
            
            // Check if it's a directory
            let c = child.lock();
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
    
    pub fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let node = self.lookup(path)?;
        let n = node.lock();
        
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
            let child = child_node.lock();
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
    
    pub fn close(&mut self, handle: FileHandle) -> Result<(), FsError> {
        self.open_handles.lock().remove(&handle);
        Ok(())
    }
}
/// Directory entry management for HNX filesystem
/// 
/// This module provides structures and functions for managing directory entries
/// and directory streams for reading directory contents.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

/// Directory entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryType {
    Unknown = 0,
    File = 1,
    Directory = 2,
    Symlink = 3,
}

/// Directory entry structure
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub inode: u64,
    pub name: String,
    pub entry_type: DirEntryType,
    pub size: usize,
}

impl DirEntry {
    /// Create a new directory entry
    pub fn new(inode: u64, name: String, entry_type: DirEntryType, size: usize) -> Self {
        Self {
            inode,
            name,
            entry_type,
            size,
        }
    }
}

/// Directory stream for reading directory contents
pub struct DirStream {
    entries: Vec<DirEntry>,
    position: usize,
}

impl DirStream {
    /// Create a new directory stream
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            position: 0,
        }
    }
    
    /// Add an entry to the directory stream
    pub fn add_entry(&mut self, entry: DirEntry) {
        self.entries.push(entry);
    }
    
    /// Read the next entry from the directory stream
    pub fn read_next(&mut self) -> Option<&DirEntry> {
        if self.position < self.entries.len() {
            let entry = &self.entries[self.position];
            self.position += 1;
            Some(entry)
        } else {
            None
        }
    }
    
    /// Rewind the directory stream to the beginning
    pub fn rewind(&mut self) {
        self.position = 0;
    }
    
    /// Get the number of entries in the directory stream
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if the directory stream is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
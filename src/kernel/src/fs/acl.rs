//! Access Control List (ACL) implementation for HNX filesystem
//!
//! This module implements POSIX-compliant ACLs that provide fine-grained
//! access control beyond traditional file permissions.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::fmt;

use crate::fs::vfs::{Permissions, FsError};

/// ACL tag types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclTag {
    /// File owner
    UserObj,
    /// Specific user (qualified by UID)
    User(u32),
    /// File group
    GroupObj,
    /// Specific group (qualified by GID)
    Group(u32),
    /// Mask for group permissions
    Mask,
    /// Other users
    Other,
}

/// ACL permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AclPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl AclPermissions {
    /// Create new ACL permissions
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self { read, write, execute }
    }
    
    /// Convert traditional file permissions to ACL permissions
    pub fn from_permissions(perm: &Permissions) -> Self {
        Self {
            read: perm.read,
            write: perm.write,
            execute: perm.execute,
        }
    }
    
    /// Check if this permission set includes another
    pub fn includes(&self, other: &Self) -> bool {
        (!other.read || self.read) && 
        (!other.write || self.write) && 
        (!other.execute || self.execute)
    }
}

impl fmt::Display for AclPermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = if self.read { 'r' } else { '-' };
        let w = if self.write { 'w' } else { '-' };
        let x = if self.execute { 'x' } else { '-' };
        write!(f, "{}{}{}", r, w, x)
    }
}

/// ACL entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclEntry {
    /// Tag type
    pub tag: AclTag,
    /// Permissions
    pub permissions: AclPermissions,
}

impl AclEntry {
    /// Create a new ACL entry
    pub fn new(tag: AclTag, permissions: AclPermissions) -> Self {
        Self { tag, permissions }
    }
    
    /// Check if this entry matches a specific user/group
    pub fn matches(&self, uid: u32, gid: u32) -> bool {
        match self.tag {
            AclTag::UserObj => true, // Always applies to owner
            AclTag::User(id) => id == uid,
            AclTag::GroupObj => true, // Always applies to group
            AclTag::Group(id) => id == gid,
            AclTag::Mask => false, // Mask doesn't match users/groups directly
            AclTag::Other => true, // Always applies to others
        }
    }
}

impl fmt::Display for AclEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag_str = match self.tag {
            AclTag::UserObj => "user::".to_string(),
            AclTag::User(uid) => alloc::format!("user:{}:", uid),
            AclTag::GroupObj => "group::".to_string(),
            AclTag::Group(gid) => alloc::format!("group:{}:", gid),
            AclTag::Mask => "mask::".to_string(),
            AclTag::Other => "other::".to_string(),
        };
        write!(f, "{}{}", tag_str, self.permissions)
    }
}

/// ACL type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclType {
    /// Access ACL (for file/directory access)
    Access,
    /// Default ACL (for new files in directory)
    Default,
}

/// ACL structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acl {
    /// ACL entries
    pub entries: Vec<AclEntry>,
    /// ACL type
    pub acl_type: AclType,
}

impl Acl {
    /// Create a new ACL
    pub fn new(acl_type: AclType) -> Self {
        Self {
            entries: Vec::new(),
            acl_type,
        }
    }
    
    /// Create an ACL from traditional permissions
    pub fn from_permissions(perm: &Permissions, acl_type: AclType) -> Self {
        let mut acl = Self::new(acl_type);
        
        // Add basic entries matching traditional permissions
        acl.entries.push(AclEntry::new(
            AclTag::UserObj,
            AclPermissions::new(perm.read, perm.write, perm.execute),
        ));
        
        acl.entries.push(AclEntry::new(
            AclTag::GroupObj,
            AclPermissions::new(perm.read, perm.write, perm.execute),
        ));
        
        acl.entries.push(AclEntry::new(
            AclTag::Other,
            AclPermissions::new(perm.read, perm.write, perm.execute),
        ));
        
        acl
    }
    
    /// Add or update an entry
    pub fn set_entry(&mut self, entry: AclEntry) {
        // Check if entry already exists
        for existing in &mut self.entries {
            if existing.tag == entry.tag {
                // Update existing entry
                *existing = entry;
                return;
            }
        }
        
        // Add new entry
        self.entries.push(entry);
    }
    
    /// Get an entry by tag
    pub fn get_entry(&self, tag: &AclTag) -> Option<&AclEntry> {
        self.entries.iter().find(|entry| &entry.tag == tag)
    }
    
    /// Remove an entry by tag
    pub fn remove_entry(&mut self, tag: &AclTag) -> Option<AclEntry> {
        if let Some(pos) = self.entries.iter().position(|entry| &entry.tag == tag) {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }
    
    /// Check permissions for a user/group
    pub fn check_permission(&self, uid: u32, gid: u32, required: &AclPermissions) -> bool {
        // First check for specific user match
        for entry in &self.entries {
            if entry.matches(uid, gid) {
                match entry.tag {
                    AclTag::User(id) if id == uid => {
                        return entry.permissions.includes(required);
                    }
                    AclTag::Group(id) if id == gid => {
                        // Check if there's a mask entry that limits group permissions
                        if let Some(mask_entry) = self.get_entry(&AclTag::Mask) {
                            let masked_perms = AclPermissions::new(
                                entry.permissions.read && mask_entry.permissions.read,
                                entry.permissions.write && mask_entry.permissions.write,
                                entry.permissions.execute && mask_entry.permissions.execute,
                            );
                            if masked_perms.includes(required) {
                                return true;
                            }
                        } else {
                            // No mask, use group permissions directly
                            if entry.permissions.includes(required) {
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Check owner permissions
        if let Some(owner_entry) = self.get_entry(&AclTag::UserObj) {
            if uid == 0 || uid == 0 { // Simplified owner check
                return owner_entry.permissions.includes(required);
            }
        }
        
        // Check group permissions
        if let Some(group_entry) = self.get_entry(&AclTag::GroupObj) {
            // Check if there's a mask entry that limits group permissions
            if let Some(mask_entry) = self.get_entry(&AclTag::Mask) {
                let masked_perms = AclPermissions::new(
                    group_entry.permissions.read && mask_entry.permissions.read,
                    group_entry.permissions.write && mask_entry.permissions.write,
                    group_entry.permissions.execute && mask_entry.permissions.execute,
                );
                if masked_perms.includes(required) {
                    return true;
                }
            } else {
                // No mask, use group permissions directly
                if group_entry.permissions.includes(required) {
                    return true;
                }
            }
        }
        
        // Check other permissions
        if let Some(other_entry) = self.get_entry(&AclTag::Other) {
            return other_entry.permissions.includes(required);
        }
        
        false
    }
    
    /// Validate ACL consistency
    pub fn validate(&self) -> Result<(), FsError> {
        // Must have user_obj, group_obj, and other entries
        let has_user_obj = self.entries.iter().any(|e| matches!(e.tag, AclTag::UserObj));
        let has_group_obj = self.entries.iter().any(|e| matches!(e.tag, AclTag::GroupObj));
        let has_other = self.entries.iter().any(|e| matches!(e.tag, AclTag::Other));
        
        if !has_user_obj || !has_group_obj || !has_other {
            return Err(FsError::InvalidPath);
        }
        
        // If we have named user or group entries, we must have a mask
        let has_named_entries = self.entries.iter().any(|e| {
            matches!(e.tag, AclTag::User(_) | AclTag::Group(_))
        });
        
        let has_mask = self.entries.iter().any(|e| matches!(e.tag, AclTag::Mask));
        
        if has_named_entries && !has_mask {
            return Err(FsError::InvalidPath);
        }
        
        Ok(())
    }
}

impl fmt::Display for Acl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "# file: <path>")?;
        writeln!(f, "# owner: <owner>")?;
        writeln!(f, "# group: <group>")?;
        
        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }
        
        Ok(())
    }
}

/// ACL operations trait
pub trait AclOps {
    /// Set ACL for a file
    fn set_acl(&mut self, path: &str, acl: &Acl) -> Result<(), FsError>;
    
    /// Get ACL for a file
    fn get_acl(&self, path: &str) -> Result<Acl, FsError>;
    
    /// Remove ACL for a file
    fn remove_acl(&mut self, path: &str) -> Result<(), FsError>;
    
    /// Check if a file has an ACL
    fn has_acl(&self, path: &str) -> bool;
}

/// Extended trait for filesystems that support default ACLs
pub trait DefaultAclOps {
    /// Set default ACL for a directory
    fn set_default_acl(&mut self, path: &str, acl: &Acl) -> Result<(), FsError>;
    
    /// Get default ACL for a directory
    fn get_default_acl(&self, path: &str) -> Result<Acl, FsError>;
    
    /// Remove default ACL for a directory
    fn remove_default_acl(&mut self, path: &str) -> Result<(), FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_acl_permissions() {
        let perm1 = AclPermissions::new(true, true, true);
        let perm2 = AclPermissions::new(true, false, true);
        
        assert!(perm1.includes(&perm2));
        assert!(!perm2.includes(&perm1));
    }
    
    #[test]
    fn test_acl_entry() {
        let entry = AclEntry::new(
            AclTag::User(1000),
            AclPermissions::new(true, false, true),
        );
        
        assert!(entry.matches(1000, 1000));
        assert!(!entry.matches(1001, 1000));
    }
    
    #[test]
    fn test_basic_acl() {
        let mut acl = Acl::new(AclType::Access);
        
        // Add basic entries
        acl.set_entry(AclEntry::new(
            AclTag::UserObj,
            AclPermissions::new(true, true, true),
        ));
        
        acl.set_entry(AclEntry::new(
            AclTag::GroupObj,
            AclPermissions::new(true, false, true),
        ));
        
        acl.set_entry(AclEntry::new(
            AclTag::Other,
            AclPermissions::new(false, false, false),
        ));
        
        // Validation should pass
        assert!(acl.validate().is_ok());
        
        // Check permissions
        let read_perm = AclPermissions::new(true, false, false);
        let write_perm = AclPermissions::new(false, true, false);
        
        assert!(acl.check_permission(0, 0, &read_perm)); // Owner can read
        assert!(acl.check_permission(0, 0, &write_perm)); // Owner can write
        assert!(acl.check_permission(1000, 1000, &read_perm)); // Group can read
        assert!(!acl.check_permission(1000, 1000, &write_perm)); // Group cannot write
        assert!(!acl.check_permission(2000, 2000, &read_perm)); // Others cannot read
    }
    
    #[test]
    fn test_acl_with_named_entries() {
        let mut acl = Acl::new(AclType::Access);
        
        // Add basic entries
        acl.set_entry(AclEntry::new(
            AclTag::UserObj,
            AclPermissions::new(true, true, true),
        ));
        
        acl.set_entry(AclEntry::new(
            AclTag::GroupObj,
            AclPermissions::new(true, false, true),
        ));
        
        // Add named user entry
        acl.set_entry(AclEntry::new(
            AclTag::User(1000),
            AclPermissions::new(true, true, false),
        ));
        
        // Add mask
        acl.set_entry(AclEntry::new(
            AclTag::Mask,
            AclPermissions::new(true, true, true),
        ));
        
        acl.set_entry(AclEntry::new(
            AclTag::Other,
            AclPermissions::new(false, false, false),
        ));
        
        // Validation should pass
        assert!(acl.validate().is_ok());
        
        // Check permissions for named user
        let read_perm = AclPermissions::new(true, false, false);
        let write_perm = AclPermissions::new(false, true, false);
        
        assert!(acl.check_permission(1000, 1000, &read_perm)); // Named user can read
        assert!(acl.check_permission(1000, 1000, &write_perm)); // Named user can write
    }
    
    #[test]
    fn test_invalid_acl() {
        let mut acl = Acl::new(AclType::Access);
        
        // Missing required entries
        acl.set_entry(AclEntry::new(
            AclTag::UserObj,
            AclPermissions::new(true, true, true),
        ));
        
        // Validation should fail
        assert!(acl.validate().is_err());
        
        // Add named user without mask
        acl.set_entry(AclEntry::new(
            AclTag::User(1000),
            AclPermissions::new(true, true, true),
        ));
        
        acl.set_entry(AclEntry::new(
            AclTag::GroupObj,
            AclPermissions::new(true, false, true),
        ));
        
        acl.set_entry(AclEntry::new(
            AclTag::Other,
            AclPermissions::new(false, false, false),
        ));
        
        // Validation should fail due to missing mask
        assert!(acl.validate().is_err());
    }
}
//! Extended attributes (xattr) implementation for HNX filesystem
//!
//! This module implements extended attributes that allow storing additional
//! metadata with files and directories. Extended attributes are organized
//! into namespaces to prevent conflicts between different uses.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::fmt;

use crate::fs::vfs::FsError;

/// Extended attribute flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrFlags {
    /// Create attribute only if it doesn't exist
    Create,
    /// Replace attribute only if it already exists
    Replace,
    /// Default behavior (create or replace)
    Default,
}

/// Extended attribute namespace
pub mod xattr_namespace {
    /// User namespace - for user-defined attributes
    pub const USER: &str = "user.";
    /// System namespace - for system-level attributes
    pub const SYSTEM: &str = "system.";
    /// Security namespace - for security-related attributes
    pub const SECURITY: &str = "security.";
    /// Trusted namespace - for trusted attributes (root only)
    pub const TRUSTED: &str = "trusted.";
    
    /// Get the namespace prefix from a full attribute name
    pub fn get_namespace(name: &str) -> Option<&str> {
        if name.starts_with(USER) {
            Some(USER)
        } else if name.starts_with(SYSTEM) {
            Some(SYSTEM)
        } else if name.starts_with(SECURITY) {
            Some(SECURITY)
        } else if name.starts_with(TRUSTED) {
            Some(TRUSTED)
        } else {
            None
        }
    }
    
    /// Check if namespace is valid
    pub fn is_valid_namespace(name: &str) -> bool {
        get_namespace(name).is_some()
    }
    
    /// Check if attribute name is valid
    pub fn is_valid_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 255 {
            return false;
        }
        
        // Must have a namespace prefix
        if !is_valid_namespace(name) {
            return false;
        }
        
        // Must have content after the namespace prefix
        let prefix_len = get_namespace(name).unwrap().len();
        if name.len() <= prefix_len {
            return false;
        }
        
        // Rest of the name must not contain null bytes or '/'
        let rest = &name[prefix_len..];
        !rest.contains('\0') && !rest.contains('/')
    }
}

/// Extended attribute
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xattr {
    /// Attribute name (including namespace)
    pub name: String,
    /// Attribute value
    pub value: Vec<u8>,
}

impl Xattr {
    /// Create a new extended attribute
    pub fn new(name: String, value: Vec<u8>) -> Result<Self, FsError> {
        // Validate attribute name
        if !xattr_namespace::is_valid_name(&name) {
            return Err(FsError::InvalidPath);
        }
        
        Ok(Self { name, value })
    }
    
    /// Get the namespace of this attribute
    pub fn namespace(&self) -> Option<&str> {
        xattr_namespace::get_namespace(&self.name)
    }
    
    /// Get the name without namespace
    pub fn name_without_namespace(&self) -> &str {
        if let Some(namespace) = self.namespace() {
            &self.name[namespace.len()..]
        } else {
            &self.name
        }
    }
}

impl fmt::Display for Xattr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.name, alloc::format!("{:?}", self.value))
    }
}

/// Extended attributes collection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XattrCollection {
    /// Attributes stored by name
    attributes: BTreeMap<String, Xattr>,
}

impl XattrCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            attributes: BTreeMap::new(),
        }
    }
    
    /// Set an extended attribute
    pub fn set(&mut self, name: String, value: Vec<u8>, flags: XattrFlags) -> Result<(), FsError> {
        // Validate attribute name
        if !xattr_namespace::is_valid_name(&name) {
            return Err(FsError::InvalidPath);
        }
        
        // Check flags
        match flags {
            XattrFlags::Create => {
                // Fail if attribute already exists
                if self.attributes.contains_key(&name) {
                    return Err(FsError::AlreadyExists);
                }
            }
            XattrFlags::Replace => {
                // Fail if attribute doesn't exist
                if !self.attributes.contains_key(&name) {
                    return Err(FsError::NotFound);
                }
            }
            XattrFlags::Default => {
                // Allow create or replace
            }
        }
        
        // Create and insert the attribute
        let attr = Xattr::new(name.clone(), value)?;
        self.attributes.insert(name, attr);
        
        Ok(())
    }
    
    /// Get an extended attribute
    pub fn get(&self, name: &str) -> Option<&Xattr> {
        self.attributes.get(name)
    }
    
    /// Remove an extended attribute
    pub fn remove(&mut self, name: &str) -> Result<(), FsError> {
        if self.attributes.remove(name).is_some() {
            Ok(())
        } else {
            Err(FsError::NotFound)
        }
    }
    
    /// List all attribute names
    pub fn list_names(&self) -> Vec<String> {
        self.attributes.keys().cloned().collect()
    }
    
    /// List all attributes in a specific namespace
    pub fn list_names_in_namespace(&self, namespace: &str) -> Vec<String> {
        self.attributes
            .keys()
            .filter(|name| name.starts_with(namespace))
            .cloned()
            .collect()
    }
    
    /// Get all attributes
    pub fn get_all(&self) -> Vec<&Xattr> {
        self.attributes.values().collect()
    }
    
    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
    
    /// Get number of attributes
    pub fn len(&self) -> usize {
        self.attributes.len()
    }
    
    /// Clear all attributes
    pub fn clear(&mut self) {
        self.attributes.clear();
    }
}

/// Extended attributes operations trait
pub trait XattrOps {
    /// Set an extended attribute
    fn set_xattr(&mut self, path: &str, name: &str, value: &[u8], flags: XattrFlags) -> Result<(), FsError>;
    
    /// Get an extended attribute
    fn get_xattr(&self, path: &str, name: &str) -> Result<Vec<u8>, FsError>;
    
    /// List extended attributes
    fn list_xattrs(&self, path: &str) -> Result<Vec<String>, FsError>;
    
    /// Remove an extended attribute
    fn remove_xattr(&mut self, path: &str, name: &str) -> Result<(), FsError>;
    
    /// List extended attributes in a specific namespace
    fn list_xattrs_in_namespace(&self, path: &str, namespace: &str) -> Result<Vec<String>, FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_namespace_validation() {
        assert!(xattr_namespace::is_valid_namespace("user.test"));
        assert!(xattr_namespace::is_valid_namespace("system.test"));
        assert!(xattr_namespace::is_valid_namespace("security.test"));
        assert!(xattr_namespace::is_valid_namespace("trusted.test"));
        assert!(!xattr_namespace::is_valid_namespace("invalid.test"));
    }
    
    #[test]
    fn test_name_validation() {
        assert!(xattr_namespace::is_valid_name("user.test"));
        assert!(xattr_namespace::is_valid_name("security.selinux"));
        assert!(!xattr_namespace::is_valid_name("test")); // No namespace
        assert!(!xattr_namespace::is_valid_name("user.")); // Empty name
        assert!(!xattr_namespace::is_valid_name("")); // Empty name
        assert!(!xattr_namespace::is_valid_name("user.test\0")); // Contains null
        assert!(!xattr_namespace::is_valid_name("user.test/name")); // Contains slash
    }
    
    #[test]
    fn test_xattr_creation() {
        let attr = Xattr::new("user.test".to_string(), alloc::vec![1, 2, 3]);
        assert!(attr.is_ok());
        
        let attr = attr.unwrap();
        assert_eq!(attr.name, "user.test");
        assert_eq!(attr.value, alloc::vec![1, 2, 3]);
        assert_eq!(attr.namespace(), Some("user."));
        assert_eq!(attr.name_without_namespace(), "test");
    }
    
    #[test]
    fn test_invalid_xattr_creation() {
        let attr = Xattr::new("invalid".to_string(), alloc::vec![1, 2, 3]);
        assert!(attr.is_err());
        
        let attr = Xattr::new("".to_string(), alloc::vec![1, 2, 3]);
        assert!(attr.is_err());
    }
    
    #[test]
    fn test_xattr_collection() {
        let mut collection = XattrCollection::new();
        
        // Set some attributes
        assert!(collection.set("user.test1".to_string(), alloc::vec![1, 2, 3], XattrFlags::Default).is_ok());
        assert!(collection.set("user.test2".to_string(), alloc::vec![4, 5, 6], XattrFlags::Default).is_ok());
        assert!(collection.set("security.label".to_string(), alloc::vec![7, 8, 9], XattrFlags::Default).is_ok());
        
        // Check collection size
        assert_eq!(collection.len(), 3);
        assert!(!collection.is_empty());
        
        // Get attributes
        let attr = collection.get("user.test1");
        assert!(attr.is_some());
        assert_eq!(attr.unwrap().value, alloc::vec![1, 2, 3]);
        
        let attr = collection.get("nonexistent");
        assert!(attr.is_none());
        
        // List all names
        let names = collection.list_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"user.test1".to_string()));
        assert!(names.contains(&"user.test2".to_string()));
        assert!(names.contains(&"security.label".to_string()));
        
        // List names in namespace
        let user_names = collection.list_names_in_namespace("user.");
        assert_eq!(user_names.len(), 2);
        assert!(user_names.contains(&"user.test1".to_string()));
        assert!(user_names.contains(&"user.test2".to_string()));
        
        let security_names = collection.list_names_in_namespace("security.");
        assert_eq!(security_names.len(), 1);
        assert!(security_names.contains(&"security.label".to_string()));
        
        // Remove an attribute
        assert!(collection.remove("user.test1").is_ok());
        assert_eq!(collection.len(), 2);
        assert!(collection.get("user.test1").is_none());
        
        // Try to remove non-existent attribute
        assert!(collection.remove("nonexistent").is_err());
    }
    
    #[test]
    fn test_xattr_flags() {
        let mut collection = XattrCollection::new();
        
        // Test Create flag
        assert!(collection.set("user.test".to_string(), alloc::vec![1, 2, 3], XattrFlags::Create).is_ok());
        assert!(collection.set("user.test".to_string(), alloc::vec![4, 5, 6], XattrFlags::Create).is_err());
        
        // Test Replace flag
        assert!(collection.set("user.test2".to_string(), alloc::vec![7, 8, 9], XattrFlags::Replace).is_err());
        assert!(collection.set("user.test".to_string(), alloc::vec![7, 8, 9], XattrFlags::Replace).is_ok());
        
        // Verify replacement
        let attr = collection.get("user.test");
        assert_eq!(attr.unwrap().value, alloc::vec![7, 8, 9]);
    }
}
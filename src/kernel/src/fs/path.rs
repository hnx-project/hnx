extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::fs::vfs::FsError;

/// Maximum path length
pub const MAX_PATH_LEN: usize = 4096;

/// Maximum component name length
pub const MAX_COMPONENT_LEN: usize = 255;

/// Canonicalize a path by resolving '.' and '..' components
pub fn canonicalize_path(path: &str) -> Result<String, FsError> {
    if path.is_empty() {
        return Err(FsError::InvalidPath);
    }
    
    // Handle absolute vs relative paths
    if path.starts_with('/') {
        // Absolute path
        normalize_absolute_path(path)
    } else {
        // Relative path - resolve against current working directory
        // For now, assume root directory as current working directory
        let abs_path = if path.is_empty() {
            String::from("/")
        } else {
            let mut s = String::from("/");
            s.push_str(path);
            s
        };
        normalize_absolute_path(&abs_path)
    }
}

/// Normalize an absolute path
fn normalize_absolute_path(path: &str) -> Result<String, FsError> {
    if path.is_empty() || !path.starts_with('/') {
        return Err(FsError::InvalidPath);
    }
    
    let mut components = Vec::new();
    
    for component in path.split('/') {
        if component.is_empty() || component == "." {
            continue;
        } else if component == ".." {
            if !components.is_empty() {
                components.pop();
            }
        } else {
            if component.len() > MAX_COMPONENT_LEN {
                return Err(FsError::NameTooLong);
            }
            components.push(component);
        }
    }
    
    let normalized = if components.is_empty() {
        String::from("/")
    } else {
        let mut s = String::from("/");
        s.push_str(&components.join("/"));
        s
    };
    
    if normalized.len() > MAX_PATH_LEN {
        return Err(FsError::NameTooLong);
    }
    
    Ok(normalized)
}

/// Join two paths together
pub fn join_paths(base: &str, relative: &str) -> String {
    if relative.starts_with('/') {
        String::from(relative)
    } else {
        let mut s = String::from(base.trim_end_matches('/'));
        s.push('/');
        s.push_str(relative);
        s
    }
}

/// Get the parent path of a given path
pub fn get_parent_path(path: &str) -> String {
    if path.is_empty() || path == "/" {
        return String::from("/");
    }
    
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return String::from("/");
    }
    
    let pos = trimmed.rfind('/').unwrap_or(0);
    if pos == 0 {
        String::from("/")
    } else {
        String::from(&trimmed[..pos])
    }
}

/// Get the basename (last component) of a path
pub fn get_basename(path: &str) -> String {
    if path.is_empty() || path == "/" {
        return String::new();
    }
    
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    
    let pos = trimmed.rfind('/').map(|p| p + 1).unwrap_or(0);
    String::from(&trimmed[pos..])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_canonicalize_path() {
        assert_eq!(canonicalize_path("/").unwrap(), "/");
        assert_eq!(canonicalize_path("/.").unwrap(), "/");
        assert_eq!(canonicalize_path("/..").unwrap(), "/");
        assert_eq!(canonicalize_path("/foo/bar").unwrap(), "/foo/bar");
        assert_eq!(canonicalize_path("/foo/bar/.").unwrap(), "/foo/bar");
        assert_eq!(canonicalize_path("/foo/bar/..").unwrap(), "/foo");
        assert_eq!(canonicalize_path("/foo/bar/../baz").unwrap(), "/foo/baz");
    }
    
    #[test]
    fn test_get_parent_path() {
        assert_eq!(get_parent_path("/"), "/");
        assert_eq!(get_parent_path("/foo"), "/");
        assert_eq!(get_parent_path("/foo/bar"), "/foo");
        assert_eq!(get_parent_path("/foo/bar/"), "/foo");
    }
    
    #[test]
    fn test_get_basename() {
        assert_eq!(get_basename("/"), "");
        assert_eq!(get_basename("/foo"), "foo");
        assert_eq!(get_basename("/foo/bar"), "bar");
        assert_eq!(get_basename("/foo/bar/"), "bar");
    }
}
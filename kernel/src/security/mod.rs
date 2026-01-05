//! Security and Capability System
//!
//! This module implements the capability-based security model for HNX microkernel:
//! - **Capabilities**: Fine-grained access control tokens
//! - **Rights Management**: READ, WRITE, EXECUTE, MANAGE permissions
//! - **System Call Authorization**: Validating syscall access rights
//! - **Object Protection**: Ensuring only authorized processes access resources
//!
//! The capability model ensures that processes can only perform operations
//! for which they have been explicitly granted rights, following the principle
//! of least privilege.

pub mod capability;
pub mod test;

use core::sync::atomic::{AtomicU32, Ordering};
use shared::sync::mutex::Mutex;

// Re-export commonly used items
pub use capability::{validate_capability as validate_security_capability, rights as security_rights};

/// Rights that can be granted by capabilities
pub mod rights {
    /// Right to read from an object
    pub const READ: u32 = 0x1;
    /// Right to write to an object
    pub const WRITE: u32 = 0x2;
    /// Right to execute an object
    pub const EXECUTE: u32 = 0x4;
    /// Right to manage an object (grant/revoke capabilities)
    pub const MANAGE: u32 = 0x8;
    
    /// All rights combined
    pub const ALL: u32 = READ | WRITE | EXECUTE | MANAGE;
}

/// Represents a security capability granting specific rights to a resource
#[derive(Copy, Clone, Debug)]
pub struct Capability {
    /// Unique identifier for this capability
    pub id: u32,
    /// The object this capability grants access to
    pub object_id: u32,
    /// Rights granted by this capability (bitmask)
    pub rights: u32,
}

/// Maximum number of capabilities in the system
const MAX_CAPABILITIES: usize = 256;

/// Global capability table
static CAPABILITY_TABLE: Mutex<[Option<Capability>; MAX_CAPABILITIES]> = 
    Mutex::new([None; MAX_CAPABILITIES]);

/// Next available capability ID
static NEXT_CAP_ID: AtomicU32 = AtomicU32::new(1);

/// Initialize the security subsystem
pub fn init() {
    crate::info!("security: initializing capability system");
}

/// Allocates a new capability with the specified rights to an object
pub fn allocate_capability(object_id: u32, rights: u32) -> Option<u32> {
    let id = NEXT_CAP_ID.fetch_add(1, Ordering::Relaxed);
    
    // Prevent integer overflow
    if id == 0 {
        return None;
    }
    
    let cap = Capability {
        id,
        object_id,
        rights,
    };
    
    let mut table = CAPABILITY_TABLE.lock();
    
    // Find an empty slot in the capability table
    for slot in table.iter_mut() {
        if slot.is_none() {
            *slot = Some(cap);
            return Some(id);
        }
    }
    
    // No available slots
    None
}

/// Validates that a capability grants the requested rights to an object
pub fn validate_capability(cap_id: u32, object_id: u32, required_rights: u32) -> bool {
    let table = CAPABILITY_TABLE.lock();
    
    // Look for the capability in the table
    for slot in table.iter() {
        if let Some(cap) = slot {
            if cap.id == cap_id && cap.object_id == object_id {
                // Check that all required rights are granted by this capability
                return (cap.rights & required_rights) == required_rights;
            }
        }
    }
    
    // Capability not found
    false
}

/// Revokes a capability, making it invalid for future use
pub fn revoke_capability(cap_id: u32) -> bool {
    let mut table = CAPABILITY_TABLE.lock();
    
    // Look for the capability in the table
    for slot in table.iter_mut() {
        if let Some(cap) = slot {
            if cap.id == cap_id {
                *slot = None;
                return true;
            }
        }
    }
    
    // Capability not found
    false
}

/// System call identifiers
pub mod syscall_ids {
    /// Write to stdout/stderr
    pub const WRITE: &str = "sys_write";
    /// Exit the current process
    pub const EXIT: &str = "sys_exit";
    /// Get current process ID
    pub const GETPID: &str = "sys_getpid";
    /// Open a file
    pub const OPEN: &str = "sys_open";
    /// Read from a file
    pub const READ: &str = "sys_read";
    /// Close a file descriptor
    pub const CLOSE: &str = "sys_close";
    /// Yield CPU time
    pub const YIELD: &str = "sys_yield";
    /// Wait for IPC
    pub const IPC_WAIT: &str = "sys_ipc_wait";
    /// Wake a process
    pub const IPC_WAKE: &str = "sys_ipc_wake";
    /// Create IPC endpoint
    pub const EP_CREATE: &str = "sys_ep_create";
    /// Send IPC message
    pub const EP_SEND: &str = "sys_ep_send";
    /// Receive IPC message
    pub const EP_RECV: &str = "sys_ep_recv";
}

/// System call required rights mapping
pub fn get_syscall_required_rights(operation: &str) -> u32 {
    match operation {
        syscall_ids::WRITE => rights::WRITE,
        syscall_ids::EXIT => rights::EXECUTE,
        syscall_ids::GETPID => rights::READ,
        syscall_ids::OPEN => rights::READ | rights::WRITE,
        syscall_ids::READ => rights::READ,
        syscall_ids::CLOSE => rights::READ,
        syscall_ids::YIELD => rights::EXECUTE,
        syscall_ids::IPC_WAIT => rights::READ,
        syscall_ids::IPC_WAKE => rights::WRITE,
        syscall_ids::EP_CREATE => rights::MANAGE,
        syscall_ids::EP_SEND => rights::WRITE,
        syscall_ids::EP_RECV => rights::READ,
        _ => rights::ALL, // Default to all rights for unknown operations
    }
}

/// Checks if a process has the required rights for a system operation
pub fn check_system_access(operation: &str, required_rights: u32) -> bool {
    // For now, we'll allow all system operations
    // In a full implementation, this would check the calling process's capabilities
    true
}

/// Validates that the current process has the required rights for a system call
pub fn validate_syscall_access(syscall_name: &str) -> bool {
    let required_rights = get_syscall_required_rights(syscall_name);
    // In a full implementation, this would check the calling process's capabilities
    // For now, we'll allow all system calls
    check_system_access(syscall_name, required_rights)
}

#[cfg(test)]
mod test;
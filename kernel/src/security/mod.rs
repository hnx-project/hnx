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

// Re-export commonly used items from capability module
pub use capability::{CapabilityId, CapabilityType, Capability, rights as security_rights};

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
        syscall_ids::WRITE => security_rights::WRITE as u32,
        syscall_ids::EXIT => security_rights::MANAGE as u32, // Changed from EXECUTE for clarity
        syscall_ids::GETPID => security_rights::READ as u32,
        syscall_ids::OPEN => security_rights::READ as u32 | security_rights::WRITE as u32,
        syscall_ids::READ => security_rights::READ as u32,
        syscall_ids::CLOSE => security_rights::MANAGE as u32, // Changed from READ for clarity
        syscall_ids::YIELD => security_rights::EXECUTE as u32,
        syscall_ids::IPC_WAIT => security_rights::READ as u32,
        syscall_ids::IPC_WAKE => security_rights::WRITE as u32,
        syscall_ids::EP_CREATE => security_rights::MANAGE as u32,
        syscall_ids::EP_SEND => security_rights::SEND as u32, // Use specific SEND right
        syscall_ids::EP_RECV => security_rights::RECV as u32, // Use specific RECV right
        _ => security_rights::ALL as u32, // Default to all rights for unknown operations
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


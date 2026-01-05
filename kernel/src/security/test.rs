//! Security system tests

#[cfg(test)]
mod tests {
    use crate::kernel;
    use crate::security;
    use crate::security::{CapabilityId, security_rights};
    
    #[test]
    fn test_capability_allocation() {
        // Test that we can allocate a capability
        let cap_id = kernel::get_kernel().capability_manager.lock().allocate_capability_by_object(1, security_rights::READ as u32);
        assert!(cap_id.is_some());
    }
    
    #[test]
    fn test_capability_validation() {
        // Allocate a capability
        let cap_id = kernel::get_kernel().capability_manager.lock().allocate_capability_by_object(1, security_rights::READ as u32).unwrap();
        
        // Validate that the capability grants read rights
        assert!(kernel::get_kernel().capability_manager.lock().validate_capability_by_object(CapabilityId(cap_id), 1, security_rights::READ as u32));
        
        // Validate that the capability does not grant write rights
        assert!(!kernel::get_kernel().capability_manager.lock().validate_capability_by_object(CapabilityId(cap_id), 1, security_rights::WRITE as u32));
    }
    
    #[test]
    fn test_capability_revocation() {
        // Allocate a capability
        let cap_id = kernel::get_kernel().capability_manager.lock().allocate_capability_by_object(1, security_rights::READ as u32).unwrap();
        
        // Validate that the capability works
        assert!(kernel::get_kernel().capability_manager.lock().validate_capability_by_object(CapabilityId(cap_id), 1, security_rights::READ as u32));
        
        // Revoke the capability
        assert!(kernel::get_kernel().capability_manager.lock().revoke_capability_by_object(CapabilityId(cap_id)));
        
        // Validate that the capability no longer works
        assert!(!kernel::get_kernel().capability_manager.lock().validate_capability_by_object(CapabilityId(cap_id), 1, security_rights::READ as u32));
    }
    
    #[test]
    fn test_syscall_rights_mapping() {
        // Test that system call rights mapping works correctly
        assert_eq!(
            security::get_syscall_required_rights("sys_write"), 
            security::security_rights::WRITE as u32
        );
        
        assert_eq!(
            security::get_syscall_required_rights("sys_open"), 
            security::security_rights::READ as u32 | security::security_rights::WRITE as u32
        );
        
        assert_eq!(
            security::get_syscall_required_rights("sys_ep_create"), 
            security::security_rights::MANAGE as u32
        );
    }
}
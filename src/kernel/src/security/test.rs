//! Security system tests

#[cfg(test)]
mod tests {
    use crate::security;
    
    #[test]
    fn test_capability_allocation() {
        // Test that we can allocate a capability
        let cap_id = security::allocate_capability(1, security::rights::READ);
        assert!(cap_id.is_some());
    }
    
    #[test]
    fn test_capability_validation() {
        // Allocate a capability
        let cap_id = security::allocate_capability(1, security::rights::READ).unwrap();
        
        // Validate that the capability grants read rights
        assert!(security::validate_capability(cap_id, 1, security::rights::READ));
        
        // Validate that the capability does not grant write rights
        assert!(!security::validate_capability(cap_id, 1, security::rights::WRITE));
    }
    
    #[test]
    fn test_capability_revocation() {
        // Allocate a capability
        let cap_id = security::allocate_capability(1, security::rights::READ).unwrap();
        
        // Validate that the capability works
        assert!(security::validate_capability(cap_id, 1, security::rights::READ));
        
        // Revoke the capability
        assert!(security::revoke_capability(cap_id));
        
        // Validate that the capability no longer works
        assert!(!security::validate_capability(cap_id, 1, security::rights::READ));
    }
    
    #[test]
    fn test_syscall_rights_mapping() {
        // Test that system call rights mapping works correctly
        assert_eq!(
            security::syscall::get_required_rights("sys_write"), 
            security::rights::WRITE
        );
        
        assert_eq!(
            security::syscall::get_required_rights("sys_open"), 
            security::rights::READ | security::rights::WRITE
        );
        
        assert_eq!(
            security::syscall::get_required_rights("sys_ep_create"), 
            security::rights::MANAGE
        );
    }
}
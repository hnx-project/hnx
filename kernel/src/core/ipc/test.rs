//! Test module for the enhanced IPC system

use crate::core::ipc::*;

/// Test basic endpoint creation and destruction
pub fn test_endpoint_management() -> bool {
    // Create an endpoint with read/write capabilities
    let capabilities = EndpointCapabilities {
        read: true,
        write: true,
        admin: true,
    };
    
    match endpoint_create(capabilities) {
        Ok(epid) => {
            // Try to destroy the endpoint
            let destroy_result = endpoint_destroy(epid);
            destroy_result.is_ok()
        }
        Err(_) => false,
    }
}

/// Test synchronous message sending and receiving
pub fn test_sync_communication() -> bool {
    // Create an endpoint with read/write capabilities
    let capabilities = EndpointCapabilities {
        read: true,
        write: true,
        admin: true,
    };
    
    match endpoint_create(capabilities) {
        Ok(epid) => {
            // Create a test message
            let mut data = [0u8; 256];
            data[0] = 0x42;
            data[1] = 0x43;
            
            let msg = IpcMessage {
                msg_id: 0, // Will be filled by IPC layer
                src_pid: 1,
                dst_epid: epid,
                op: 1,
                priority: Priority::Normal,
                data_len: 2,
                data,
                timestamp: 0,
            };
            
            // Send the message
            match endpoint_send_sync(epid, msg) {
                Ok(_) => {
                    // Clean up
                    let _ = endpoint_destroy(epid);
                    true
                }
                Err(_) => {
                    let _ = endpoint_destroy(epid);
                    false
                }
            }
        }
        Err(_) => false,
    }
}

/// Test capability management
pub fn test_capabilities() -> bool {
    // Create an endpoint with only read capability
    let read_only_cap = EndpointCapabilities {
        read: true,
        write: false,
        admin: false,
    };
    
    match endpoint_create(read_only_cap) {
        Ok(epid) => {
            // Try to grant write capability
            let write_cap = EndpointCapabilities {
                read: true,
                write: true,
                admin: false,
            };
            
            let grant_result = endpoint_grant_capability(epid, 1, write_cap);
            let success = grant_result.is_ok();
            
            let _ = endpoint_destroy(epid);
            success
        }
        Err(_) => false,
    }
}

/// Test priority-based messaging
pub fn test_priority_messaging() -> bool {
    // Create an endpoint with full capabilities
    let capabilities = EndpointCapabilities {
        read: true,
        write: true,
        admin: true,
    };
    
    match endpoint_create(capabilities) {
        Ok(epid) => {
            // Send messages with different priorities
            let priorities = [Priority::Low, Priority::Normal, Priority::High, Priority::Critical];
            
            let mut success = true;
            for (i, priority) in priorities.iter().enumerate() {
                let mut data = [0u8; 256];
                data[0] = i as u8;
                
                let msg = IpcMessage {
                    msg_id: 0, // Will be filled by IPC layer
                    src_pid: 1,
                    dst_epid: epid,
                    op: 1,
                    priority: *priority,
                    data_len: 1,
                    data,
                    timestamp: crate::arch::timer::now_us(),
                };
                
                match endpoint_send_sync(epid, msg) {
                    Ok(_) => {}
                    Err(_) => {
                        success = false;
                        break;
                    }
                }
            }
            
            // Clean up
            let _ = endpoint_destroy(epid);
            success
        }
        Err(_) => false,
    }
}

/// Run all IPC tests
pub fn run_all_tests() {
    crate::info!("Running IPC tests...");
    
    let test1 = test_endpoint_management();
    crate::info!("Endpoint management test: {}", if test1 { "PASSED" } else { "FAILED" });
    
    let test2 = test_sync_communication();
    crate::info!("Sync communication test: {}", if test2 { "PASSED" } else { "FAILED" });
    
    let test3 = test_capabilities();
    crate::info!("Capabilities test: {}", if test3 { "PASSED" } else { "FAILED" });
    
    let test4 = test_priority_messaging();
    crate::info!("Priority messaging test: {}", if test4 { "PASSED" } else { "FAILED" });
    
    crate::info!("IPC tests completed.");
}
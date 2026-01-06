//! Example usage of the enhanced IPC system

use crate::core::ipc::*;

/// Example of creating an endpoint and sending a message
pub fn example_basic_usage() {
    // Create an endpoint with full capabilities
    let capabilities = EndpointCapabilities {
        read: true,
        write: true,
        admin: true,
    };
    
    match endpoint_create(capabilities) {
        Ok(epid) => {
            crate::info!("Created endpoint with ID: {}", epid);
            
            // Create a message
            let mut data = [0u8; 256];
            data[0] = 0x48; // 'H'
            data[1] = 0x65; // 'e'
            data[2] = 0x6C; // 'l'
            data[3] = 0x6C; // 'l'
            data[4] = 0x6F; // 'o'
            
            let msg = IpcMessage {
                msg_id: 0, // Will be filled by IPC layer
                src_pid: 1,
                dst_epid: epid,
                op: 1, // Some operation code
                priority: Priority::Normal,
                data_len: 5,
                data,
                timestamp: crate::arch::timer::now_us(),
            };
            
            // Send the message synchronously
            match endpoint_send_sync(epid, msg) {
                Ok(response) => {
                    crate::info!("Message sent successfully, response code: {}", response.code);
                }
                Err(e) => {
                    crate::info!("Failed to send message: {:?}", e);
                }
            }
            
            // Clean up
            let _ = endpoint_destroy(epid);
        }
        Err(e) => {
            crate::info!("Failed to create endpoint: {:?}", e);
        }
    }
}

/// Example of using priority-based messaging
pub fn example_priority_messaging() {
    // Create an endpoint with full capabilities
    let capabilities = EndpointCapabilities {
        read: true,
        write: true,
        admin: true,
    };
    
    if let Ok(epid) = endpoint_create(capabilities) {
        crate::info!("Created endpoint with ID: {}", epid);
        
        // Send messages with different priorities
        let priorities = [Priority::Low, Priority::Normal, Priority::High, Priority::Critical];
        
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
                Ok(_) => {
                    crate::info!("Sent {} priority message", 
                        match priority {
                            Priority::Low => "low",
                            Priority::Normal => "normal",
                            Priority::High => "high",
                            Priority::Critical => "critical",
                        }
                    );
                }
                Err(e) => {
                    crate::info!("Failed to send {} priority message: {:?}", 
                        match priority {
                            Priority::Low => "low",
                            Priority::Normal => "normal",
                            Priority::High => "high",
                            Priority::Critical => "critical",
                        }, e
                    );
                }
            }
        }
        
        // When receiving, critical messages should be received first
        // This demonstrates the priority-based ordering
        
        // Clean up
        let _ = endpoint_destroy(epid);
    }
}

/// Example of capability-based access control
pub fn example_capabilities() {
    // Create an endpoint with read-only capabilities
    let read_only_caps = EndpointCapabilities {
        read: true,
        write: false,
        admin: false,
    };
    
    if let Ok(epid) = endpoint_create(read_only_caps) {
        crate::info!("Created read-only endpoint with ID: {}", epid);
        
        // Try to send a message (should fail)
        let msg = IpcMessage {
            msg_id: 0, // Will be filled by IPC layer
            src_pid: 1,
            dst_epid: epid,
            op: 1,
            priority: Priority::Normal,
            data_len: 0,
            data: [0; 256],
            timestamp: crate::arch::timer::now_us(),
        };
        
        match endpoint_send_sync(epid, msg) {
            Ok(_) => {
                crate::info!("Unexpectedly succeeded in sending to read-only endpoint");
            }
            Err(IpcError::PermissionDenied) => {
                crate::info!("Correctly denied write access to read-only endpoint");
            }
            Err(e) => {
                crate::info!("Unexpected error: {:?}", e);
            }
        }
        
        // Try to grant write capability (would need admin rights)
        let write_caps = EndpointCapabilities {
            read: true,
            write: true,
            admin: false,
        };
        
        match endpoint_grant_capability(epid, 1, write_caps) {
            Ok(_) => {
                crate::info!("Granted write capability");
            }
            Err(IpcError::PermissionDenied) => {
                crate::info!("Correctly denied capability modification (no admin rights)");
            }
            Err(e) => {
                crate::info!("Unexpected error: {:?}", e);
            }
        }
        
        // Clean up
        let _ = endpoint_destroy(epid);
    }
}

/// Run all examples
pub fn run_all_examples() {
    crate::info!("Running IPC examples...");
    
    example_basic_usage();
    example_priority_messaging();
    example_capabilities();
    
    crate::info!("IPC examples completed.");
}
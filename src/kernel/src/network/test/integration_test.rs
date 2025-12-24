//! Main integration test for network functionality
//!
//! This test brings together all the network components to verify
//! that the complete socket API works correctly.

extern crate alloc;

use crate::network::test::{NetworkError, Result};
use crate::network::test::socket::{SocketDomain, SocketType, SocketProtocol, SocketAddress};
use crate::network::test::interface::{NetworkInterface, InterfaceFlags};

/// Main integration test for network functionality
pub fn integration_test_network() -> Result<()> {
    crate::info!("network/integration: Starting main network integration test");
    
    // Initialize the network subsystem
    crate::network::init();
    
    // Test 1: Basic socket creation and binding
    crate::info!("network/integration: Test 1 - Socket creation and binding");
    
    // Create a network interface for testing
    let mut interface = NetworkInterface::new(1, "test0");
    interface.set_mac_address([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    interface.set_ip_address(0xC0A80101u32.to_be(), 0xFFFFFF00u32.to_be()); // 192.168.1.1/24
    interface.flags.set(InterfaceFlags::UP);
    
    // Register the interface
    crate::network::interface::register_interface(interface)?;
    
    // Create a TCP socket
    let socket_id = crate::network::socket::socket(
        SocketDomain::INET, 
        SocketType::STREAM, 
        SocketProtocol::TCP
    )?;
    
    crate::info!("network/integration: Created socket with ID {}", socket_id);
    
    // Bind the socket to a local address
    let local_addr = SocketAddress::new(0xC0A80101u32, 9000u16); // 192.168.1.1:9000
    crate::network::socket::bind(socket_id, local_addr)?;
    
    crate::info!("network/integration: Bound socket to address");
    
    // Test 2: Listening functionality
    crate::info!("network/integration: Test 2 - Listening functionality");
    
    // Listen for incoming connections with a backlog of 10
    crate::network::socket::listen(socket_id, 10)?;
    
    crate::info!("network/integration: Started listening with backlog of 10");
    
    // Test 3: Socket API functions
    crate::info!("network/integration: Test 3 - Socket API functions");
    
    // Test that we can close the socket
    crate::network::socket::close(socket_id)?;
    
    crate::info!("network/integration: Closed socket successfully");
    
    // Clean up
    crate::network::interface::unregister_interface(1)?;
    
    crate::info!("network/integration: Main network integration test completed successfully");
    
    Ok(())
}
//! Integration test for TCP accept functionality

extern crate alloc;

use crate::network::test::{NetworkError, Result};
use crate::network::test::socket::{SocketDomain, SocketType, SocketProtocol, SocketAddress};
use crate::network::test::interface::{NetworkInterface, InterfaceFlags};

/// Integration test for TCP accept functionality
pub fn integration_test_tcp_accept() -> Result<()> {
    crate::info!("network/integration_test: Starting TCP accept integration test");
    
    // Initialize the network subsystem
    crate::network::init();
    
    // Create a network interface for testing
    let mut interface = NetworkInterface::new(1, "test0");
    interface.set_mac_address([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    interface.set_ip_address(0x0A000001u32.to_be(), 0xFFFFFF00u32.to_be()); // 10.0.0.1/24
    interface.flags.set(InterfaceFlags::UP);
    
    // Register the interface
    crate::network::interface::register_interface(interface)?;
    
    // Create a TCP socket
    let socket_id = crate::network::socket::socket(
        SocketDomain::INET, 
        SocketType::STREAM, 
        SocketProtocol::TCP
    )?;
    
    crate::info!("network/integration_test: Created socket with ID {}", socket_id);
    
    // Bind the socket to a local address
    let local_addr = SocketAddress::new(0x0A000001u32, 8080u16); // 10.0.0.1:8080
    crate::network::socket::bind(socket_id, local_addr)?;
    
    crate::info!("network/integration_test: Bound socket to address");
    
    // Listen for incoming connections with a backlog of 5
    crate::network::socket::listen(socket_id, 5)?;
    
    crate::info!("network/integration_test: Started listening with backlog of 5");
    
    // Test that accept returns Timeout when no connections are available
    let accept_result = crate::network::socket::accept(socket_id);
    assert_eq!(accept_result, Err(NetworkError::Timeout));
    crate::info!("network/integration_test: Verified accept returns Timeout when no connections available");
    
    // Clean up
    crate::network::socket::close(socket_id)?;
    crate::network::interface::unregister_interface(1)?;
    
    crate::info!("network/integration_test: TCP accept integration test completed successfully");
    
    Ok(())
}
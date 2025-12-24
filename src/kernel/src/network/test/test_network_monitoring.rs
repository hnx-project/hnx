//! Network monitoring test

extern crate alloc;

use crate::network::test::{NetworkError, Result};
use crate::network::test::interface::{NetworkInterface, InterfaceFlags};
use crate::network::test::monitoring;

/// Test network monitoring functionality
pub fn test_network_monitoring() -> Result<()> {
    crate::info!("network/monitoring: Starting monitoring test");
    
    // Network subsystem is already initialized in main.rs, so we don't need to initialize it again
    crate::info!("network/monitoring: Network subsystem already initialized");
    
    // Use a unique ID to avoid conflicts with previous tests
    let interface_id = crate::network::interface::allocate_interface_id();
    
    // Create a network interface for testing
    let mut interface = NetworkInterface::new(interface_id, "test0");
    interface.set_mac_address([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    interface.set_ip_address(0xC0A80101u32.to_be(), 0xFFFFFF00u32.to_be()); // 192.168.1.1/24
    interface.flags.set(InterfaceFlags::UP);
    
    // Register the interface
    crate::network::interface::register_interface(interface)?;
    
    // Get initial statistics
    if let Some(stats) = monitoring::get_network_stats() {
        crate::info!("network/monitoring: Initial stats collected");
        let formatted = monitoring::format_network_stats(&stats);
        crate::info!("network/monitoring: Stats:\n{}", formatted);
    } else {
        crate::info!("network/monitoring: Failed to get initial stats");
        return Err(NetworkError::InvalidParameter);
    }
    
    // Simulate some network activity by updating stats directly
    crate::network::interface::update_rx_stats(interface_id, 1000, 0)?;  // 1000 bytes received, no errors
    crate::network::interface::update_tx_stats(interface_id, 500, 0)?;   // 500 bytes sent, no errors
    crate::network::interface::update_rx_stats(interface_id, 1500, 1)?;  // 1500 bytes received, 1 error
    crate::network::interface::update_tx_stats(interface_id, 2000, 0)?;  // 2000 bytes sent, no errors
    
    // Get updated statistics
    if let Some(stats) = monitoring::get_network_stats() {
        crate::info!("network/monitoring: Updated stats collected");
        let formatted = monitoring::format_network_stats(&stats);
        crate::info!("network/monitoring: Updated Stats:\n{}", formatted);
        
        // Verify the stats are correct
        if stats.total_rx_packets != 2 || stats.total_tx_packets != 2 ||
           stats.total_rx_bytes != 2500 || stats.total_tx_bytes != 2500 ||
           stats.total_rx_errors != 1 || stats.total_tx_errors != 0 {
            crate::info!("network/monitoring: Stats verification failed");
            return Err(NetworkError::InvalidParameter);
        }
    } else {
        crate::info!("network/monitoring: Failed to get updated stats");
        return Err(NetworkError::InvalidParameter);
    }
    
    // Test interface-specific statistics
    if let Some(interface_stats) = monitoring::get_interface_statistics(interface_id) {
        crate::info!("network/monitoring: Interface-specific stats collected");
        if interface_stats.rx_packets != 2 || interface_stats.tx_packets != 2 ||
           interface_stats.rx_bytes != 2500 || interface_stats.tx_bytes != 2500 ||
           interface_stats.rx_errors != 1 || interface_stats.tx_errors != 0 {
            crate::info!("network/monitoring: Interface stats verification failed");
            return Err(NetworkError::InvalidParameter);
        }
    } else {
        crate::info!("network/monitoring: Failed to get interface-specific stats");
        return Err(NetworkError::InvalidParameter);
    }
    
    // Test resetting interface statistics
    monitoring::reset_interface_statistics(interface_id)?;
    
    if let Some(interface_stats) = monitoring::get_interface_statistics(interface_id) {
        if interface_stats.rx_packets != 0 || interface_stats.tx_packets != 0 ||
           interface_stats.rx_bytes != 0 || interface_stats.tx_bytes != 0 ||
           interface_stats.rx_errors != 0 || interface_stats.tx_errors != 0 {
            crate::info!("network/monitoring: Interface stats reset failed");
            return Err(NetworkError::InvalidParameter);
        }
    }
    
    // Clean up
    crate::network::interface::unregister_interface(interface_id)?;
    
    crate::info!("network/monitoring: Monitoring test completed successfully");
    
    Ok(())
}
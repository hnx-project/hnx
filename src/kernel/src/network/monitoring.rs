//! Network monitoring and statistics collection
//!
//! This module provides functionality for collecting and reporting
//! network statistics across all layers of the network stack.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::interface::{InterfaceStats, get_interface_stats, reset_interface_stats};
use alloc::collections::BTreeMap;
use spin::Mutex;

/// Network monitoring statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// Per-interface statistics
    pub interface_stats: BTreeMap<u32, InterfaceStats>,
    /// Total packets received across all interfaces
    pub total_rx_packets: u64,
    /// Total packets transmitted across all interfaces
    pub total_tx_packets: u64,
    /// Total bytes received across all interfaces
    pub total_rx_bytes: u64,
    /// Total bytes transmitted across all interfaces
    pub total_tx_bytes: u64,
    /// Total receive errors across all interfaces
    pub total_rx_errors: u64,
    /// Total transmit errors across all interfaces
    pub total_tx_errors: u64,
}

impl NetworkStats {
    /// Create new empty network statistics
    pub fn new() -> Self {
        NetworkStats {
            interface_stats: BTreeMap::new(),
            total_rx_packets: 0,
            total_tx_packets: 0,
            total_rx_bytes: 0,
            total_tx_bytes: 0,
            total_rx_errors: 0,
            total_tx_errors: 0,
        }
    }
    
    /// Aggregate statistics from all interfaces
    pub fn aggregate(&mut self) {
        // Reset totals
        self.total_rx_packets = 0;
        self.total_tx_packets = 0;
        self.total_rx_bytes = 0;
        self.total_tx_bytes = 0;
        self.total_rx_errors = 0;
        self.total_tx_errors = 0;
        
        // Get all interfaces and aggregate their stats
        let interfaces = crate::network::interface::list_interfaces();
        self.interface_stats.clear();
        
        for interface in interfaces {
            self.interface_stats.insert(interface.id, interface.stats.clone());
            
            self.total_rx_packets += interface.stats.rx_packets;
            self.total_tx_packets += interface.stats.tx_packets;
            self.total_rx_bytes += interface.stats.rx_bytes;
            self.total_tx_bytes += interface.stats.tx_bytes;
            self.total_rx_errors += interface.stats.rx_errors;
            self.total_tx_errors += interface.stats.tx_errors;
        }
    }
}

/// Global network monitoring state
static NETWORK_MONITORING: Mutex<Option<NetworkStats>> = Mutex::new(None);

/// Initialize the network monitoring subsystem
pub fn init() {
    crate::info!("network/monitoring: initializing");
    
    let mut monitoring = NETWORK_MONITORING.lock();
    *monitoring = Some(NetworkStats::new());
    
    crate::info!("network/monitoring: initialized");
}

/// Get current network statistics
pub fn get_network_stats() -> Option<NetworkStats> {
    let mut monitoring = NETWORK_MONITORING.lock();
    
    if let Some(stats) = monitoring.as_mut() {
        stats.aggregate();
        Some(stats.clone())
    } else {
        None
    }
}

/// Get statistics for a specific network interface
pub fn get_interface_statistics(interface_id: u32) -> Option<InterfaceStats> {
    get_interface_stats(interface_id)
}

/// Reset statistics for a specific network interface
pub fn reset_interface_statistics(interface_id: u32) -> Result<()> {
    reset_interface_stats(interface_id)
}

/// Reset all network statistics
pub fn reset_all_statistics() -> Result<()> {
    let interfaces = crate::network::interface::list_interfaces();
    
    for interface in interfaces {
        reset_interface_stats(interface.id)?;
    }
    
    // Reset global monitoring state
    let mut monitoring = NETWORK_MONITORING.lock();
    if let Some(stats) = monitoring.as_mut() {
        *stats = NetworkStats::new();
    }
    
    Ok(())
}

/// Format network statistics for display
pub fn format_network_stats(stats: &NetworkStats) -> alloc::string::String {
    use alloc::format;
    
    let mut output = alloc::string::String::new();
    
    output.push_str("Network Statistics:\n");
    output.push_str("==================\n");
    output.push_str(&format!("Total RX Packets: {}\n", stats.total_rx_packets));
    output.push_str(&format!("Total TX Packets: {}\n", stats.total_tx_packets));
    output.push_str(&format!("Total RX Bytes: {}\n", stats.total_rx_bytes));
    output.push_str(&format!("Total TX Bytes: {}\n", stats.total_tx_bytes));
    output.push_str(&format!("Total RX Errors: {}\n", stats.total_rx_errors));
    output.push_str(&format!("Total TX Errors: {}\n", stats.total_tx_errors));
    output.push_str("\nPer-Interface Statistics:\n");
    
    for (id, interface_stats) in &stats.interface_stats {
        output.push_str(&format!("\nInterface ID {}: \n", id));
        output.push_str(&format!("  RX Packets: {}\n", interface_stats.rx_packets));
        output.push_str(&format!("  TX Packets: {}\n", interface_stats.tx_packets));
        output.push_str(&format!("  RX Bytes: {}\n", interface_stats.rx_bytes));
        output.push_str(&format!("  TX Bytes: {}\n", interface_stats.tx_bytes));
        output.push_str(&format!("  RX Errors: {}\n", interface_stats.rx_errors));
        output.push_str(&format!("  TX Errors: {}\n", interface_stats.tx_errors));
    }
    
    output
}
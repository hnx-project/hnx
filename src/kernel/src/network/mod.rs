//! Network subsystem for the HNX kernel
//!
//! This module implements the TCP/IP protocol stack and provides
//! networking capabilities to the kernel and applications.

extern crate alloc;

pub mod interface;
pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod ipv6;
pub mod icmp;
pub mod udp;
pub mod tcp;
pub mod tcp_congestion;
pub mod socket;
pub mod dhcp;
pub mod dns;
pub mod monitoring;
pub mod firewall;
pub mod firewall_enhanced;
pub mod qos;
pub mod qos_enhanced;
pub mod qos_example;
pub mod firewall_example;
pub mod ipc_protocol;
pub mod manager;
pub mod zerocopy;
#[cfg(test)]
pub mod test;

/// Network error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkError {
    /// Invalid parameter
    InvalidParameter,
    /// Out of memory
    OutOfMemory,
    /// Operation not supported
    NotSupported,
    /// Network interface not found
    InterfaceNotFound,
    /// Timeout occurred
    Timeout,
    /// Connection refused
    ConnectionRefused,
    /// Connection reset
    ConnectionReset,
    /// Host unreachable
    HostUnreachable,
    /// Network unreachable
    NetworkUnreachable,
    /// Address in use
    AddressInUse,
    /// Address not available
    AddressNotAvailable,
    /// Resource temporarily unavailable (for non-blocking I/O)
    WouldBlock,
}

/// Result type for network operations
pub type Result<T> = core::result::Result<T, NetworkError>;

/// Initialize the network subsystem
pub fn init() {
    crate::info!("network: initializing network subsystem");
    
    // Initialize subsystems
    interface::init();
    ethernet::init();
    arp::init();
    ipv4::init();
    ipv6::init();
    icmp::init();
    udp::init();
    tcp::init();
    socket::init();
    dhcp::init();
    dns::init();
    monitoring::init();
    firewall::init();
    qos::init();
    
    // Initialize the network manager
    // Note: Network server registration will happen later when user-space starts

    
    crate::info!("network: network subsystem initialized");
}
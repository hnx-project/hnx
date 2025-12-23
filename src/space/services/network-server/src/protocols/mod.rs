//! Network protocols implementation for user-space network server

pub mod tcp;
pub mod udp;
pub mod ipv4;
pub mod ipv6;
pub mod arp;
pub mod icmp;
pub mod dhcp;
pub mod dns;

/// Initialize all protocol implementations
pub fn init() {
    crate::println!("Initializing network protocols...");
    // Protocol initialization would happen here
}
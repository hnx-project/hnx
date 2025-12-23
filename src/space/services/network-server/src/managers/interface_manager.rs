//! Interface manager for user-space network server

use alloc::collections::BTreeMap;
use spin::Mutex;

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub id: u32,
    pub name: alloc::string::String,
    pub mac_address: [u8; 6],
    pub ip_addresses: alloc::vec::Vec<IpAddress>,
    pub mtu: u16,
}

/// IP address representation
#[derive(Debug, Clone)]
pub enum IpAddress {
    Ipv4(u32),
    Ipv6([u8; 16]),
}

/// Interface manager responsible for managing network interfaces
pub struct InterfaceManager {
    interfaces: Mutex<BTreeMap<u32, NetworkInterface>>,
}

impl InterfaceManager {
    /// Create a new interface manager
    pub fn new() -> Self {
        Self {
            interfaces: Mutex::new(BTreeMap::new()),
        }
    }

    /// Add a network interface
    pub fn add_interface(&self, interface: NetworkInterface) {
        self.interfaces.lock().insert(interface.id, interface);
    }

    /// Get interface information
    pub fn get_interface(&self, id: u32) -> Option<NetworkInterface> {
        self.interfaces.lock().get(&id).cloned()
    }

    /// Remove an interface
    pub fn remove_interface(&self, id: u32) -> Result<(), ()> {
        if self.interfaces.lock().remove(&id).is_some() {
            Ok(())
        } else {
            Err(())
        }
    }
}
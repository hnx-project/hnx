//! Network interface management

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Network interface flags
#[derive(Debug, Clone, Copy)]
pub struct InterfaceFlags(u32);

impl InterfaceFlags {
    pub const UP: u32 = 1 << 0;
    pub const BROADCAST: u32 = 1 << 1;
    pub const LOOPBACK: u32 = 1 << 2;
    pub const POINT_TO_POINT: u32 = 1 << 3;
    pub const RUNNING: u32 = 1 << 4;
    pub const NO_ARP: u32 = 1 << 5;
    pub const PROMISC: u32 = 1 << 6;
    
    pub fn new() -> Self {
        InterfaceFlags(0)
    }
    
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }
    
    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }
    
    pub fn is_set(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
}

/// Network interface statistics
#[derive(Debug, Clone)]
pub struct InterfaceStats {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

impl InterfaceStats {
    pub fn new() -> Self {
        InterfaceStats {
            rx_packets: 0,
            tx_packets: 0,
            rx_bytes: 0,
            tx_bytes: 0,
            rx_errors: 0,
            tx_errors: 0,
        }
    }
}

impl core::ops::AddAssign for InterfaceStats {
    fn add_assign(&mut self, other: Self) {
        self.rx_packets += other.rx_packets;
        self.tx_packets += other.tx_packets;
        self.rx_bytes += other.rx_bytes;
        self.tx_bytes += other.tx_bytes;
        self.rx_errors += other.rx_errors;
        self.tx_errors += other.tx_errors;
    }
}

/// Network interface structure
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub id: u32,
    pub name: String,
    pub mac_address: [u8; 6],
    pub ip_address: u32,
    pub netmask: u32,
    pub gateway: u32,
    pub mtu: u16,
    pub flags: InterfaceFlags,
    pub stats: InterfaceStats,
}

impl NetworkInterface {
    pub fn new(id: u32, name: &str) -> Self {
        NetworkInterface {
            id,
            name: String::from(name),
            mac_address: [0; 6],
            ip_address: 0,
            netmask: 0,
            gateway: 0,
            mtu: 1500,
            flags: InterfaceFlags::new(),
            stats: InterfaceStats::new(),
        }
    }
    
    /// Update RX statistics
    pub fn update_rx_stats(&mut self, bytes: u64, errors: u64) {
        self.stats.rx_packets += 1;
        self.stats.rx_bytes += bytes;
        self.stats.rx_errors += errors;
    }
    
    /// Update TX statistics
    pub fn update_tx_stats(&mut self, bytes: u64, errors: u64) {
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += bytes;
        self.stats.tx_errors += errors;
    }
    
    pub fn set_mac_address(&mut self, mac: [u8; 6]) {
        self.mac_address = mac;
    }
    
    pub fn set_ip_address(&mut self, ip: u32, netmask: u32) {
        self.ip_address = ip;
        self.netmask = netmask;
    }
    
    pub fn set_gateway(&mut self, gateway: u32) {
        self.gateway = gateway;
    }
    
    pub fn set_mtu(&mut self, mtu: u16) {
        self.mtu = mtu;
    }
}

/// Global network interface manager
static INTERFACES: Mutex<BTreeMap<u32, NetworkInterface>> = Mutex::new(BTreeMap::new());
static NEXT_INTERFACE_ID: Mutex<u32> = Mutex::new(1);

/// Initialize the network interface subsystem
pub fn init() {
    crate::info!("network/interface: initializing");
}

/// Register a new network interface
pub fn register_interface(mut interface: NetworkInterface) -> Result<u32> {
    let mut interfaces = INTERFACES.lock();
    let id = interface.id;
    
    if interfaces.contains_key(&id) {
        return Err(NetworkError::AddressInUse);
    }
    
    interfaces.insert(id, interface);
    crate::info!("network/interface: registered interface {} with ID {}", 
                 interfaces.get(&id).unwrap().name, id);
    
    Ok(id)
}

/// Unregister a network interface
pub fn unregister_interface(id: u32) -> Result<()> {
    let mut interfaces = INTERFACES.lock();
    
    if !interfaces.contains_key(&id) {
        return Err(NetworkError::InterfaceNotFound);
    }
    
    let interface = interfaces.remove(&id).unwrap();
    crate::info!("network/interface: unregistered interface {} with ID {}", 
                 interface.name, id);
    
    Ok(())
}

/// Get a network interface by ID
pub fn get_interface(id: u32) -> Option<NetworkInterface> {
    let interfaces = INTERFACES.lock();
    interfaces.get(&id).cloned()
}

/// List all network interfaces
pub fn list_interfaces() -> alloc::vec::Vec<NetworkInterface> {
    let interfaces = INTERFACES.lock();
    interfaces.values().cloned().collect()
}

/// Allocate a new interface ID
pub fn allocate_interface_id() -> u32 {
    let mut next_id = NEXT_INTERFACE_ID.lock();
    let id = *next_id;
    *next_id += 1;
    id
}

/// Get statistics for a network interface
pub fn get_interface_stats(id: u32) -> Option<InterfaceStats> {
    let interfaces = INTERFACES.lock();
    interfaces.get(&id).map(|iface| iface.stats.clone())
}

/// Reset statistics for a network interface
pub fn reset_interface_stats(id: u32) -> Result<()> {
    let mut interfaces = INTERFACES.lock();
    
    if let Some(iface) = interfaces.get_mut(&id) {
        iface.stats = InterfaceStats::new();
        Ok(())
    } else {
        Err(NetworkError::InterfaceNotFound)
    }
}

/// Update RX statistics for a network interface
pub fn update_rx_stats(id: u32, bytes: u64, errors: u64) -> Result<()> {
    let mut interfaces = INTERFACES.lock();
    
    if let Some(iface) = interfaces.get_mut(&id) {
        iface.update_rx_stats(bytes, errors);
        Ok(())
    } else {
        Err(NetworkError::InterfaceNotFound)
    }
}

/// Update TX statistics for a network interface
pub fn update_tx_stats(id: u32, bytes: u64, errors: u64) -> Result<()> {
    let mut interfaces = INTERFACES.lock();
    
    if let Some(iface) = interfaces.get_mut(&id) {
        iface.update_tx_stats(bytes, errors);
        Ok(())
    } else {
        Err(NetworkError::InterfaceNotFound)
    }
}
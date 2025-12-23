//! ARP (Address Resolution Protocol) implementation

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;

/// ARP hardware types
pub mod hardware_type {
    pub const ETHERNET: u16 = 1;
}

/// ARP protocol types
pub mod protocol_type {
    pub const IPV4: u16 = 0x0800;
}

/// ARP operation codes
pub mod opcode {
    pub const REQUEST: u16 = 1;
    pub const REPLY: u16 = 2;
}

/// ARP packet header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ArpHeader {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_length: u8,
    pub protocol_length: u8,
    pub opcode: u16,
}

impl ArpHeader {
    pub const SIZE: usize = 8;
    
    /// Create a new ARP header
    pub fn new(hardware_type: u16, protocol_type: u16, opcode: u16) -> Self {
        ArpHeader {
            hardware_type: hardware_type.to_be(),
            protocol_type: protocol_type.to_be(),
            hardware_length: 6,  // Ethernet MAC address length
            protocol_length: 4,  // IPv4 address length
            opcode: opcode.to_be(),
        }
    }
    
    /// Parse an ARP header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        Ok(ArpHeader {
            hardware_type: u16::from_be_bytes([data[0], data[1]]),
            protocol_type: u16::from_be_bytes([data[2], data[3]]),
            hardware_length: data[4],
            protocol_length: data[5],
            opcode: u16::from_be_bytes([data[6], data[7]]),
        })
    }
    
    /// Convert ARP header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0..2].copy_from_slice(&self.hardware_type.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.protocol_type.to_be_bytes());
        bytes[4] = self.hardware_length;
        bytes[5] = self.protocol_length;
        bytes[6..8].copy_from_slice(&self.opcode.to_be_bytes());
        
        bytes
    }
}

/// ARP packet
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ArpPacket {
    pub header: ArpHeader,
    pub sender_mac: [u8; 6],
    pub sender_ip: u32,
    pub target_mac: [u8; 6],
    pub target_ip: u32,
}

impl ArpPacket {
    pub const SIZE: usize = ArpHeader::SIZE + 20;
    
    /// Create a new ARP packet
    pub fn new(header: ArpHeader, sender_mac: [u8; 6], sender_ip: u32, 
               target_mac: [u8; 6], target_ip: u32) -> Self {
        ArpPacket {
            header,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        }
    }
    
    /// Parse an ARP packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = ArpHeader::from_bytes(data)?;
        
        let mut sender_mac = [0u8; 6];
        let mut target_mac = [0u8; 6];
        
        sender_mac.copy_from_slice(&data[8..14]);
        let sender_ip = u32::from_be_bytes([data[14], data[15], data[16], data[17]]);
        target_mac.copy_from_slice(&data[18..24]);
        let target_ip = u32::from_be_bytes([data[24], data[25], data[26], data[27]]);
        
        Ok(ArpPacket {
            header,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        })
    }
    
    /// Convert ARP packet to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0..8].copy_from_slice(&self.header.to_bytes());
        bytes[8..14].copy_from_slice(&self.sender_mac);
        bytes[14..18].copy_from_slice(&self.sender_ip.to_be_bytes());
        bytes[18..24].copy_from_slice(&self.target_mac);
        bytes[24..28].copy_from_slice(&self.target_ip.to_be_bytes());
        
        bytes
    }
}

/// ARP cache entry
#[derive(Debug, Clone)]
struct ArpCacheEntry {
    mac_address: [u8; 6],
    timestamp: u64,  // Timestamp when entry was created/updated
    expires_at: u64, // Expiration time for the entry
}

/// ARP cache
static ARP_CACHE: Mutex<BTreeMap<u32, ArpCacheEntry>> = Mutex::new(BTreeMap::new());

/// Initialize the ARP subsystem
pub fn init() {
    crate::info!("network/arp: initializing");
}

/// Process an incoming ARP packet
pub fn process_packet(data: &[u8]) -> Result<()> {
    let packet = ArpPacket::from_bytes(data)?;
    
    // Check if this is for Ethernet and IPv4
    let hardware_type = u16::from_be(packet.header.hardware_type);
    let protocol_type = u16::from_be(packet.header.protocol_type);
    let opcode = u16::from_be(packet.header.opcode);
    
    if hardware_type != hardware_type::ETHERNET ||
       protocol_type != protocol_type::IPV4 {
        return Err(NetworkError::NotSupported);
    }
    
    match opcode {
        opcode::REQUEST => {
            crate::info!("network/arp: received ARP request for IP {:08X}", 
                         u32::from_be(packet.target_ip));
            // Handle ARP request
            handle_request(&packet)?;
        }
        opcode::REPLY => {
            crate::info!("network/arp: received ARP reply from IP {:08X}", 
                         u32::from_be(packet.sender_ip));
            // Handle ARP reply
            handle_reply(&packet)?;
        }
        _ => {
            crate::info!("network/arp: unsupported opcode {}", opcode);
            return Err(NetworkError::NotSupported);
        }
    }
    
    Ok(())
}

/// Handle an ARP request
fn handle_request(packet: &ArpPacket) -> Result<()> {
    // Get all network interfaces
    let interfaces = crate::network::interface::list_interfaces();
    
    // Check if the target IP belongs to any of our interfaces
    for interface in &interfaces {
        if interface.ip_address == packet.target_ip {
            // Create ARP reply
            let reply_header = ArpHeader::new(
                hardware_type::ETHERNET,
                protocol_type::IPV4,
                opcode::REPLY,
            );
            
            let reply_packet = ArpPacket::new(
                reply_header,
                interface.mac_address,
                packet.target_ip,
                packet.sender_mac,
                packet.sender_ip,
            );
            
            // Convert to bytes
            let reply_bytes = reply_packet.to_bytes();
            
            // Create Ethernet frame for reply
            let eth_header = crate::network::ethernet::EthernetHeader::new(
                packet.sender_mac,
                interface.mac_address,
                crate::network::ethernet::ethertype::ARP,
            );
            
            let reply_frame = crate::network::ethernet::EthernetFrame::new(
                eth_header,
                reply_bytes.to_vec(),
            );
            
            // Send the reply
            crate::network::ethernet::send_frame(interface.id, &reply_frame)?;
            
            crate::info!("network/arp: sent ARP reply for IP {:08X}", 
                         u32::from_be(packet.target_ip));
            
            // Add sender to ARP cache
            add_to_cache(u32::from_be(packet.sender_ip), packet.sender_mac);
            
            return Ok(());
        }
    }
    
    // If target IP doesn't belong to us, just add sender to cache
    let sender_ip = u32::from_be(packet.sender_ip);
    add_to_cache(sender_ip, packet.sender_mac);
    
    Ok(())
}

/// Handle an ARP reply
fn handle_reply(packet: &ArpPacket) -> Result<()> {
    let sender_ip = u32::from_be(packet.sender_ip);
    
    // Add sender to ARP cache
    add_to_cache(sender_ip, packet.sender_mac);
    
    Ok(())
}

/// Add an entry to the ARP cache
fn add_to_cache(ip: u32, mac: [u8; 6]) {
    let ip = u32::from_be(ip);
    
    // In a real implementation, we would use actual timestamp
    // For now, we'll use a placeholder value
    let timestamp = 0;
    let expires_at = timestamp + 300; // Expire after 300 seconds (5 minutes)
    
    let mut cache = ARP_CACHE.lock();
    let entry = ArpCacheEntry {
        mac_address: mac,
        timestamp,
        expires_at,
    };
    cache.insert(ip, entry);
    
    crate::info!("network/arp: cached IP {:08X} -> MAC {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                 ip, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
}

/// Remove expired entries from the ARP cache
fn cleanup_expired_entries() {
    let mut cache = ARP_CACHE.lock();
    
    // In a real implementation, we would use actual timestamp
    let current_time = 0;
    
    cache.retain(|_, entry| entry.expires_at > current_time);
}

/// Lookup MAC address for IP in ARP cache
pub fn lookup(ip: u32) -> Option<[u8; 6]> {
    // Clean up expired entries first
    cleanup_expired_entries();
    
    let cache = ARP_CACHE.lock();
    cache.get(&ip).map(|entry| entry.mac_address)
}

/// Create and send an ARP request
pub fn send_request(interface_id: u32, target_ip: u32) -> Result<()> {
    // Get the network interface
    let interface = match crate::network::interface::get_interface(interface_id) {
        Some(iface) => iface,
        None => return Err(NetworkError::InterfaceNotFound),
    };
    
    // Create ARP header
    let header = ArpHeader::new(
        hardware_type::ETHERNET,
        protocol_type::IPV4,
        opcode::REQUEST,
    );
    
    // Create ARP packet
    let packet = ArpPacket::new(
        header,
        interface.mac_address,
        interface.ip_address,
        [0, 0, 0, 0, 0, 0], // Target MAC is unknown
        target_ip,
    );
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Create Ethernet frame
    let eth_header = crate::network::ethernet::EthernetHeader::new(
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Broadcast MAC
        interface.mac_address,
        crate::network::ethernet::ethertype::ARP,
    );
    
    let frame = crate::network::ethernet::EthernetFrame::new(
        eth_header,
        bytes.to_vec(),
    );
    
    // Send the frame
    crate::network::ethernet::send_frame(interface_id, &frame)?;
    
    Ok(())
}
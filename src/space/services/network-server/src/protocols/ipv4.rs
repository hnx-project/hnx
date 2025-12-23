//! IPv4 protocol implementation for user-space network server

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;

/// IPv4 packet header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv4Header {
    pub version_ihl: u8,        // Version (4 bits) + Internet Header Length (4 bits)
    pub type_of_service: u8,    // Type of Service
    pub total_length: u16,      // Total Length
    pub identification: u16,    // Identification
    pub flags_fragment: u16,    // Flags (3 bits) + Fragment Offset (13 bits)
    pub ttl: u8,                // Time to Live
    pub protocol: u8,           // Protocol
    pub header_checksum: u16,   // Header Checksum
    pub source_ip: u32,         // Source Address
    pub destination_ip: u32,    // Destination Address
}

impl Ipv4Header {
    pub const MIN_SIZE: usize = 20;
    
    /// Create a new IPv4 header
    pub fn new(source_ip: u32, destination_ip: u32, protocol: u8, payload_length: usize) -> Self {
        let total_length = (Self::MIN_SIZE + payload_length) as u16;
        
        Ipv4Header {
            version_ihl: 0x45, // Version 4, IHL 5 (20 bytes)
            type_of_service: 0,
            total_length: total_length.to_be(),
            identification: 0, // In a real implementation, this would be incremented
            flags_fragment: 0x4000, // Don't fragment flag
            ttl: 64,
            protocol,
            header_checksum: 0, // Will be calculated later
            source_ip: source_ip.to_be(),
            destination_ip: destination_ip.to_be(),
        }
    }
    
    /// Parse an IPv4 header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::MIN_SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        Ok(Ipv4Header {
            version_ihl: data[0],
            type_of_service: data[1],
            total_length: u16::from_be_bytes([data[2], data[3]]),
            identification: u16::from_be_bytes([data[4], data[5]]),
            flags_fragment: u16::from_be_bytes([data[6], data[7]]),
            ttl: data[8],
            protocol: data[9],
            header_checksum: u16::from_be_bytes([data[10], data[11]]),
            source_ip: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            destination_ip: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        })
    }
    
    /// Convert IPv4 header to bytes
    pub fn to_bytes(&self) -> [u8; Self::MIN_SIZE] {
        let mut bytes = [0u8; Self::MIN_SIZE];
        
        bytes[0] = self.version_ihl;
        bytes[1] = self.type_of_service;
        bytes[2..4].copy_from_slice(&self.total_length.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.identification.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.flags_fragment.to_be_bytes());
        bytes[8] = self.ttl;
        bytes[9] = self.protocol;
        bytes[10..12].copy_from_slice(&self.header_checksum.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.source_ip.to_be_bytes());
        bytes[16..20].copy_from_slice(&self.destination_ip.to_be_bytes());
        
        bytes
    }
    
    /// Get the header length in bytes
    pub fn header_length(&self) -> usize {
        ((self.version_ihl & 0x0F) as usize) * 4
    }
    
    /// Get the total length in bytes
    pub fn total_length(&self) -> usize {
        u16::from_be(self.total_length) as usize
    }
    
    /// Calculate header checksum
    pub fn calculate_checksum(&self) -> u16 {
        let bytes = self.to_bytes();
        let mut sum: u32 = 0;
        
        // Sum all 16-bit words
        for i in 0..10 {
            let word = u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
            sum += word as u32;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        !sum as u16
    }
    
    /// Verify header checksum
    pub fn verify_checksum(&self) -> bool {
        self.calculate_checksum() == 0
    }
}

/// IPv4 routing table entry
#[derive(Debug, Clone)]
pub struct RoutingTableEntry {
    pub destination: u32,
    pub netmask: u32,
    pub gateway: u32,
    pub interface_id: u32,
}

/// IPv4 packet
#[derive(Debug, Clone)]
pub struct Ipv4Packet {
    pub header: Ipv4Header,
    pub payload: alloc::vec::Vec<u8>,
}

impl Ipv4Packet {
    /// Create a new IPv4 packet
    pub fn new(header: Ipv4Header, payload: alloc::vec::Vec<u8>) -> Self {
        Ipv4Packet {
            header,
            payload,
        }
    }
    
    /// Parse an IPv4 packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Ipv4Header::MIN_SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = Ipv4Header::from_bytes(data)?;
        let header_length = header.header_length();
        
        if data.len() < header.total_length() as usize {
            return Err(NetworkError::InvalidParameter);
        }
        
        let payload = data[header_length..header.total_length() as usize].to_vec();
        
        Ok(Ipv4Packet {
            header,
            payload,
        })
    }
    
    /// Convert IPv4 packet to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::with_capacity(Ipv4Header::MIN_SIZE + self.payload.len());
        
        // Calculate and set checksum
        let mut header = self.header;
        header.header_checksum = header.calculate_checksum();
        
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
}

/// IPv4 protocol numbers
pub mod protocol {
    pub const ICMP: u8 = 1;
    pub const TCP: u8 = 6;
    pub const UDP: u8 = 17;
}

/// Global routing table
static ROUTING_TABLE: Mutex<BTreeMap<u32, RoutingTableEntry>> = Mutex::new(BTreeMap::new());

/// Initialize the IPv4 subsystem
pub fn init() {
    crate::println!("network/ipv4: initializing");
    
    // Add default route for loopback interface (if it exists)
    // This is just an example - in a real implementation, routes would be added
    // when interfaces are configured
}

/// Add a route to the routing table
pub fn add_route(destination: u32, netmask: u32, gateway: u32, interface_id: u32) -> Result<()> {
    let mut routing_table = ROUTING_TABLE.lock();
    
    let entry = RoutingTableEntry {
        destination,
        netmask,
        gateway,
        interface_id,
    };
    
    routing_table.insert(destination, entry);
    
    crate::println!("network/ipv4: added route to {:08X}/{:08X} via {:08X} on interface {}", 
                 destination, netmask, gateway, interface_id);
    
    Ok(())
}

/// Remove a route from the routing table
pub fn remove_route(destination: u32) -> Result<()> {
    let mut routing_table = ROUTING_TABLE.lock();
    
    if routing_table.remove(&destination).is_some() {
        crate::println!("network/ipv4: removed route to {:08X}", destination);
        Ok(())
    } else {
        Err(NetworkError::AddressNotAvailable)
    }
}

/// Find the best route for a destination IP
pub fn find_route(destination: u32) -> Option<RoutingTableEntry> {
    let routing_table = ROUTING_TABLE.lock();
    
    // In a real implementation, we would use longest prefix matching
    // For simplicity, we'll just look for an exact match or a default route (0.0.0.0)
    
    // First check for exact match
    if let Some(entry) = routing_table.get(&destination) {
        return Some(entry.clone());
    }
    
    // Then check for default route
    if let Some(entry) = routing_table.get(&0) {
        return Some(entry.clone());
    }
    
    None
}

/// Process an incoming IPv4 packet
pub fn process_packet(data: &[u8]) -> Result<()> {
    let packet = Ipv4Packet::from_bytes(data)?;
    
    // Verify checksum
    if !packet.header.verify_checksum() {
        crate::println!("network/ipv4: invalid checksum");
        return Err(NetworkError::InvalidParameter);
    }
    
    // Check if packet is for us
    // In a real implementation, we would check against our IP addresses
    
    crate::println!("network/ipv4: received packet from {:08X} to {:08X}, protocol {}", 
                 u32::from_be(packet.header.source_ip),
                 u32::from_be(packet.header.destination_ip),
                 packet.header.protocol);
    
    // Process packet through firewall (only if firewall is enabled)
    // Note: In user-space implementation, firewall would be handled differently
    // For now, we'll just log that we would process it
    
    crate::println!("network/ipv4: would process packet through firewall");
    
    // Classify packet with QoS (only if QoS is enabled)
    // Note: In user-space implementation, QoS would be handled differently
    // For now, we'll just log that we would process it
    
    crate::println!("network/ipv4: would classify packet with QoS");
    
    match packet.header.protocol {
        protocol::ICMP => {
            crate::println!("network/ipv4: would process ICMP packet");
        }
        protocol::UDP => {
            crate::println!("network/ipv4: would process UDP packet");
        }
        protocol::TCP => {
            crate::println!("network/ipv4: would process TCP packet");
        }
        _ => {
            crate::println!("network/ipv4: unsupported protocol {}", packet.header.protocol);
            return Err(NetworkError::NotSupported);
        }
    }
    
    Ok(())
}

/// Send an IPv4 packet
pub fn send_packet(interface_id: u32, destination_ip: u32, protocol: u8, 
                   payload: alloc::vec::Vec<u8>) -> Result<()> {
    // In a real implementation, we would get the network interface
    // For now, we'll just log that we would do this
    
    crate::println!("network/ipv4: would get network interface {}", interface_id);
    
    // Find route for destination
    let route = match find_route(destination_ip) {
        Some(route) => route,
        None => {
            // If no route found, assume direct delivery on the same interface
            RoutingTableEntry {
                destination: destination_ip,
                netmask: 0,
                gateway: 0,
                interface_id,
            }
        }
    };
    
    // Determine actual destination IP (use gateway if indirect route)
    let actual_destination = if route.gateway != 0 {
        route.gateway
    } else {
        destination_ip
    };
    
    // Create IPv4 header
    let header = Ipv4Header::new(
        0, // source IP would come from interface
        actual_destination,
        protocol,
        payload.len(),
    );
    
    // Process packet through firewall (only if firewall is enabled)
    // Note: In user-space implementation, firewall would be handled differently
    // For now, we'll just log that we would process it
    
    crate::println!("network/ipv4: would process outgoing packet through firewall");
    
    // Classify outgoing packet with QoS (only if QoS is enabled)
    // Note: In user-space implementation, QoS would be handled differently
    // For now, we'll just log that we would process it
    
    crate::println!("network/ipv4: would classify outgoing packet with QoS");
    
    // Create IPv4 packet
    let packet = Ipv4Packet::new(header, payload);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Look up destination MAC address in ARP cache
    // Note: In user-space implementation, ARP would be handled differently
    // For now, we'll just log that we would do this
    
    crate::println!("network/ipv4: would lookup destination MAC address in ARP cache");
    
    // Create Ethernet frame
    // Note: In user-space implementation, Ethernet would be handled differently
    // For now, we'll just log that we would do this
    
    crate::println!("network/ipv4: would create Ethernet frame");
    
    // Send the frame
    // Note: In user-space implementation, we would send this via IPC to kernel
    // For now, we'll just log that we would do this
    
    crate::println!("network/ipv4: would send frame via IPC to kernel");
    
    Ok(())
}
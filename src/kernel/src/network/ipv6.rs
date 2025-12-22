//! IPv6 protocol implementation
//!
//! This module implements the IPv6 protocol for the HNX kernel's network stack.
//! It includes support for IPv6 headers, packets, routing, and basic functionality.
//! IPv6 is the next-generation Internet Protocol designed to replace IPv4.

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IPv6 packet header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv6Header {
    pub version_traffic_class_flow_label: u32, // Version (4 bits) + Traffic Class (8 bits) + Flow Label (20 bits)
    pub payload_length: u16,                   // Payload Length
    pub next_header: u8,                       // Next Header
    pub hop_limit: u8,                         // Hop Limit
    pub source_addr: [u8; 16],                 // Source Address
    pub destination_addr: [u8; 16],            // Destination Address
}

impl Ipv6Header {
    pub const SIZE: usize = 40;
    
    /// Create a new IPv6 header
    pub fn new(source_addr: [u8; 16], destination_addr: [u8; 16], next_header: u8, payload_length: usize) -> Self {
        Ipv6Header {
            version_traffic_class_flow_label: 0x60000000, // Version 6, Traffic Class 0, Flow Label 0
            payload_length: payload_length as u16,
            next_header,
            hop_limit: 64,
            source_addr,
            destination_addr,
        }
    }
    
    /// Parse an IPv6 header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let version_traffic_class_flow_label = u32::from_be_bytes([
            data[0], data[1], data[2], data[3]
        ]);
        
        Ok(Ipv6Header {
            version_traffic_class_flow_label,
            payload_length: u16::from_be_bytes([data[4], data[5]]),
            next_header: data[6],
            hop_limit: data[7],
            source_addr: [
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15],
                data[16], data[17], data[18], data[19],
                data[20], data[21], data[22], data[23],
            ],
            destination_addr: [
                data[24], data[25], data[26], data[27],
                data[28], data[29], data[30], data[31],
                data[32], data[33], data[34], data[35],
                data[36], data[37], data[38], data[39],
            ],
        })
    }
    
    /// Convert IPv6 header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        let version_bytes = self.version_traffic_class_flow_label.to_be_bytes();
        bytes[0..4].copy_from_slice(&version_bytes);
        bytes[4..6].copy_from_slice(&self.payload_length.to_be_bytes());
        bytes[6] = self.next_header;
        bytes[7] = self.hop_limit;
        bytes[8..24].copy_from_slice(&self.source_addr);
        bytes[24..40].copy_from_slice(&self.destination_addr);
        
        bytes
    }
    
    /// Get the version from the header
    pub fn version(&self) -> u8 {
        ((self.version_traffic_class_flow_label >> 28) & 0xF) as u8
    }
    
    /// Get the traffic class from the header
    pub fn traffic_class(&self) -> u8 {
        ((self.version_traffic_class_flow_label >> 20) & 0xFF) as u8
    }
    
    /// Get the flow label from the header
    pub fn flow_label(&self) -> u32 {
        self.version_traffic_class_flow_label & 0xFFFFF
    }
    
    /// Get the payload length
    pub fn payload_length(&self) -> usize {
        self.payload_length as usize
    }
}

/// IPv6 routing table entry
#[derive(Debug, Clone)]
pub struct Ipv6RoutingTableEntry {
    pub destination: [u8; 16],
    pub prefix_length: u8,
    pub gateway: [u8; 16],
    pub interface_id: u32,
}

/// IPv6 packet
#[derive(Debug, Clone)]
pub struct Ipv6Packet {
    pub header: Ipv6Header,
    pub payload: Vec<u8>,
}

impl Ipv6Packet {
    /// Create a new IPv6 packet
    pub fn new(header: Ipv6Header, payload: Vec<u8>) -> Self {
        Ipv6Packet {
            header,
            payload,
        }
    }
    
    /// Parse an IPv6 packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Ipv6Header::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = Ipv6Header::from_bytes(data)?;
        
        if data.len() < Ipv6Header::SIZE + header.payload_length() {
            return Err(NetworkError::InvalidParameter);
        }
        
        let payload = data[Ipv6Header::SIZE..Ipv6Header::SIZE + header.payload_length()].to_vec();
        
        Ok(Ipv6Packet {
            header,
            payload,
        })
    }
    
    /// Convert IPv6 packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Ipv6Header::SIZE + self.payload.len());
        
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
}

/// IPv6 next header values
pub mod next_header {
    pub const ICMPV6: u8 = 58;
    pub const TCP: u8 = 6;
    pub const UDP: u8 = 17;
}

/// Global IPv6 routing table
static IPV6_ROUTING_TABLE: Mutex<BTreeMap<[u8; 16], Ipv6RoutingTableEntry>> = Mutex::new(BTreeMap::new());

/// Initialize the IPv6 subsystem
pub fn init() {
    crate::info!("network/ipv6: initializing");
    
    // Add default route for loopback interface (if it exists)
    // This is just an example - in a real implementation, routes would be added
    // when interfaces are configured
}

/// Add a route to the IPv6 routing table
pub fn add_route(destination: [u8; 16], prefix_length: u8, gateway: [u8; 16], interface_id: u32) -> Result<()> {
    let mut routing_table = IPV6_ROUTING_TABLE.lock();
    
    let entry = Ipv6RoutingTableEntry {
        destination,
        prefix_length,
        gateway,
        interface_id,
    };
    
    routing_table.insert(destination, entry);
    
    crate::info!("network/ipv6: added route to {:?}/{} via {:?} on interface {}", 
                 destination, prefix_length, gateway, interface_id);
    
    Ok(())
}

/// Remove a route from the IPv6 routing table
pub fn remove_route(destination: [u8; 16]) -> Result<()> {
    let mut routing_table = IPV6_ROUTING_TABLE.lock();
    
    if routing_table.remove(&destination).is_some() {
        crate::info!("network/ipv6: removed route to {:?}", destination);
        Ok(())
    } else {
        Err(NetworkError::AddressNotAvailable)
    }
}

/// Find the best route for a destination IPv6 address
pub fn find_route(destination: [u8; 16]) -> Option<Ipv6RoutingTableEntry> {
    let routing_table = IPV6_ROUTING_TABLE.lock();
    
    // In a real implementation, we would use longest prefix matching
    // For simplicity, we'll just look for an exact match
    
    if let Some(entry) = routing_table.get(&destination) {
        return Some(entry.clone());
    }
    
    // Check for default route (::/0)
    let default_route: [u8; 16] = [0; 16];
    if let Some(entry) = routing_table.get(&default_route) {
        return Some(entry.clone());
    }
    
    None
}

/// Process an incoming IPv6 packet
pub fn process_packet(data: &[u8]) -> Result<()> {
    let packet = Ipv6Packet::from_bytes(data)?;
    
    // Verify version
    if packet.header.version() != 6 {
        crate::info!("network/ipv6: invalid version");
        return Err(NetworkError::InvalidParameter);
    }
    
    crate::info!("network/ipv6: received packet from {:?} to {:?}, next header {}", 
                 packet.header.source_addr,
                 packet.header.destination_addr,
                 packet.header.next_header);
    
    // Process packet through firewall (only if firewall is enabled)
    if crate::network::firewall::is_enabled() {
        let firewall_action = crate::network::firewall_enhanced::process_incoming_packet_v6(
            &packet.header,
            Some(&packet.payload),
        )?;
        
        // Handle firewall action
        match firewall_action {
            crate::network::firewall::FirewallAction::Drop => {
                crate::info!("network/ipv6: packet dropped by firewall");
                return Ok(()); // Silently drop the packet
            }
            crate::network::firewall::FirewallAction::Reject => {
                crate::info!("network/ipv6: packet rejected by firewall");
                // In a full implementation, we would send back an ICMPv6 reject message
                return Ok(());
            }
            crate::network::firewall::FirewallAction::Allow => {
                // Packet is allowed, continue processing
            }
        }
    }
    
    // Classify packet with QoS (only if QoS is enabled)
    if crate::network::qos::is_enabled() {
        if let Some(qos_action) = crate::network::qos_enhanced::classify_packet_v6(
            &packet.header,
            Some(&packet.payload),
            packet.payload.len(),
        ) {
            match qos_action {
                crate::network::qos::QosAction::Drop => {
                    crate::info!("network/ipv6: packet dropped by QoS");
                    return Ok(()); // Silently drop the packet
                }
                crate::network::qos::QosAction::Allow { priority } => {
                    crate::info!("network/ipv6: packet allowed by QoS with priority {}", priority);
                    // Continue processing with assigned priority
                }
                crate::network::qos::QosAction::Shape { .. } => {
                    crate::info!("network/ipv6: packet shaped by QoS");
                    // Continue processing (shaping handled in classify_packet_v6)
                }
                crate::network::qos::QosAction::Police { .. } => {
                    crate::info!("network/ipv6: packet policed by QoS");
                    // Continue processing (policing handled in classify_packet_v6)
                }
            }
        }
    }
    
    match packet.header.next_header {
        next_header::ICMPV6 => {
            // For now, we'll use the existing ICMP process_packet function
            // In a full implementation, we would have a separate process_packet_v6 function
            crate::network::icmp::process_packet(&packet.payload)?;
        }
        next_header::UDP => {
            // For now, we'll use the existing UDP process_packet function
            // In a full implementation, we would have a separate process_packet_v6 function
            crate::network::udp::process_packet(0, u32::from_be_bytes([
                packet.header.source_addr[12],
                packet.header.source_addr[13],
                packet.header.source_addr[14],
                packet.header.source_addr[15]
            ]), &packet.payload)?;
        }
        next_header::TCP => {
            // For now, we'll use the existing TCP process_packet function
            // In a full implementation, we would have a separate process_packet_v6 function
            crate::network::tcp::process_packet(0, u32::from_be_bytes([
                packet.header.source_addr[12],
                packet.header.source_addr[13],
                packet.header.source_addr[14],
                packet.header.source_addr[15]
            ]), &packet.payload)?;
        }
        _ => {
            crate::info!("network/ipv6: unsupported next header {}", packet.header.next_header);
            return Err(NetworkError::NotSupported);
        }
    }
    
    Ok(())
}

/// Send an IPv6 packet
pub fn send_packet(interface_id: u32, destination_addr: [u8; 16], next_header: u8, 
                   payload: Vec<u8>) -> Result<()> {
    // Get the network interface
    let interface = match crate::network::interface::get_interface(interface_id) {
        Some(iface) => iface,
        None => return Err(NetworkError::InterfaceNotFound),
    };
    
    // Find route for destination
    let route = match find_route(destination_addr) {
        Some(route) => route,
        None => {
            // If no route found, assume direct delivery on the same interface
            Ipv6RoutingTableEntry {
                destination: destination_addr,
                prefix_length: 0,
                gateway: [0; 16],
                interface_id,
            }
        }
    };
    
    // Determine actual destination address (use gateway if indirect route)
    let actual_destination = if route.gateway != [0; 16] {
        route.gateway
    } else {
        destination_addr
    };
    
    // Create IPv6 header
    // Convert IPv4 address to IPv6 mapped address for now
    let mut ipv6_source_addr = [0u8; 16];
    ipv6_source_addr[10] = 0xFF;
    ipv6_source_addr[11] = 0xFF;
    let ipv4_bytes = interface.ip_address.to_be_bytes();
    ipv6_source_addr[12] = ipv4_bytes[0];
    ipv6_source_addr[13] = ipv4_bytes[1];
    ipv6_source_addr[14] = ipv4_bytes[2];
    ipv6_source_addr[15] = ipv4_bytes[3];
    
    let header = Ipv6Header::new(
        ipv6_source_addr,
        actual_destination,
        next_header,
        payload.len(),
    );
    
    // Process packet through firewall (only if firewall is enabled)
    if crate::network::firewall::is_enabled() {
        let firewall_action = crate::network::firewall_enhanced::process_outgoing_packet_v6(
            &header,
            Some(&payload),
        )?;
        
        // Handle firewall action
        match firewall_action {
            crate::network::firewall::FirewallAction::Drop => {
                crate::info!("network/ipv6: outgoing packet dropped by firewall");
                return Ok(()); // Silently drop the packet
            }
            crate::network::firewall::FirewallAction::Reject => {
                crate::info!("network/ipv6: outgoing packet rejected by firewall");
                // In a full implementation, we might handle this differently
                return Ok(());
            }
            crate::network::firewall::FirewallAction::Allow => {
                // Packet is allowed, continue sending
            }
        }
    }
    
    // Classify outgoing packet with QoS (only if QoS is enabled)
    if crate::network::qos::is_enabled() {
        if let Some(qos_action) = crate::network::qos_enhanced::classify_packet_v6(
            &header,
            Some(&payload),
            payload.len(),
        ) {
            match qos_action {
                crate::network::qos::QosAction::Drop => {
                    crate::info!("network/ipv6: outgoing packet dropped by QoS");
                    return Ok(()); // Silently drop the packet
                }
                crate::network::qos::QosAction::Allow { priority } => {
                    crate::info!("network/ipv6: outgoing packet allowed by QoS with priority {}", priority);
                    // Continue processing with assigned priority
                }
                crate::network::qos::QosAction::Shape { .. } => {
                    crate::info!("network/ipv6: outgoing packet shaped by QoS");
                    // Continue processing (shaping handled in classify_packet_v6)
                }
                crate::network::qos::QosAction::Police { .. } => {
                    crate::info!("network/ipv6: outgoing packet policed by QoS");
                    // Continue processing (policing handled in classify_packet_v6)
                }
            }
        }
    }
    
    // Create IPv6 packet
    let packet = Ipv6Packet::new(header, payload);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // For IPv6, we would typically use Neighbor Discovery Protocol (NDP) instead of ARP
    // For simplicity, we'll assume we can send directly
    let dest_mac = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // Broadcast for now
    
    // Create Ethernet frame
    let eth_header = crate::network::ethernet::EthernetHeader::new(
        dest_mac,
        interface.mac_address,
        crate::network::ethernet::ethertype::IPV6,
    );
    
    let frame = crate::network::ethernet::EthernetFrame::new(
        eth_header,
        bytes,
    );
    
    // Send the frame
    crate::network::ethernet::send_frame(route.interface_id, &frame)?;
    
    Ok(())
}
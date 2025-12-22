//! Ethernet protocol implementation

extern crate alloc;

use crate::network::{NetworkError, Result};

/// Ethernet frame header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EthernetHeader {
    pub destination: [u8; 6],
    pub source: [u8; 6],
    pub ethertype: u16,
}

impl EthernetHeader {
    pub const SIZE: usize = 14;
    
    /// Create a new Ethernet header
    pub fn new(destination: [u8; 6], source: [u8; 6], ethertype: u16) -> Self {
        EthernetHeader {
            destination,
            source,
            ethertype: ethertype.to_be(),
        }
    }
    
    /// Parse an Ethernet header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let mut destination = [0u8; 6];
        let mut source = [0u8; 6];
        
        destination.copy_from_slice(&data[0..6]);
        source.copy_from_slice(&data[6..12]);
        
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        
        Ok(EthernetHeader {
            destination,
            source,
            ethertype,
        })
    }
    
    /// Convert Ethernet header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0..6].copy_from_slice(&self.destination);
        bytes[6..12].copy_from_slice(&self.source);
        bytes[12..14].copy_from_slice(&self.ethertype.to_be_bytes());
        
        bytes
    }
}

/// Ethernet frame
#[derive(Debug, Clone)]
pub struct EthernetFrame {
    pub header: EthernetHeader,
    pub payload: alloc::vec::Vec<u8>,
}

impl EthernetFrame {
    /// Create a new Ethernet frame
    pub fn new(header: EthernetHeader, payload: alloc::vec::Vec<u8>) -> Self {
        EthernetFrame {
            header,
            payload,
        }
    }
    
    /// Parse an Ethernet frame from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < EthernetHeader::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = EthernetHeader::from_bytes(data)?;
        let payload = data[EthernetHeader::SIZE..].to_vec();
        
        Ok(EthernetFrame {
            header,
            payload,
        })
    }
    
    /// Convert Ethernet frame to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::with_capacity(EthernetHeader::SIZE + self.payload.len());
        
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
}

/// Ethernet protocol types
pub mod ethertype {
    pub const IPV4: u16 = 0x0800;
    pub const ARP: u16 = 0x0806;
    pub const IPV6: u16 = 0x86DD;
}

/// Initialize the Ethernet subsystem
pub fn init() {
    crate::info!("network/ethernet: initializing");
}

/// Process an incoming Ethernet frame
pub fn process_frame(interface_id: u32, frame: &EthernetFrame) -> Result<()> {
    // Check if the frame is for us
    // This would normally check against our MAC addresses
    
    let ethertype = u16::from_be(frame.header.ethertype);
    
    // Update RX statistics
    let frame_size = EthernetHeader::SIZE + frame.payload.len();
    crate::network::interface::update_rx_stats(interface_id, frame_size as u64, 0)?;
    
    match ethertype {
        ethertype::IPV4 => {
            // Pass to IPv4 handler
            crate::network::ipv4::process_packet(&frame.payload)?;
        }
        ethertype::ARP => {
            // Pass to ARP handler
            crate::network::arp::process_packet(&frame.payload)?;
        }
        _ => {
            crate::info!("network/ethernet: unsupported ethertype 0x{:04X}", 
                         ethertype);
            // Update error statistics
            crate::network::interface::update_rx_stats(interface_id, 0, 1)?;
            return Err(NetworkError::NotSupported);
        }
    }
    
    Ok(())
}

/// Send an Ethernet frame
pub fn send_frame(interface_id: u32, frame: &EthernetFrame) -> Result<()> {
    // Get the network interface
    let interface = match crate::network::interface::get_interface(interface_id) {
        Some(iface) => iface,
        None => return Err(NetworkError::InterfaceNotFound),
    };
    
    // Convert frame to bytes
    let bytes = frame.to_bytes();
    
    // In a real implementation, this would call the network driver
    // to send the frame over the wire
    
    crate::info!("network/ethernet: sending frame of {} bytes on interface {}", 
                 bytes.len(), interface.name);
    
    // Update TX statistics
    crate::network::interface::update_tx_stats(interface_id, bytes.len() as u64, 0)?;
    
    Ok(())
}
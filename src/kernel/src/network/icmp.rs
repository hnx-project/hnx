//! ICMP (Internet Control Message Protocol) implementation

extern crate alloc;

use crate::network::{NetworkError, Result};

/// ICMP packet types
pub mod icmp_type {
    pub const ECHO_REPLY: u8 = 0;
    pub const DESTINATION_UNREACHABLE: u8 = 3;
    pub const SOURCE_QUENCH: u8 = 4;
    pub const REDIRECT: u8 = 5;
    pub const ECHO_REQUEST: u8 = 8;
    pub const TIME_EXCEEDED: u8 = 11;
    pub const PARAMETER_PROBLEM: u8 = 12;
    pub const TIMESTAMP_REQUEST: u8 = 13;
    pub const TIMESTAMP_REPLY: u8 = 14;
}

/// ICMP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpHeader {
    pub type_: u8,
    pub code: u8,
    pub checksum: u16,
}

impl IcmpHeader {
    pub const SIZE: usize = 4;
    
    /// Create a new ICMP header
    pub fn new(type_: u8, code: u8) -> Self {
        IcmpHeader {
            type_,
            code,
            checksum: 0, // Will be calculated later
        }
    }
    
    /// Parse an ICMP header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        Ok(IcmpHeader {
            type_: data[0],
            code: data[1],
            checksum: u16::from_be_bytes([data[2], data[3]]),
        })
    }
    
    /// Convert ICMP header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0] = self.type_;
        bytes[1] = self.code;
        bytes[2..4].copy_from_slice(&self.checksum.to_be_bytes());
        
        bytes
    }
    
    /// Calculate checksum
    pub fn calculate_checksum(&self, data: &[u8]) -> u16 {
        let header_bytes = self.to_bytes();
        let mut sum: u32 = 0;
        
        // Sum header
        for i in 0..2 {
            let word = u16::from_be_bytes([header_bytes[i * 2], header_bytes[i * 2 + 1]]);
            sum += word as u32;
        }
        
        // Sum data
        let mut i = 0;
        while i < data.len() {
            let word = if i + 1 < data.len() {
                u16::from_be_bytes([data[i], data[i + 1]])
            } else {
                u16::from_be_bytes([data[i], 0])
            };
            sum += word as u32;
            i += 2;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        !sum as u16
    }
}

/// ICMP Echo packet (Ping)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpEcho {
    pub header: IcmpHeader,
    pub identifier: u16,
    pub sequence_number: u16,
}

impl IcmpEcho {
    pub const HEADER_SIZE: usize = 8;
    
    /// Create a new ICMP Echo packet
    pub fn new(type_: u8, identifier: u16, sequence_number: u16) -> Self {
        IcmpEcho {
            header: IcmpHeader::new(type_, 0),
            identifier: identifier.to_be(),
            sequence_number: sequence_number.to_be(),
        }
    }
    
    /// Parse an ICMP Echo packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::HEADER_SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = IcmpHeader::from_bytes(data)?;
        
        Ok(IcmpEcho {
            header,
            identifier: u16::from_be_bytes([data[4], data[5]]),
            sequence_number: u16::from_be_bytes([data[6], data[7]]),
        })
    }
    
    /// Convert ICMP Echo packet to bytes
    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let mut bytes = [0u8; Self::HEADER_SIZE];
        
        let header_bytes = self.header.to_bytes();
        bytes[0..4].copy_from_slice(&header_bytes);
        bytes[4..6].copy_from_slice(&self.identifier.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.sequence_number.to_be_bytes());
        
        bytes
    }
}

/// Initialize the ICMP subsystem
pub fn init() {
    crate::info!("network/icmp: initializing");
}

/// Process an incoming ICMP packet
pub fn process_packet(data: &[u8]) -> Result<()> {
    if data.len() < IcmpHeader::SIZE {
        return Err(NetworkError::InvalidParameter);
    }
    
    let header = IcmpHeader::from_bytes(data)?;
    
    crate::info!("network/icmp: received packet type {}, code {}", 
                 header.type_, header.code);
    
    match header.type_ {
        icmp_type::ECHO_REQUEST => {
            handle_echo_request(data)?;
        }
        icmp_type::ECHO_REPLY => {
            handle_echo_reply(data)?;
        }
        _ => {
            crate::info!("network/icmp: unsupported type {}", header.type_);
            return Err(NetworkError::NotSupported);
        }
    }
    
    Ok(())
}

/// Handle ICMP Echo Request (Ping)
fn handle_echo_request(data: &[u8]) -> Result<()> {
    crate::info!("network/icmp: received ping request");
    
    if data.len() < IcmpEcho::HEADER_SIZE {
        return Err(NetworkError::InvalidParameter);
    }
    
    let echo = IcmpEcho::from_bytes(data)?;
    
    crate::info!("network/icmp: ping from id {}, seq {}", 
                 u16::from_be(echo.identifier), 
                 u16::from_be(echo.sequence_number));
    
    // Send echo reply
    let reply = IcmpEcho {
        header: IcmpHeader::new(icmp_type::ECHO_REPLY, 0),
        identifier: echo.identifier,
        sequence_number: echo.sequence_number,
    };
    
    // Create reply packet
    let mut reply_bytes = reply.to_bytes().to_vec();
    
    // Add any payload data from the original request
    if data.len() > IcmpEcho::HEADER_SIZE {
        reply_bytes.extend_from_slice(&data[IcmpEcho::HEADER_SIZE..]);
    }
    
    // Calculate checksum
    let checksum = reply.header.calculate_checksum(&reply_bytes[IcmpHeader::SIZE..]);
    let checksum_bytes = checksum.to_be_bytes();
    reply_bytes[2] = checksum_bytes[0];
    reply_bytes[3] = checksum_bytes[1];
    
    // Send the reply
    // In a real implementation, we would send this back to the sender
    // For now, we'll just log that we would send it
    
    crate::info!("network/icmp: sending ping reply");
    
    Ok(())
}

/// Handle ICMP Echo Reply (Ping response)
fn handle_echo_reply(data: &[u8]) -> Result<()> {
    crate::info!("network/icmp: received ping reply");
    
    if data.len() < IcmpEcho::HEADER_SIZE {
        return Err(NetworkError::InvalidParameter);
    }
    
    let echo = IcmpEcho::from_bytes(data)?;
    
    crate::info!("network/icmp: pong from id {}, seq {}", 
                 u16::from_be(echo.identifier), 
                 u16::from_be(echo.sequence_number));
    
    Ok(())
}

/// Send an ICMP Echo Request (Ping)
pub fn send_ping(interface_id: u32, destination_ip: u32, 
                 identifier: u16, sequence_number: u16) -> Result<()> {
    // Create ICMP Echo Request
    let mut echo = IcmpEcho::new(icmp_type::ECHO_REQUEST, identifier, sequence_number);
    
    // Convert to bytes (without checksum for now)
    let echo_bytes = echo.to_bytes();
    
    // Calculate checksum
    echo.header.checksum = echo.header.calculate_checksum(&[]);
    
    // Send the packet via IPv4
    let payload = echo_bytes.to_vec();
    crate::network::ipv4::send_packet(interface_id, destination_ip, 
                                      crate::network::ipv4::protocol::ICMP, 
                                      payload)?;
    
    Ok(())
}
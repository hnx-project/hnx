//! UDP (User Datagram Protocol) implementation for user-space network server

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;

/// UDP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
}

/// UDP socket structure
#[derive(Debug, Clone)]
pub struct UdpSocket {
    pub local_port: u16,
    pub remote_addr: Option<(u32, u16)>, // (IP, port)
    pub rx_queue: VecDeque<(u32, u16, alloc::vec::Vec<u8>)>, // (source_ip, source_port, data)
}

impl UdpHeader {
    pub const SIZE: usize = 8;
    
    /// Create a new UDP header
    pub fn new(source_port: u16, destination_port: u16, payload_length: usize) -> Self {
        let length = (Self::SIZE + payload_length) as u16;
        
        UdpHeader {
            source_port: source_port.to_be(),
            destination_port: destination_port.to_be(),
            length: length.to_be(),
            checksum: 0, // In many implementations, UDP checksum is optional
        }
    }
    
    /// Parse a UDP header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        Ok(UdpHeader {
            source_port: u16::from_be_bytes([data[0], data[1]]),
            destination_port: u16::from_be_bytes([data[2], data[3]]),
            length: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        })
    }
    
    /// Convert UDP header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0..2].copy_from_slice(&self.source_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.destination_port.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.length.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        
        bytes
    }
    
    /// Get the total length in bytes
    pub fn total_length(&self) -> usize {
        self.length as usize
    }
}

/// UDP packet
#[derive(Debug, Clone)]
pub struct UdpPacket {
    pub header: UdpHeader,
    pub payload: alloc::vec::Vec<u8>,
}

impl UdpPacket {
    /// Create a new UDP packet
    pub fn new(header: UdpHeader, payload: alloc::vec::Vec<u8>) -> Self {
        UdpPacket {
            header,
            payload,
        }
    }
    
    /// Parse a UDP packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < UdpHeader::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = UdpHeader::from_bytes(data)?;
        
        if data.len() < header.total_length() {
            return Err(NetworkError::InvalidParameter);
        }
        
        let payload = data[UdpHeader::SIZE..header.total_length()].to_vec();
        
        Ok(UdpPacket {
            header,
            payload,
        })
    }
    
    /// Convert UDP packet to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::with_capacity(UdpHeader::SIZE + self.payload.len());
        
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
}

/// Global UDP socket table
static UDP_SOCKETS: Mutex<BTreeMap<u16, UdpSocket>> = Mutex::new(BTreeMap::new());

/// Initialize the UDP subsystem
pub fn init() {
    crate::println!("network/udp: initializing");
}

/// Create a new UDP socket
pub fn create_socket(local_port: u16) -> Result<()> {
    let mut sockets = UDP_SOCKETS.lock();
    
    if sockets.contains_key(&local_port) {
        return Err(NetworkError::AddressInUse);
    }
    
    let socket = UdpSocket {
        local_port,
        remote_addr: None,
        rx_queue: VecDeque::new(),
    };
    
    sockets.insert(local_port, socket);
    Ok(())
}

/// Bind a UDP socket to a remote address
pub fn bind_socket(local_port: u16, remote_ip: u32, remote_port: u16) -> Result<()> {
    let mut sockets = UDP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    socket.remote_addr = Some((remote_ip, remote_port));
    Ok(())
}

/// Close a UDP socket
pub fn close_socket(local_port: u16) -> Result<()> {
    let mut sockets = UDP_SOCKETS.lock();
    
    if sockets.remove(&local_port).is_some() {
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// Process an incoming UDP packet
pub fn process_packet(interface_id: u32, src_ip: u32, data: &[u8]) -> Result<()> {
    let packet = UdpPacket::from_bytes(data)?;
    
    let src_port = u16::from_be(packet.header.source_port);
    let dst_port = u16::from_be(packet.header.destination_port);
    
    crate::println!("network/udp: received packet from port {} to port {}", 
                 src_port, dst_port);
    
    // Deliver the packet to the appropriate socket
    let mut sockets = UDP_SOCKETS.lock();
    
    if let Some(socket) = sockets.get_mut(&dst_port) {
        // Add packet to receive queue
        socket.rx_queue.push_back((src_ip, src_port, packet.payload));
    } else {
        crate::println!("network/udp: no socket bound to port {}", dst_port);
        return Err(NetworkError::AddressNotAvailable);
    }
    
    Ok(())
}

/// Send a UDP packet
pub fn send_packet(interface_id: u32, destination_ip: u32, 
                   source_port: u16, destination_port: u16,
                   payload: alloc::vec::Vec<u8>) -> Result<()> {
    // Create UDP header
    let header = UdpHeader::new(source_port, destination_port, payload.len());
    
    // Create UDP packet
    let packet = UdpPacket::new(header, payload);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/udp: would send packet to kernel for transmission");
    
    Ok(())
}

/// Receive data from a UDP socket
pub fn recv_from(local_port: u16, buffer: &mut [u8]) -> Result<(usize, u32, u16)> {
    let mut sockets = UDP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if there's data in the receive queue
    if let Some((src_ip, src_port, data)) = socket.rx_queue.pop_front() {
        let len = core::cmp::min(buffer.len(), data.len());
        buffer[..len].copy_from_slice(&data[..len]);
        Ok((len, src_ip, src_port))
    } else {
        // No data available
        Ok((0, 0, 0))
    }
}
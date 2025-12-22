//! Network IPC protocol definitions
//!
//! This module defines the standardized IPC messages for network operations
//! between user-space network server and kernel network manager.

extern crate alloc;

/// Network manager opcodes
pub const NET_OP_SOCKET: u64 = 1;
pub const NET_OP_BIND: u64 = 2;
pub const NET_OP_LISTEN: u64 = 3;
pub const NET_OP_ACCEPT: u64 = 4;
pub const NET_OP_CONNECT: u64 = 5;
pub const NET_OP_SEND: u64 = 6;
pub const NET_OP_RECV: u64 = 7;
pub const NET_OP_CLOSE: u64 = 8;
pub const NET_OP_PACKET_IN: u64 = 9;   // Kernel → Network Server
pub const NET_OP_PACKET_OUT: u64 = 10; // Network Server → Kernel

// Standard socket domains
pub const AF_INET: u32 = 1;
pub const AF_INET6: u32 = 2;

// Standard socket types
pub const SOCK_STREAM: u32 = 1;
pub const SOCK_DGRAM: u32 = 2;

// Standard protocols
pub const IPPROTO_IP: u32 = 0;
pub const IPPROTO_TCP: u32 = 6;
pub const IPPROTO_UDP: u32 = 17;
pub const IPPROTO_ICMP: u32 = 1;

/// Socket creation request
#[derive(Debug, Clone)]
pub struct SocketRequest {
    pub domain: u32,    // AF_INET, AF_INET6
    pub sock_type: u32, // SOCK_STREAM, SOCK_DGRAM
    pub protocol: u32,  // IPPROTO_TCP, IPPROTO_UDP
}

/// Socket creation response
#[derive(Debug, Clone)]
pub struct SocketResponse {
    pub sockfd: u32,    // Socket file descriptor
    pub error: u32,     // Error code (0 = success)
}

/// Bind request
#[derive(Debug, Clone)]
pub struct BindRequest {
    pub sockfd: u32,
    pub addr: [u8; 16], // IP address (IPv4 or IPv6)
    pub port: u16,
}

/// Listen request
#[derive(Debug, Clone)]
pub struct ListenRequest {
    pub sockfd: u32,
    pub backlog: u32,
}

/// Accept response
#[derive(Debug, Clone)]
pub struct AcceptResponse {
    pub new_sockfd: u32,
    pub addr: [u8; 16],
    pub port: u16,
    pub error: u32,
}

/// Connect request
#[derive(Debug, Clone)]
pub struct ConnectRequest {
    pub sockfd: u32,
    pub addr: [u8; 16],
    pub port: u16,
}

/// Send/Recv data transfer
#[derive(Debug, Clone)]
pub struct DataTransfer {
    pub sockfd: u32,
    pub flags: u32,
    // Data is sent in message payload
}

/// Packet forwarding structure
#[derive(Debug, Clone)]
pub struct PacketForward {
    pub interface_id: u32,
    pub packet_len: u32,
    // Packet data is in message payload
}

// Simple serialization functions for the protocol structures
impl SocketRequest {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.domain.to_le_bytes());
        data.extend_from_slice(&self.sock_type.to_le_bytes());
        data.extend_from_slice(&self.protocol.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        Some(Self {
            domain: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            sock_type: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            protocol: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
        })
    }
}

impl SocketResponse {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.sockfd.to_le_bytes());
        data.extend_from_slice(&self.error.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            sockfd: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            error: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        })
    }
}

impl BindRequest {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.sockfd.to_le_bytes());
        data.extend_from_slice(&self.addr);
        data.extend_from_slice(&self.port.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 22 {
            return None;
        }
        let mut addr = [0u8; 16];
        addr.copy_from_slice(&data[4..20]);
        Some(Self {
            sockfd: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            addr,
            port: u16::from_le_bytes([data[20], data[21]]),
        })
    }
}

impl ListenRequest {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.sockfd.to_le_bytes());
        data.extend_from_slice(&self.backlog.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            sockfd: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            backlog: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        })
    }
}

impl AcceptResponse {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.new_sockfd.to_le_bytes());
        data.extend_from_slice(&self.addr);
        data.extend_from_slice(&self.port.to_le_bytes());
        data.extend_from_slice(&self.error.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 26 {
            return None;
        }
        let mut addr = [0u8; 16];
        addr.copy_from_slice(&data[4..20]);
        Some(Self {
            new_sockfd: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            addr,
            port: u16::from_le_bytes([data[20], data[21]]),
            error: u32::from_le_bytes([data[22], data[23], data[24], data[25]]),
        })
    }
}

impl ConnectRequest {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.sockfd.to_le_bytes());
        data.extend_from_slice(&self.addr);
        data.extend_from_slice(&self.port.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 22 {
            return None;
        }
        let mut addr = [0u8; 16];
        addr.copy_from_slice(&data[4..20]);
        Some(Self {
            sockfd: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            addr,
            port: u16::from_le_bytes([data[20], data[21]]),
        })
    }
}

impl DataTransfer {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.sockfd.to_le_bytes());
        data.extend_from_slice(&self.flags.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            sockfd: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            flags: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        })
    }
}

impl PacketForward {
    pub fn serialize(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&self.interface_id.to_le_bytes());
        data.extend_from_slice(&self.packet_len.to_le_bytes());
        data
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            interface_id: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            packet_len: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        })
    }
}
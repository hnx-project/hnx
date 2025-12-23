//! TCP (Transmission Control Protocol) implementation for user-space network server

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;

/// TCP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub source_port: u16,
    pub destination_port: u16,
    pub sequence_number: u32,
    pub acknowledgment_number: u32,
    pub data_offset_reserved_ns: u16, // Data offset (4 bits) + Reserved (3 bits) + NS (1 bit) + Flags (8 bits)
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
}

/// Pending connection information
#[derive(Debug, Clone)]
pub struct PendingConnection {
    pub remote_ip: u32,
    pub remote_port: u16,
    pub sequence_number: u32,
    pub acknowledgment_number: u32,
}

/// TCP socket structure
#[derive(Debug, Clone)]
pub struct TcpSocket {
    pub local_port: u16,
    pub remote_addr: Option<(u32, u16)>, // (IP, port)
    pub state: TcpState,
    pub sequence_number: u32,
    pub acknowledgment_number: u32,
    pub rx_queue: VecDeque<alloc::vec::Vec<u8>>,
    pub tx_queue: VecDeque<alloc::vec::Vec<u8>>,
    /// Pending connections (only for listening sockets)
    pub pending_connections: Option<VecDeque<PendingConnection>>,
    /// Maximum length of the pending connections queue (only for listening sockets)
    pub backlog: Option<usize>,
}

/// TCP connection states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

impl TcpHeader {
    pub const MIN_SIZE: usize = 20;
    
    /// Create a new TCP header
    pub fn new(source_port: u16, destination_port: u16) -> Self {
        TcpHeader {
            source_port: source_port.to_be(),
            destination_port: destination_port.to_be(),
            sequence_number: 0,
            acknowledgment_number: 0,
            data_offset_reserved_ns: 0x5000, // Data offset = 5 (20 bytes), no flags set
            window_size: 65535,
            checksum: 0, // Will be calculated later
            urgent_pointer: 0,
        }
    }
    
    /// Parse a TCP header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::MIN_SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        Ok(TcpHeader {
            source_port: u16::from_be_bytes([data[0], data[1]]),
            destination_port: u16::from_be_bytes([data[2], data[3]]),
            sequence_number: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            acknowledgment_number: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            data_offset_reserved_ns: u16::from_be_bytes([data[12], data[13]]),
            window_size: u16::from_be_bytes([data[14], data[15]]),
            checksum: u16::from_be_bytes([data[16], data[17]]),
            urgent_pointer: u16::from_be_bytes([data[18], data[19]]),
        })
    }
    
    /// Convert TCP header to bytes
    pub fn to_bytes(&self) -> [u8; Self::MIN_SIZE] {
        let mut bytes = [0u8; Self::MIN_SIZE];
        
        bytes[0..2].copy_from_slice(&self.source_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.destination_port.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.sequence_number.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.acknowledgment_number.to_be_bytes());
        bytes[12..14].copy_from_slice(&self.data_offset_reserved_ns.to_be_bytes());
        bytes[14..16].copy_from_slice(&self.window_size.to_be_bytes());
        bytes[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[18..20].copy_from_slice(&self.urgent_pointer.to_be_bytes());
        
        bytes
    }
    
    /// Get the header length in bytes
    pub fn header_length(&self) -> usize {
        (((self.data_offset_reserved_ns >> 12) & 0xF) as usize) * 4
    }
}

/// TCP flags
pub mod flags {
    pub const FIN: u16 = 0x001;
    pub const SYN: u16 = 0x002;
    pub const RST: u16 = 0x004;
    pub const PSH: u16 = 0x008;
    pub const ACK: u16 = 0x010;
    pub const URG: u16 = 0x020;
    pub const ECE: u16 = 0x040;
    pub const CWR: u16 = 0x080;
    pub const NS: u16 = 0x100;
}

impl TcpHeader {
    /// Set a flag
    pub fn set_flag(&mut self, flag: u16) {
        let flags = self.data_offset_reserved_ns & 0xFF;
        self.data_offset_reserved_ns = (self.data_offset_reserved_ns & 0xFF00) | (flags | flag);
    }
    
    /// Clear a flag
    pub fn clear_flag(&mut self, flag: u16) {
        let flags = self.data_offset_reserved_ns & 0xFF;
        self.data_offset_reserved_ns = (self.data_offset_reserved_ns & 0xFF00) | (flags & !flag);
    }
    
    /// Check if a flag is set
    pub fn is_flag_set(&self, flag: u16) -> bool {
        (self.data_offset_reserved_ns & flag) != 0
    }
}

/// TCP packet
#[derive(Debug, Clone)]
pub struct TcpPacket {
    pub header: TcpHeader,
    pub payload: alloc::vec::Vec<u8>,
}

impl TcpPacket {
    /// Create a new TCP packet
    pub fn new(header: TcpHeader, payload: alloc::vec::Vec<u8>) -> Self {
        TcpPacket {
            header,
            payload,
        }
    }
    
    /// Parse a TCP packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < TcpHeader::MIN_SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = TcpHeader::from_bytes(data)?;
        let header_length = header.header_length();
        
        if data.len() < header_length {
            return Err(NetworkError::InvalidParameter);
        }
        
        let payload = data[header_length..].to_vec();
        
        Ok(TcpPacket {
            header,
            payload,
        })
    }
    
    /// Convert TCP packet to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::with_capacity(TcpHeader::MIN_SIZE + self.payload.len());
        
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
}

/// Global TCP socket table
static TCP_SOCKETS: Mutex<BTreeMap<u16, TcpSocket>> = Mutex::new(BTreeMap::new());

/// Initialize the TCP subsystem
pub fn init() {
    crate::println!("network/tcp: initializing");
}

/// Create a new TCP socket
pub fn create_socket(local_port: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS.lock();
    
    if sockets.contains_key(&local_port) {
        return Err(NetworkError::AddressInUse);
    }
    
    let socket = TcpSocket {
        local_port,
        remote_addr: None,
        state: TcpState::Closed,
        sequence_number: 0,
        acknowledgment_number: 0,
        rx_queue: VecDeque::new(),
        tx_queue: VecDeque::new(),
        pending_connections: None,
        backlog: None,
    };
    
    sockets.insert(local_port, socket);
    Ok(())
}

/// Close a TCP socket
pub fn close_socket(local_port: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS.lock();
    
    if sockets.remove(&local_port).is_some() {
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// Process an incoming TCP packet
pub fn process_packet(interface_id: u32, src_ip: u32, data: &[u8]) -> Result<()> {
    let packet = TcpPacket::from_bytes(data)?;
    
    let src_port = u16::from_be(packet.header.source_port);
    let dst_port = u16::from_be(packet.header.destination_port);
    
    crate::println!("network/tcp: received packet from port {} to port {}", 
                 src_port, dst_port);
    
    // Log flags
    let flags = packet.header.data_offset_reserved_ns & 0xFF;
    let mut flag_str = alloc::string::String::new();
    
    if (flags & flags::SYN) != 0 { flag_str.push_str("SYN "); }
    if (flags & flags::ACK) != 0 { flag_str.push_str("ACK "); }
    if (flags & flags::FIN) != 0 { flag_str.push_str("FIN "); }
    if (flags & flags::RST) != 0 { flag_str.push_str("RST "); }
    if (flags & flags::PSH) != 0 { flag_str.push_str("PSH "); }
    if (flags & flags::URG) != 0 { flag_str.push_str("URG "); }
    
    crate::println!("network/tcp: flags: {}", flag_str);
    
    // Process the packet according to TCP state machine
    process_tcp_packet(interface_id, src_ip, src_port, dst_port, &packet, flags)?;
    
    Ok(())
}

/// Process TCP packet according to state machine
fn process_tcp_packet(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                      packet: &TcpPacket, flags: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS.lock();
    
    // Check if we have a socket for this port
    if let Some(socket) = sockets.get_mut(&dst_port) {
        match socket.state {
            TcpState::Listen => {
                // Handle incoming connection request
                if (flags & flags::SYN) != 0 {
                    handle_syn_packet(interface_id, src_ip, src_port, dst_port, socket, packet)?;
                }
            }
            TcpState::SynSent => {
                // Handle SYN-ACK packet
                if (flags & flags::SYN) != 0 && (flags & flags::ACK) != 0 {
                    handle_syn_ack_packet(interface_id, src_ip, src_port, dst_port, socket, packet)?;
                }
            }
            TcpState::Established => {
                // Handle data packets
                if (flags & flags::ACK) != 0 {
                    handle_data_packet(interface_id, src_ip, src_port, dst_port, socket, packet, flags)?;
                }
            }
            _ => {
                // Handle other states
                crate::println!("network/tcp: unhandled state {:?}", socket.state);
            }
        }
    } else {
        crate::println!("network/tcp: no socket bound to port {}", dst_port);
        return Err(NetworkError::AddressNotAvailable);
    }
    
    Ok(())
}

/// Handle incoming SYN packet (connection request)
fn handle_syn_packet(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                     socket: &mut TcpSocket, packet: &TcpPacket) -> Result<()> {
    crate::println!("network/tcp: handling SYN packet from {}:{}", 
                 src_ip, src_port);
    
    // Check if this is a listening socket
    if socket.state != TcpState::Listen {
        return Err(NetworkError::InvalidParameter);
    }
    
    // Check if we have a pending connections queue
    let pending_connections = match socket.pending_connections.as_mut() {
        Some(queue) => queue,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if queue is full
    let backlog = socket.backlog.unwrap_or(5); // Default backlog of 5
    if pending_connections.len() >= backlog {
        crate::println!("network/tcp: connection queue is full, dropping SYN packet");
        return Ok(()); // Silently drop the packet
    }
    
    // Add connection to pending connections queue
    let pending_conn = PendingConnection {
        remote_ip: src_ip,
        remote_port: src_port,
        sequence_number: packet.header.sequence_number.wrapping_add(1),
        acknowledgment_number: packet.header.sequence_number.wrapping_add(1),
    };
    
    pending_connections.push_back(pending_conn.clone());
    
    crate::println!("network/tcp: added connection to pending queue, queue size: {}", 
                 pending_connections.len());
    
    // Send SYN-ACK packet immediately to establish the connection
    let mut syn_ack_header = TcpHeader::new(dst_port, src_port);
    syn_ack_header.set_flag(flags::SYN | flags::ACK);
    syn_ack_header.sequence_number = 0; // We would generate a random sequence number
    syn_ack_header.acknowledgment_number = pending_conn.sequence_number;
    
    let syn_ack_packet = TcpPacket::new(syn_ack_header, alloc::vec::Vec::new());
    let bytes = syn_ack_packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send SYN-ACK packet to kernel for transmission");
    
    Ok(())
}

/// Handle incoming SYN-ACK packet (connection response)
fn handle_syn_ack_packet(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                         socket: &mut TcpSocket, packet: &TcpPacket) -> Result<()> {
    crate::println!("network/tcp: handling SYN-ACK packet");
    
    // Verify acknowledgment number
    if packet.header.acknowledgment_number != socket.sequence_number.wrapping_add(1) {
        crate::println!("network/tcp: invalid acknowledgment number");
        return Err(NetworkError::InvalidParameter);
    }
    
    // Set acknowledgment number
    socket.acknowledgment_number = packet.header.sequence_number.wrapping_add(1);
    
    // Send ACK packet
    let mut ack_header = TcpHeader::new(dst_port, src_port);
    ack_header.set_flag(flags::ACK);
    ack_header.sequence_number = socket.sequence_number.wrapping_add(1);
    ack_header.acknowledgment_number = socket.acknowledgment_number;
    
    let ack_packet = TcpPacket::new(ack_header, alloc::vec::Vec::new());
    let bytes = ack_packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send ACK packet to kernel for transmission");
    
    // Transition to ESTABLISHED state
    socket.state = TcpState::Established;
    
    Ok(())
}

/// Handle incoming data packet
fn handle_data_packet(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                      socket: &mut TcpSocket, packet: &TcpPacket, flags: u16) -> Result<()> {
    crate::println!("network/tcp: handling data packet");
    
    // Verify acknowledgment number
    if packet.header.acknowledgment_number != socket.sequence_number.wrapping_add(1) {
        crate::println!("network/tcp: invalid acknowledgment number");
        // In a real implementation, we would handle this case appropriately
    }
    
    // Update acknowledgment number
    socket.acknowledgment_number = packet.header.sequence_number.wrapping_add(packet.payload.len() as u32);
    
    // Add payload to receive queue if there is data
    if !packet.payload.is_empty() {
        socket.rx_queue.push_back(packet.payload.clone());
    }
    
    // Send ACK packet if needed
    if !packet.payload.is_empty() || (flags & flags::FIN) != 0 {
        let mut ack_header = TcpHeader::new(dst_port, src_port);
        ack_header.set_flag(flags::ACK);
        ack_header.sequence_number = socket.sequence_number;
        ack_header.acknowledgment_number = socket.acknowledgment_number;
        
        // If this is a FIN packet, transition to CLOSE-WAIT state
        if (flags & flags::FIN) != 0 {
            socket.state = TcpState::CloseWait;
        }
        
        let ack_packet = TcpPacket::new(ack_header, alloc::vec::Vec::new());
        let bytes = ack_packet.to_bytes();
        
        // In a real implementation, we would send this packet via the IPC to the kernel
        // For now, we'll just log that we would send it
        crate::println!("network/tcp: would send ACK packet to kernel for transmission");
    }
    
    Ok(())
}

/// Send a TCP packet
pub fn send_packet(interface_id: u32, destination_ip: u32,
                   source_port: u16, destination_port: u16,
                   flags: u16, payload: alloc::vec::Vec<u8>) -> Result<()> {
    // Create TCP header
    let mut header = TcpHeader::new(source_port, destination_port);
    
    // Set flags
    header.data_offset_reserved_ns |= flags;
    
    // Create TCP packet
    let packet = TcpPacket::new(header, payload);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send packet to kernel for transmission");
    
    Ok(())
}

/// Establish a TCP connection (active open)
pub fn connect(interface_id: u32, local_port: u16, remote_ip: u32, remote_port: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Set remote address
    socket.remote_addr = Some((remote_ip, remote_port));
    
    // Generate initial sequence number
    socket.sequence_number = 0; // We would generate a random sequence number in a real implementation
    
    // Send SYN packet
    let mut syn_header = TcpHeader::new(local_port, remote_port);
    syn_header.set_flag(flags::SYN);
    syn_header.sequence_number = socket.sequence_number;
    
    let syn_packet = TcpPacket::new(syn_header, alloc::vec::Vec::new());
    let bytes = syn_packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send SYN packet to kernel for transmission");
    
    // Transition to SYN-SENT state
    socket.state = TcpState::SynSent;
    
    Ok(())
}

/// Listen for incoming TCP connections
pub fn listen(local_port: u16, backlog: usize) -> Result<()> {
    let mut sockets = TCP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Transition to LISTEN state
    socket.state = TcpState::Listen;
    
    // Initialize pending connections queue
    socket.pending_connections = Some(VecDeque::new());
    socket.backlog = Some(backlog);
    
    Ok(())
}

/// Accept an incoming TCP connection
pub fn accept(listen_port: u16) -> Result<(u32, u16)> {
    let mut sockets = TCP_SOCKETS.lock();
    
    // Get the listening socket
    let listen_socket = match sockets.get_mut(&listen_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if this is a listening socket
    if listen_socket.state != TcpState::Listen {
        return Err(NetworkError::InvalidParameter);
    }
    
    // Get the pending connections queue
    let pending_connections = match listen_socket.pending_connections.as_mut() {
        Some(queue) => queue,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if there are any pending connections
    if pending_connections.is_empty() {
        return Err(NetworkError::Timeout); // No connections available
    }
    
    // Get the first pending connection
    let pending_conn = pending_connections.pop_front().unwrap();
    
    crate::println!("network/tcp: accepting connection from {}:{}", 
                 pending_conn.remote_ip, pending_conn.remote_port);
    
    // Send final ACK to complete three-way handshake
    let mut ack_header = TcpHeader::new(listen_port, pending_conn.remote_port);
    ack_header.set_flag(flags::ACK);
    ack_header.sequence_number = pending_conn.acknowledgment_number;
    ack_header.acknowledgment_number = pending_conn.sequence_number;
    
    let ack_packet = TcpPacket::new(ack_header, alloc::vec::Vec::new());
    let bytes = ack_packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send final ACK packet to kernel for transmission");
    
    // Return the remote address
    Ok((pending_conn.remote_ip, pending_conn.remote_port))
}

/// Send data through a TCP socket
pub fn send_to(local_port: u16, data: &[u8]) -> Result<usize> {
    let mut sockets = TCP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if connection is established
    if socket.state != TcpState::Established {
        return Err(NetworkError::InvalidParameter);
    }
    
    // Get remote address
    let (remote_ip, remote_port) = match socket.remote_addr {
        Some(addr) => addr,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Send data packet
    let mut data_header = TcpHeader::new(local_port, remote_port);
    data_header.set_flag(flags::ACK);
    data_header.sequence_number = socket.sequence_number;
    data_header.acknowledgment_number = socket.acknowledgment_number;
    
    let data_packet = TcpPacket::new(data_header, data.to_vec());
    let bytes = data_packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send data packet to kernel for transmission");
    
    // Update sequence number
    socket.sequence_number = socket.sequence_number.wrapping_add(data.len() as u32);
    
    Ok(data.len())
}

/// Receive data from a TCP socket
pub fn recv_from(local_port: u16, buffer: &mut [u8]) -> Result<usize> {
    let mut sockets = TCP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if there's data in the receive queue
    if let Some(data) = socket.rx_queue.pop_front() {
        let len = core::cmp::min(buffer.len(), data.len());
        buffer[..len].copy_from_slice(&data[..len]);
        Ok(len)
    } else {
        // No data available
        Ok(0)
    }
}

/// Close a TCP connection
pub fn close(local_port: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS.lock();
    
    let socket = match sockets.get_mut(&local_port) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Get remote address
    let (remote_ip, remote_port) = match socket.remote_addr {
        Some(addr) => addr,
        None => {
            // If no remote address, just close the socket
            socket.state = TcpState::Closed;
            return Ok(());
        }
    };
    
    // Send FIN packet
    let mut fin_header = TcpHeader::new(local_port, remote_port);
    fin_header.set_flag(flags::FIN | flags::ACK);
    fin_header.sequence_number = socket.sequence_number;
    fin_header.acknowledgment_number = socket.acknowledgment_number;
    
    let fin_packet = TcpPacket::new(fin_header, alloc::vec::Vec::new());
    let bytes = fin_packet.to_bytes();
    
    // In a real implementation, we would send this packet via the IPC to the kernel
    // For now, we'll just log that we would send it
    crate::println!("network/tcp: would send FIN packet to kernel for transmission");
    
    // Transition to FIN-WAIT-1 state
    socket.state = TcpState::FinWait1;
    
    Ok(())
}
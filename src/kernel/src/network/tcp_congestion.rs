//! TCP (Transmission Control Protocol) implementation with congestion control
//!
//! This module implements the TCP protocol with support for congestion control algorithms
//! including Tahoe, Reno, and NewReno. It builds upon the basic TCP implementation to
//! provide more sophisticated congestion management.

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;

/// TCP Congestion Control Algorithms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CongestionControlAlgorithm {
    Tahoe,
    Reno,
    NewReno,
}

/// Congestion Control State
#[derive(Debug, Clone)]
pub struct CongestionControl {
    pub algorithm: CongestionControlAlgorithm,
    pub cwnd: u32,           // Congestion window
    pub ssthresh: u32,       // Slow start threshold
    pub dup_acks: u32,       // Number of duplicate acknowledgments
    pub recover: u32,        // Recovery point
    pub in_recovery: bool,   // Are we in recovery mode?
}

impl CongestionControl {
    /// Create a new congestion control state with the specified algorithm
    pub fn new(algorithm: CongestionControlAlgorithm) -> Self {
        CongestionControl {
            algorithm,
            cwnd: 1,        // Initial congestion window
            ssthresh: 65535, // Initial slow start threshold (64KB)
            dup_acks: 0,
            recover: 0,
            in_recovery: false,
        }
    }
    
    /// Handle a duplicate acknowledgment
    pub fn on_dup_ack(&mut self) {
        self.dup_acks += 1;
        
        match self.algorithm {
            CongestionControlAlgorithm::Tahoe => {
                // Tahoe: Enter slow start on triple duplicate ACK
                if self.dup_acks == 3 {
                    self.ssthresh = self.cwnd / 2;
                    self.cwnd = 1;
                    self.in_recovery = true;
                }
            }
            CongestionControlAlgorithm::Reno => {
                // Reno: Fast retransmit and fast recovery
                if self.dup_acks == 3 {
                    self.ssthresh = self.cwnd / 2;
                    self.cwnd = self.ssthresh + 3; // Add 3 for the 3 duplicate ACKs
                    self.recover = 0; // Would be set to highest sequence number sent so far
                    self.in_recovery = true;
                } else if self.dup_acks > 3 && self.in_recovery {
                    self.cwnd += 1;
                }
            }
            CongestionControlAlgorithm::NewReno => {
                // NewReno: Partial ACK handling
                if self.dup_acks == 3 {
                    self.ssthresh = self.cwnd / 2;
                    self.cwnd = self.ssthresh + 3;
                    self.recover = 0; // Would be set to highest sequence number sent so far
                    self.in_recovery = true;
                } else if self.dup_acks > 3 && self.in_recovery {
                    self.cwnd += 1;
                }
            }
        }
    }
    
    /// Handle a new acknowledgment
    pub fn on_new_ack(&mut self, ack_count: u32) {
        self.dup_acks = 0;
        
        if self.in_recovery {
            // We're in recovery mode
            match self.algorithm {
                CongestionControlAlgorithm::Tahoe => {
                    self.in_recovery = false;
                    self.cwnd = self.ssthresh;
                }
                CongestionControlAlgorithm::Reno => {
                    self.in_recovery = false;
                    self.cwnd = self.ssthresh;
                }
                CongestionControlAlgorithm::NewReno => {
                    // Check if this is a partial ACK
                    if ack_count < self.recover {
                        self.cwnd += 1;
                    } else {
                        self.in_recovery = false;
                        self.cwnd = self.ssthresh;
                    }
                }
            }
        } else {
            // Normal operation
            if self.cwnd < self.ssthresh {
                // Slow start
                self.cwnd += ack_count;
            } else {
                // Congestion avoidance
                self.cwnd += ack_count * ack_count / self.cwnd;
                if self.cwnd == 0 {
                    self.cwnd = 1;
                }
            }
        }
    }
    
    /// Handle a timeout
    pub fn on_timeout(&mut self) {
        self.ssthresh = self.cwnd / 2;
        if self.ssthresh < 2 {
            self.ssthresh = 2;
        }
        self.cwnd = 1;
        self.dup_acks = 0;
        self.in_recovery = false;
    }
    
    /// Get the current send window size
    pub fn send_window(&self) -> u32 {
        self.cwnd
    }
}

/// Enhanced TCP socket structure with congestion control
#[derive(Debug, Clone)]
pub struct TcpSocketWithCC {
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
    /// Congestion control state
    pub congestion_control: CongestionControl,
}

// Re-export existing TCP types that we're extending
pub use super::tcp::{
    TcpHeader, TcpPacket, TcpState, PendingConnection, 
    flags, process_packet as process_ipv4_packet
};

/// Global TCP socket table with congestion control
static TCP_SOCKETS_CC: Mutex<BTreeMap<u16, TcpSocketWithCC>> = Mutex::new(BTreeMap::new());

/// Initialize the TCP congestion control subsystem
pub fn init_congestion_control() {
    crate::info!("network/tcp_cc: initializing congestion control");
}

/// Create a new TCP socket with congestion control
pub fn create_socket_cc(local_port: u16, algorithm: CongestionControlAlgorithm) -> Result<()> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
    if sockets.contains_key(&local_port) {
        return Err(NetworkError::AddressInUse);
    }
    
    let socket = TcpSocketWithCC {
        local_port,
        remote_addr: None,
        state: TcpState::Closed,
        sequence_number: 0,
        acknowledgment_number: 0,
        rx_queue: VecDeque::new(),
        tx_queue: VecDeque::new(),
        pending_connections: None,
        backlog: None,
        congestion_control: CongestionControl::new(algorithm),
    };
    
    sockets.insert(local_port, socket);
    Ok(())
}

/// Close a TCP socket with congestion control
pub fn close_socket_cc(local_port: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
    if sockets.remove(&local_port).is_some() {
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// Process an incoming TCP packet with congestion control
pub fn process_packet_cc(interface_id: u32, src_ip: u32, data: &[u8]) -> Result<()> {
    let packet = TcpPacket::from_bytes(data)?;
    
    let src_port = u16::from_be(packet.header.source_port);
    let dst_port = u16::from_be(packet.header.destination_port);
    
    crate::info!("network/tcp_cc: received packet from port {} to port {}", 
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
    
    crate::info!("network/tcp_cc: flags: {}", flag_str);
    
    // Process the packet according to TCP state machine with congestion control
    process_tcp_packet_cc(interface_id, src_ip, src_port, dst_port, &packet, flags)?;
    
    Ok(())
}

/// Process TCP packet according to state machine with congestion control
fn process_tcp_packet_cc(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                        packet: &TcpPacket, flags: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
    // Check if we have a socket for this port
    if let Some(socket) = sockets.get_mut(&dst_port) {
        // Handle congestion control based on ACKs
        if (flags & flags::ACK) != 0 {
            // Check if this is a duplicate ACK
            if packet.header.acknowledgment_number == socket.acknowledgment_number {
                // Duplicate ACK
                socket.congestion_control.on_dup_ack();
            } else {
                // New ACK
                let ack_count = packet.header.acknowledgment_number - socket.acknowledgment_number;
                socket.congestion_control.on_new_ack(ack_count);
            }
        }
        
        // Process state transitions (similar to basic TCP)
        match socket.state {
            TcpState::Listen => {
                // Handle incoming connection request
                if (flags & flags::SYN) != 0 {
                    handle_syn_packet_cc(interface_id, src_ip, src_port, dst_port, socket, packet)?;
                }
            }
            TcpState::SynSent => {
                // Handle SYN-ACK packet
                if (flags & flags::SYN) != 0 && (flags & flags::ACK) != 0 {
                    handle_syn_ack_packet_cc(interface_id, src_ip, src_port, dst_port, socket, packet)?;
                }
            }
            TcpState::Established => {
                // Handle data packets
                if (flags & flags::ACK) != 0 {
                    handle_data_packet_cc(interface_id, src_ip, src_port, dst_port, socket, packet, flags)?;
                }
            }
            _ => {
                // Handle other states
                crate::info!("network/tcp_cc: unhandled state {:?}", socket.state);
            }
        }
    } else {
        crate::info!("network/tcp_cc: no socket bound to port {}", dst_port);
        return Err(NetworkError::AddressNotAvailable);
    }
    
    Ok(())
}

/// Handle incoming SYN packet (connection request) with congestion control
fn handle_syn_packet_cc(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                       socket: &mut TcpSocketWithCC, packet: &TcpPacket) -> Result<()> {
    crate::info!("network/tcp_cc: handling SYN packet from {}:{}", 
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
        crate::info!("network/tcp_cc: connection queue is full, dropping SYN packet");
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
    
    crate::info!("network/tcp_cc: added connection to pending queue, queue size: {}", 
                 pending_connections.len());
    
    // Send SYN-ACK packet immediately to establish the connection
    let mut syn_ack_header = TcpHeader::new(dst_port, src_port);
    syn_ack_header.set_flag(flags::SYN | flags::ACK);
    syn_ack_header.sequence_number = 0; // We would generate a random sequence number
    syn_ack_header.acknowledgment_number = pending_conn.sequence_number;
    
    let syn_ack_packet = TcpPacket::new(syn_ack_header, alloc::vec::Vec::new());
    let bytes = syn_ack_packet.to_bytes();
    
    crate::network::ipv4::send_packet(interface_id, src_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    Ok(())
}

/// Handle incoming SYN-ACK packet (connection response) with congestion control
fn handle_syn_ack_packet_cc(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                           socket: &mut TcpSocketWithCC, packet: &TcpPacket) -> Result<()> {
    crate::info!("network/tcp_cc: handling SYN-ACK packet");
    
    // Verify acknowledgment number
    if packet.header.acknowledgment_number != socket.sequence_number.wrapping_add(1) {
        crate::info!("network/tcp_cc: invalid acknowledgment number");
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
    
    crate::network::ipv4::send_packet(interface_id, src_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    // Transition to ESTABLISHED state
    socket.state = TcpState::Established;
    
    Ok(())
}

/// Handle incoming data packet with congestion control
fn handle_data_packet_cc(interface_id: u32, src_ip: u32, src_port: u16, dst_port: u16, 
                        socket: &mut TcpSocketWithCC, packet: &TcpPacket, flags: u16) -> Result<()> {
    crate::info!("network/tcp_cc: handling data packet");
    
    // Verify acknowledgment number
    if packet.header.acknowledgment_number != socket.sequence_number.wrapping_add(1) {
        crate::info!("network/tcp_cc: invalid acknowledgment number");
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
        
        crate::network::ipv4::send_packet(interface_id, src_ip, 
                                          crate::network::ipv4::protocol::TCP, 
                                          bytes)?;
    }
    
    Ok(())
}

/// Send a TCP packet with congestion control
pub fn send_packet_cc(interface_id: u32, destination_ip: u32,
                      source_port: u16, destination_port: u16,
                      flags: u16, payload: alloc::vec::Vec<u8>) -> Result<()> {
    // Check congestion window before sending
    let sockets = TCP_SOCKETS_CC.lock();
    
    if let Some(socket) = sockets.get(&source_port) {
        let cwnd = socket.congestion_control.send_window();
        
        // In a real implementation, we would check if the payload fits within the congestion window
        // For now, we'll just log the congestion window size
        crate::info!("network/tcp_cc: sending packet with congestion window: {}", cwnd);
    }
    
    // Create TCP header
    let mut header = TcpHeader::new(source_port, destination_port);
    
    // Set flags
    header.data_offset_reserved_ns |= flags;
    
    // Create TCP packet
    let packet = TcpPacket::new(header, payload);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Send the packet via IPv4
    crate::network::ipv4::send_packet(interface_id, destination_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    Ok(())
}

/// Establish a TCP connection (active open) with congestion control
pub fn connect_cc(interface_id: u32, local_port: u16, remote_ip: u32, remote_port: u16,
                  algorithm: CongestionControlAlgorithm) -> Result<()> {
    // First create the socket with congestion control
    create_socket_cc(local_port, algorithm)?;
    
    let mut sockets = TCP_SOCKETS_CC.lock();
    
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
    
    crate::network::ipv4::send_packet(interface_id, remote_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    // Transition to SYN-SENT state
    socket.state = TcpState::SynSent;
    
    Ok(())
}

/// Listen for incoming TCP connections with congestion control
pub fn listen_cc(local_port: u16, backlog: usize, algorithm: CongestionControlAlgorithm) -> Result<()> {
    // First create the socket with congestion control
    create_socket_cc(local_port, algorithm)?;
    
    let mut sockets = TCP_SOCKETS_CC.lock();
    
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

/// Accept an incoming TCP connection with congestion control
pub fn accept_cc(listen_port: u16) -> Result<(u32, u16)> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
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
    
    crate::info!("network/tcp_cc: accepting connection from {}:{}", 
                 pending_conn.remote_ip, pending_conn.remote_port);
    
    // Send final ACK to complete three-way handshake
    let mut ack_header = TcpHeader::new(listen_port, pending_conn.remote_port);
    ack_header.set_flag(flags::ACK);
    ack_header.sequence_number = pending_conn.acknowledgment_number;
    ack_header.acknowledgment_number = pending_conn.sequence_number;
    
    let ack_packet = TcpPacket::new(ack_header, alloc::vec::Vec::new());
    let bytes = ack_packet.to_bytes();
    
    // In a real implementation, we would get the interface ID from the socket
    // For now, we'll use a placeholder value
    let interface_id = 0;
    
    crate::network::ipv4::send_packet(interface_id, pending_conn.remote_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    // Return the remote address
    Ok((pending_conn.remote_ip, pending_conn.remote_port))
}

/// Send data through a TCP socket with congestion control
pub fn send_to_cc(local_port: u16, data: &[u8]) -> Result<usize> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
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
    
    // Check congestion window
    let cwnd = socket.congestion_control.send_window();
    
    // In a real implementation, we would check if the data fits within the congestion window
    // For now, we'll just log the congestion window size
    crate::info!("network/tcp_cc: sending data with congestion window: {}", cwnd);
    
    // Send data packet
    let mut data_header = TcpHeader::new(local_port, remote_port);
    data_header.set_flag(flags::ACK);
    data_header.sequence_number = socket.sequence_number;
    data_header.acknowledgment_number = socket.acknowledgment_number;
    
    let data_packet = TcpPacket::new(data_header, data.to_vec());
    let bytes = data_packet.to_bytes();
    
    // In a real implementation, we would get the interface ID from the socket
    // For now, we'll use a placeholder value
    let interface_id = 0;
    
    crate::network::ipv4::send_packet(interface_id, remote_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    // Update sequence number
    socket.sequence_number = socket.sequence_number.wrapping_add(data.len() as u32);
    
    Ok(data.len())
}

/// Receive data from a TCP socket with congestion control
pub fn recv_from_cc(local_port: u16, buffer: &mut [u8]) -> Result<usize> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
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

/// Close a TCP connection with congestion control
pub fn close_cc(local_port: u16) -> Result<()> {
    let mut sockets = TCP_SOCKETS_CC.lock();
    
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
    
    // In a real implementation, we would get the interface ID from the socket
    // For now, we'll use a placeholder value
    let interface_id = 0;
    
    crate::network::ipv4::send_packet(interface_id, remote_ip, 
                                      crate::network::ipv4::protocol::TCP, 
                                      bytes)?;
    
    // Transition to FIN-WAIT-1 state
    socket.state = TcpState::FinWait1;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_congestion_control_tahoe() {
        let mut cc = CongestionControl::new(CongestionControlAlgorithm::Tahoe);
        
        // Test initial state
        assert_eq!(cc.cwnd, 1);
        assert_eq!(cc.ssthresh, 65535);
        assert_eq!(cc.dup_acks, 0);
        assert_eq!(cc.in_recovery, false);
        
        // Test slow start
        cc.on_new_ack(1);
        assert_eq!(cc.cwnd, 2);
        
        cc.on_new_ack(2);
        assert_eq!(cc.cwnd, 4);
        
        // Test transition to congestion avoidance
        cc.ssthresh = 3;
        cc.on_new_ack(1);
        assert_eq!(cc.cwnd, 5); // Should be in congestion avoidance now
        
        // Test triple duplicate ACK
        cc.on_dup_ack(); // dup_acks = 1
        assert_eq!(cc.dup_acks, 1);
        assert_eq!(cc.cwnd, 5); // No change yet
        
        cc.on_dup_ack(); // dup_acks = 2
        assert_eq!(cc.dup_acks, 2);
        assert_eq!(cc.cwnd, 5); // No change yet
        
        cc.on_dup_ack(); // dup_acks = 3, should trigger fast retransmit
        assert_eq!(cc.dup_acks, 3);
        assert_eq!(cc.cwnd, 1); // Back to slow start
        assert_eq!(cc.ssthresh, 2); // Half of previous cwnd
        assert_eq!(cc.in_recovery, true);
    }
    
    #[test]
    fn test_congestion_control_reno() {
        let mut cc = CongestionControl::new(CongestionControlAlgorithm::Reno);
        
        // Test initial state
        assert_eq!(cc.cwnd, 1);
        assert_eq!(cc.ssthresh, 65535);
        assert_eq!(cc.dup_acks, 0);
        assert_eq!(cc.in_recovery, false);
        
        // Test slow start
        cc.on_new_ack(1);
        assert_eq!(cc.cwnd, 2);
        
        cc.on_new_ack(2);
        assert_eq!(cc.cwnd, 4);
        
        // Test transition to congestion avoidance
        cc.ssthresh = 3;
        cc.on_new_ack(1);
        assert_eq!(cc.cwnd, 5); // Should be in congestion avoidance now
        
        // Test triple duplicate ACK (Fast Retransmit)
        cc.on_dup_ack(); // dup_acks = 1
        assert_eq!(cc.dup_acks, 1);
        assert_eq!(cc.cwnd, 5); // No change yet
        
        cc.on_dup_ack(); // dup_acks = 2
        assert_eq!(cc.dup_acks, 2);
        assert_eq!(cc.cwnd, 5); // No change yet
        
        cc.on_dup_ack(); // dup_acks = 3, should trigger fast retransmit
        assert_eq!(cc.dup_acks, 3);
        assert_eq!(cc.cwnd, 5); // ssthresh (2) + 3 duplicate ACKs
        assert_eq!(cc.ssthresh, 2); // Half of previous cwnd
        assert_eq!(cc.in_recovery, true);
        
        // Test fast recovery
        cc.on_dup_ack(); // dup_acks = 4, in recovery
        assert_eq!(cc.cwnd, 6); // Increment cwnd during recovery
    }
    
    #[test]
    fn test_congestion_control_timeout() {
        let mut cc = CongestionControl::new(CongestionControlAlgorithm::Tahoe);
        
        // Increase cwnd to simulate successful transmission
        cc.on_new_ack(1);
        cc.on_new_ack(1);
        cc.on_new_ack(1);
        assert_eq!(cc.cwnd, 4);
        
        // Simulate a timeout
        cc.on_timeout();
        assert_eq!(cc.cwnd, 1); // Reset to 1
        assert_eq!(cc.ssthresh, 2); // Half of previous cwnd (4/2 = 2)
        assert_eq!(cc.dup_acks, 0); // Reset duplicate ACK counter
        assert_eq!(cc.in_recovery, false); // Exit recovery mode
    }
}
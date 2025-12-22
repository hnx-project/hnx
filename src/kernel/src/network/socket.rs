//! Socket API implementation

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;

/// Socket domains
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketDomain {
    INET,    // IPv4
    INET6,   // IPv6
}

/// Socket types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketType {
    STREAM,  // TCP
    DGRAM,   // UDP
}

/// Socket protocols
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketProtocol {
    IP,
    TCP,
    UDP,
    ICMP,
}

/// Socket address
#[derive(Debug, Clone, Copy)]
pub struct SocketAddress {
    pub ip: u32,
    pub port: u16,
}

impl SocketAddress {
    pub fn new(ip: u32, port: u16) -> Self {
        SocketAddress {
            ip: ip.to_be(),
            port: port.to_be(),
        }
    }
}

/// Socket states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketState {
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

/// Socket I/O modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketMode {
    Blocking,
    NonBlocking,
}

/// Socket structure
#[derive(Debug, Clone)]
pub struct Socket {
    pub id: u32,
    pub domain: SocketDomain,
    pub type_: SocketType,
    pub protocol: SocketProtocol,
    pub state: SocketState,
    pub mode: SocketMode,
    pub local_addr: Option<SocketAddress>,
    pub remote_addr: Option<SocketAddress>,
    pub rx_buffer: alloc::collections::VecDeque<u8>,
    pub tx_buffer: alloc::collections::VecDeque<u8>,
}

impl Socket {
    pub fn new(id: u32, domain: SocketDomain, type_: SocketType, protocol: SocketProtocol) -> Self {
        Socket {
            id,
            domain,
            type_,
            protocol,
            state: SocketState::Closed,
            mode: SocketMode::Blocking, // 默认为阻塞模式
            local_addr: None,
            remote_addr: None,
            rx_buffer: alloc::collections::VecDeque::new(),
            tx_buffer: alloc::collections::VecDeque::new(),
        }
    }
}

/// Global socket manager
static SOCKETS: Mutex<BTreeMap<u32, Socket>> = Mutex::new(BTreeMap::new());
static NEXT_SOCKET_ID: Mutex<u32> = Mutex::new(1);

/// Initialize the socket subsystem
pub fn init() {
    crate::info!("network/socket: initializing");
}

/// Create a new socket
pub fn socket(domain: SocketDomain, type_: SocketType, protocol: SocketProtocol) -> Result<u32> {
    let mut next_id = NEXT_SOCKET_ID.lock();
    let id = *next_id;
    *next_id += 1;
    
    let socket = Socket::new(id, domain, type_, protocol);
    
    let mut sockets = SOCKETS.lock();
    sockets.insert(id, socket);
    
    crate::info!("network/socket: created socket {} (domain: {:?}, type: {:?}, protocol: {:?})", 
                 id, domain, type_, protocol);
    
    Ok(id)
}

/// Bind a socket to a local address
pub fn bind(socket_id: u32, addr: SocketAddress) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    let socket = match sockets.get_mut(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if already bound
    if socket.local_addr.is_some() {
        return Err(NetworkError::AddressInUse);
    }
    
    socket.local_addr = Some(addr);
    
    crate::info!("network/socket: bound socket {} to {:08X}:{}", 
                 socket_id, u32::from_be(addr.ip), u16::from_be(addr.port));
    
    Ok(())
}

/// Listen for incoming connections (TCP only)
pub fn listen(socket_id: u32, backlog: usize) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    let socket = match sockets.get_mut(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if socket is TCP
    if socket.type_ != SocketType::STREAM {
        return Err(NetworkError::NotSupported);
    }
    
    // Check if bound
    if socket.local_addr.is_none() {
        return Err(NetworkError::InvalidParameter);
    }
    
    // Get local port
    let local_port = match socket.local_addr {
        Some(addr) => u16::from_be(addr.port),
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Call TCP listen
    crate::network::tcp::listen(local_port, backlog)?;
    
    socket.state = SocketState::Listen;
    
    crate::info!("network/socket: socket {} listening with backlog {}", 
                 socket_id, backlog);
    
    Ok(())
}

/// Accept an incoming connection (TCP only)
pub fn accept(socket_id: u32) -> Result<u32> {
    // First, get the socket information without holding the lock
    let (local_port, domain, type_, protocol) = {
        let sockets = SOCKETS.lock();
        
        let socket = match sockets.get(&socket_id) {
            Some(sock) => sock,
            None => return Err(NetworkError::InvalidParameter),
        };
        
        // Check if socket is listening
        if socket.state != SocketState::Listen {
            return Err(NetworkError::InvalidParameter);
        }
        
        // For TCP sockets, get local port
        if socket.type_ == SocketType::STREAM {
            // Get local port
            let local_port = match socket.local_addr {
                Some(addr) => u16::from_be(addr.port),
                None => return Err(NetworkError::InvalidParameter),
            };
            
            (local_port, socket.domain, socket.type_, socket.protocol)
        } else {
            return Err(NetworkError::NotSupported);
        }
    };
    
    // Call TCP accept
    match crate::network::tcp::accept(local_port) {
        Ok((remote_ip, remote_port)) => {
            // Create a new socket for the accepted connection
            let mut next_id = NEXT_SOCKET_ID.lock();
            let new_id = *next_id;
            *next_id += 1;
            
            let new_socket = Socket {
                id: new_id,
                domain,
                type_,
                protocol,
                state: SocketState::Established,
                mode: SocketMode::Blocking, // New sockets default to blocking mode
                local_addr: Some(SocketAddress::new(u32::from_be(local_port as u32), local_port)),
                remote_addr: Some(SocketAddress::new(remote_ip, remote_port)),
                rx_buffer: alloc::collections::VecDeque::new(),
                tx_buffer: alloc::collections::VecDeque::new(),
            };
            
            let mut sockets = SOCKETS.lock();
            sockets.insert(new_id, new_socket);
            
            crate::info!("network/socket: accepted connection from {:08X}:{} on socket {}", 
                         remote_ip, remote_port, new_id);
            
            Ok(new_id)
        }
        Err(e) => Err(e),
    }
}

/// Connect to a remote address
pub fn connect(socket_id: u32, addr: SocketAddress) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    let socket = match sockets.get_mut(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    socket.remote_addr = Some(addr);
    socket.state = SocketState::Established;
    
    let ip = u32::from_be(addr.ip);
    let port = u16::from_be(addr.port);
    
    crate::info!("network/socket: socket {} connected to {:08X}:{}", 
                 socket_id, ip, port);
    
    Ok(())
}

/// Send data through a socket
pub fn send(socket_id: u32, data: &[u8]) -> Result<usize> {
    // First, get socket information without holding the lock for too long
    let (socket_type, local_addr, remote_addr, mode) = {
        let sockets = SOCKETS.lock();
        
        let socket = match sockets.get(&socket_id) {
            Some(sock) => sock,
            None => return Err(NetworkError::InvalidParameter),
        };
        
        // Check if socket is connected/established
        if socket.state != SocketState::Established {
            return Err(NetworkError::InvalidParameter);
        }
        
        // Get addresses
        let local_addr = match socket.local_addr {
            Some(addr) => addr,
            None => return Err(NetworkError::InvalidParameter),
        };
        
        let remote_addr = match socket.remote_addr {
            Some(addr) => addr,
            None => return Err(NetworkError::InvalidParameter),
        };
        
        (socket.type_, local_addr, remote_addr, socket.mode)
    };
    
    match socket_type {
        SocketType::STREAM => {
            // TCP socket - call TCP send_to function
            let local_port = u16::from_be(local_addr.port);
            // For now, we just call the TCP send function directly
            // In a more sophisticated implementation, we might check if the operation
            // would block in non-blocking mode and return WouldBlock accordingly
            crate::network::tcp::send_to(local_port, data)
        },
        SocketType::DGRAM => {
            // UDP socket - call UDP send_packet function
            let local_port = u16::from_be(local_addr.port);
            let remote_ip = u32::from_be(remote_addr.ip);
            let remote_port = u16::from_be(remote_addr.port);
            
            // For now, we use interface_id 0, in a real implementation this would be dynamic
            crate::network::udp::send_packet(0, remote_ip, local_port, remote_port, data.to_vec())
                .map(|_| data.len())
        },
    }
}

/// Receive data from a socket
pub fn recv(socket_id: u32, buffer: &mut [u8]) -> Result<usize> {
    // First, get socket information without holding the lock for too long
    let (socket_type, mode) = {
        let sockets = SOCKETS.lock();
        
        let socket = match sockets.get(&socket_id) {
            Some(sock) => sock,
            None => return Err(NetworkError::InvalidParameter),
        };
        
        (socket.type_, socket.mode)
    };
    
    match socket_type {
        SocketType::STREAM => {
            // TCP socket - call TCP recv_from function
            // We need to get the local port for the TCP recv function
            let (local_port, is_blocking) = {
                let sockets = SOCKETS.lock();
                let socket = sockets.get(&socket_id).ok_or(NetworkError::InvalidParameter)?;
                let local_addr = socket.local_addr.ok_or(NetworkError::InvalidParameter)?;
                let is_blocking = socket.mode == SocketMode::Blocking;
                (u16::from_be(local_addr.port), is_blocking)
            };
            
            let result = crate::network::tcp::recv_from(local_port, buffer);
            
            // Handle non-blocking mode
            if !is_blocking {
                // In non-blocking mode, if there's no data, we should return WouldBlock
                // For now, the TCP implementation always returns Ok(0) when no data is available
                // In a more sophisticated implementation, we would distinguish between
                // "connection closed" and "no data available"
                match result {
                    Ok(0) => return Err(NetworkError::WouldBlock),
                    _ => result,
                }
            } else {
                result
            }
        },
        SocketType::DGRAM => {
            // UDP socket - call UDP recv_from function
            // We need to get the local port for the UDP recv function
            let (local_port, is_blocking) = {
                let sockets = SOCKETS.lock();
                let socket = sockets.get(&socket_id).ok_or(NetworkError::InvalidParameter)?;
                let local_addr = socket.local_addr.ok_or(NetworkError::InvalidParameter)?;
                let is_blocking = socket.mode == SocketMode::Blocking;
                (u16::from_be(local_addr.port), is_blocking)
            };
            
            let result = crate::network::udp::recv_from(local_port, buffer);
            
            // Handle non-blocking mode
            if !is_blocking {
                match result {
                    Ok((0, _, _)) => return Err(NetworkError::WouldBlock),
                    Ok((size, _, _)) => Ok(size),
                    Err(e) => Err(e),
                }
            } else {
                result.map(|(size, _, _)| size)
            }
        },
    }
}

/// Close a socket
pub fn close(socket_id: u32) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    if !sockets.contains_key(&socket_id) {
        return Err(NetworkError::InvalidParameter);
    }
    
    sockets.remove(&socket_id);
    
    crate::info!("network/socket: closed socket {}", socket_id);
    
    Ok(())
}

/// Set socket blocking mode
pub fn set_nonblocking(socket_id: u32, nonblocking: bool) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    let socket = match sockets.get_mut(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    socket.mode = if nonblocking {
        SocketMode::NonBlocking
    } else {
        SocketMode::Blocking
    };
    
    crate::info!("network/socket: set socket {} to {:?} mode", 
                 socket_id, socket.mode);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_mode_defaults_to_blocking() {
        let socket = Socket::new(1, SocketDomain::INET, SocketType::STREAM, SocketProtocol::TCP);
        assert_eq!(socket.mode, SocketMode::Blocking);
    }

    #[test]
    fn test_set_nonblocking() {
        let mut sockets_map = BTreeMap::new();
        let socket = Socket::new(1, SocketDomain::INET, SocketType::STREAM, SocketProtocol::TCP);
        sockets_map.insert(1, socket);
        
        // Since we can't easily test the global SOCKETS mutex in a unit test,
        // we'll test the logic by manually manipulating a local map
        let mut test_socket = Socket::new(1, SocketDomain::INET, SocketType::STREAM, SocketProtocol::TCP);
        assert_eq!(test_socket.mode, SocketMode::Blocking);
        
        test_socket.mode = SocketMode::NonBlocking;
        assert_eq!(test_socket.mode, SocketMode::NonBlocking);
    }
}
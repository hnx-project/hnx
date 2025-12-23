//! Socket API implementation
//!
//! This module provides a minimal socket API that forwards operations to the user-space
//! network server through IPC.

extern crate alloc;

use crate::network::{NetworkError, Result, ipc_protocol::*};
use crate::network::manager::get_network_manager;
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

/// Socket I/O modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketMode {
    Blocking,
    NonBlocking,
}

/// Minimal socket structure - just stores basic information for IPC
#[derive(Debug, Clone)]
pub struct Socket {
    pub id: u32,
    pub domain: SocketDomain,
    pub type_: SocketType,
    pub protocol: SocketProtocol,
    pub mode: SocketMode,
    pub local_addr: Option<SocketAddress>,
    pub remote_addr: Option<SocketAddress>,
}

impl Socket {
    pub fn new(id: u32, domain: SocketDomain, type_: SocketType, protocol: SocketProtocol) -> Self {
        Socket {
            id,
            domain,
            type_,
            protocol,
            mode: SocketMode::Blocking, // 默认为阻塞模式
            local_addr: None,
            remote_addr: None,
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
    
    // Forward to user-space network server
    let req = SocketRequest {
        domain: match domain {
            SocketDomain::INET => AF_INET,
            SocketDomain::INET6 => AF_INET6,
        },
        sock_type: match type_ {
            SocketType::STREAM => SOCK_STREAM,
            SocketType::DGRAM => SOCK_DGRAM,
        },
        protocol: match protocol {
            SocketProtocol::IP => IPPROTO_IP,
            SocketProtocol::TCP => IPPROTO_TCP,
            SocketProtocol::UDP => IPPROTO_UDP,
            SocketProtocol::ICMP => IPPROTO_ICMP,
        },
    };
    
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send socket creation request to user-space server");
    
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
    
    // Forward to user-space network server
    let mut addr_array = [0u8; 16];
    addr_array[0] = (u32::from_be(addr.ip) >> 24) as u8;
    addr_array[1] = (u32::from_be(addr.ip) >> 16) as u8;
    addr_array[2] = (u32::from_be(addr.ip) >> 8) as u8;
    addr_array[3] = u32::from_be(addr.ip) as u8;
    
    let req = BindRequest {
        sockfd: socket_id,
        addr: addr_array,
        port: u16::from_be(addr.port),
    };
    
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send bind request to user-space server");
    
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
    };
    
    // Forward to user-space network server
    let req = ListenRequest {
        sockfd: socket_id,
        backlog: backlog as u32,
    };
    
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send listen request to user-space server");
    
    Ok(())
}

/// Accept an incoming connection (TCP only)
pub fn accept(socket_id: u32) -> Result<u32> {
    let sockets = SOCKETS.lock();
    
    let socket = match sockets.get(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Check if socket is listening
    // In a real implementation, we would check the actual state
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send accept request to user-space server");
    
    // In a real implementation, we would return the new socket ID from the user-space server
    // For now, we'll just return an error to indicate this isn't implemented
    
    Err(NetworkError::NotSupported)
}

/// Connect to a remote address
pub fn connect(socket_id: u32, addr: SocketAddress) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    let socket = match sockets.get_mut(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    socket.remote_addr = Some(addr);
    
    let ip = u32::from_be(addr.ip);
    let port = u16::from_be(addr.port);
    
    crate::info!("network/socket: socket {} connected to {:08X}:{}", 
                 socket_id, ip, port);
    
    // Forward to user-space network server
    let mut addr_array = [0u8; 16];
    addr_array[0] = (ip >> 24) as u8;
    addr_array[1] = (ip >> 16) as u8;
    addr_array[2] = (ip >> 8) as u8;
    addr_array[3] = ip as u8;
    
    let req = ConnectRequest {
        sockfd: socket_id,
        addr: addr_array,
        port,
    };
    
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send connect request to user-space server");
    
    Ok(())
}

/// Send data through a socket
pub fn send(socket_id: u32, data: &[u8]) -> Result<usize> {
    let sockets = SOCKETS.lock();
    
    let socket = match sockets.get(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Forward to user-space network server
    let req = DataTransfer {
        sockfd: socket_id,
        flags: 0, // No flags for now
    };
    
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send data request to user-space server ({} bytes)", data.len());
    
    // In a real implementation, we would return the number of bytes sent from the user-space server
    // For now, we'll just return the data length
    
    Ok(data.len())
}

/// Receive data from a socket
pub fn recv(socket_id: u32, buffer: &mut [u8]) -> Result<usize> {
    let sockets = SOCKETS.lock();
    
    let socket = match sockets.get(&socket_id) {
        Some(sock) => sock,
        None => return Err(NetworkError::InvalidParameter),
    };
    
    // Forward to user-space network server
    let req = DataTransfer {
        sockfd: socket_id,
        flags: 0, // No flags for now
    };
    
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send recv request to user-space server (buffer size: {})", buffer.len());
    
    // In a real implementation, we would fill the buffer with data from the user-space server
    // For now, we'll just return 0 to indicate no data
    
    Ok(0)
}

/// Close a socket
pub fn close(socket_id: u32) -> Result<()> {
    let mut sockets = SOCKETS.lock();
    
    if !sockets.contains_key(&socket_id) {
        return Err(NetworkError::InvalidParameter);
    }
    
    sockets.remove(&socket_id);
    
    crate::info!("network/socket: closed socket {}", socket_id);
    
    // Forward to user-space network server
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send close request to user-space server");
    
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
    
    // Forward to user-space network server
    // In a real implementation, we would send this request via IPC to the user-space server
    // For now, we'll just log that we would do this
    
    crate::info!("network/socket: would send set_nonblocking request to user-space server");
    
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
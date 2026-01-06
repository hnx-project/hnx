//! Socket manager for user-space network server

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

/// Socket information
#[derive(Debug, Clone)]
pub struct SocketInfo {
    pub fd: u32,
    pub domain: u32,
    pub sock_type: u32,
    pub protocol: u32,
    pub bound_addr: Option<(IpAddress, u16)>,
    pub connected: bool,
}

/// IP address representation
#[derive(Debug, Clone)]
pub enum IpAddress {
    Ipv4(u32),
    Ipv6([u8; 16]),
}

/// Socket manager responsible for managing socket resources
pub struct SocketManager {
    sockets: Mutex<BTreeMap<u32, SocketInfo>>,
    next_socket_fd: AtomicU32,
}

impl SocketManager {
    /// Create a new socket manager
    pub fn new() -> Self {
        Self {
            sockets: Mutex::new(BTreeMap::new()),
            next_socket_fd: AtomicU32::new(1000), // Start from 1000 to avoid conflicts
        }
    }

    /// Create a new socket
    pub fn create_socket(&self, domain: u32, sock_type: u32, protocol: u32) -> u32 {
        let sockfd = self.next_socket_fd.fetch_add(1, Ordering::SeqCst);
        
        let socket_info = SocketInfo {
            fd: sockfd,
            domain,
            sock_type,
            protocol,
            bound_addr: None,
            connected: false,
        };
        
        self.sockets.lock().insert(sockfd, socket_info);
        sockfd
    }

    /// Get socket information
    pub fn get_socket(&self, sockfd: u32) -> Option<SocketInfo> {
        self.sockets.lock().get(&sockfd).cloned()
    }

    /// Bind a socket to an address
    pub fn bind_socket(&self, sockfd: u32, addr: IpAddress, port: u16) -> Result<(), ()> {
        let mut sockets = self.sockets.lock();
        if let Some(socket) = sockets.get_mut(&sockfd) {
            socket.bound_addr = Some((addr, port));
            Ok(())
        } else {
            Err(())
        }
    }

    /// Close a socket
    pub fn close_socket(&self, sockfd: u32) -> Result<(), ()> {
        if self.sockets.lock().remove(&sockfd).is_some() {
            Ok(())
        } else {
            Err(())
        }
    }
}
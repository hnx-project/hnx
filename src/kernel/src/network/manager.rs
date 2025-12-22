//! Network Manager for HNX kernel
//!
//! This module implements the kernel-side network manager that routes packets
//! between hardware interfaces and user-space network servers through IPC.

extern crate alloc;

use crate::network::{NetworkError, ipc_protocol::*};
use crate::kernel_core::ipc::{endpoint_send_sync, IpcMessage, Priority};
use crate::drivers::ipc_protocol::EndpointId;
use spin::Mutex;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub id: u32,
    pub name: alloc::string::String,
    pub mac_address: [u8; 6],
    pub ip_addresses: alloc::vec::Vec<IpAddress>,
    pub mtu: u16,
}

/// IP address representation
#[derive(Debug, Clone)]
pub enum IpAddress {
    Ipv4(u32),
    Ipv6([u8; 16]),
}

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

/// Network manager responsible for routing packets between hardware and user-space
pub struct NetworkManager {
    network_server_epid: Option<EndpointId>,
    interfaces: BTreeMap<u32, NetworkInterface>,
    sockets: BTreeMap<u32, SocketInfo>,
    next_socket_fd: AtomicU32,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new() -> Self {
        Self {
            network_server_epid: None,
            interfaces: BTreeMap::new(),
            sockets: BTreeMap::new(),
            next_socket_fd: AtomicU32::new(1000), // Start from 1000 to avoid conflicts
        }
    }

    /// Register the network server endpoint
    pub fn register_network_server(&mut self, epid: EndpointId) -> Result<(), NetworkError> {
        self.network_server_epid = Some(epid);
        crate::info!("Network server registered with EPID {}", epid.0);
        Ok(())
    }
    
    /// Forward incoming packet from hardware to network server
    pub fn forward_packet_in(&self, interface_id: u32, packet: &[u8]) -> Result<(), NetworkError> {
        if let Some(epid) = self.network_server_epid {
            let forward = PacketForward {
                interface_id,
                packet_len: packet.len() as u32,
            };
            
            let mut serialized_data = forward.serialize();
            serialized_data.extend_from_slice(packet);
            
            // Create IPC message
            let msg = IpcMessage {
                src_pid: 0, // Kernel PID
                dst_epid: epid.0 as u32,
                op: NET_OP_PACKET_IN as u16,
                priority: Priority::Normal,
                data_len: serialized_data.len(),
                data: {
                    let mut arr = [0u8; 256];
                    let len = core::cmp::min(serialized_data.len(), arr.len());
                    arr[..len].copy_from_slice(&serialized_data[..len]);
                    arr
                },
                timestamp: crate::arch::timer::now_us(),
            };
            
            endpoint_send_sync(epid.0 as u32, msg)
                .map_err(|_| NetworkError::InvalidParameter)?;
            Ok(())
        } else {
            Err(NetworkError::InvalidParameter)
        }
    }
    
    /// Forward outgoing packet from network server to hardware
    pub fn forward_packet_out(&self, interface_id: u32, packet: &[u8]) -> Result<(), NetworkError> {
        // Send packet to hardware interface
        // This would interface with the device driver framework
        self.send_to_interface(interface_id, packet)
    }
    
    /// Create a new socket
    pub fn create_socket(&mut self, req: SocketRequest) -> Result<u32, NetworkError> {
        let sockfd = self.next_socket_fd.fetch_add(1, Ordering::SeqCst);
        
        let socket_info = SocketInfo {
            fd: sockfd,
            domain: req.domain,
            sock_type: req.sock_type,
            protocol: req.protocol,
            bound_addr: None,
            connected: false,
        };
        
        self.sockets.insert(sockfd, socket_info);
        Ok(sockfd)
    }
    
    /// Bind a socket to an address
    pub fn bind_socket(&mut self, req: BindRequest) -> Result<(), NetworkError> {
        if let Some(socket) = self.sockets.get_mut(&req.sockfd) {
            // Convert addr array to IpAddress based on domain
            let ip_addr = if socket.domain == AF_INET {
                IpAddress::Ipv4(u32::from_be_bytes([
                    req.addr[0], req.addr[1], req.addr[2], req.addr[3]
                ]))
            } else {
                // Handle IPv6
                IpAddress::Ipv6(req.addr)
            };
            
            socket.bound_addr = Some((ip_addr, req.port));
            Ok(())
        } else {
            Err(NetworkError::InvalidParameter)
        }
    }
    
    /// Send IPC request and wait for response
    pub fn send_ipc_request(&self, msg: IpcMessage) -> Result<IpcMessage, NetworkError> {
        if let Some(epid) = self.network_server_epid {
            endpoint_send_sync(epid.0 as u32, msg)
                .map_err(|_| NetworkError::InvalidParameter)?;
            // For simplicity, we're not actually waiting for a response in this implementation
            // A full implementation would need to handle async responses
            Ok(IpcMessage {
                src_pid: 0,
                dst_epid: 0,
                op: 0,
                priority: Priority::Normal,
                data_len: 0,
                data: [0; 256],
                timestamp: 0,
            })
        } else {
            Err(NetworkError::InvalidParameter)
        }
    }
    
    /// Send packet to network interface (stub implementation)
    fn send_to_interface(&self, _interface_id: u32, _packet: &[u8]) -> Result<(), NetworkError> {
        // This would interface with the actual device driver
        // For now, we'll just return success
        Ok(())
    }
}

/// Global network manager instance
static mut NETWORK_MANAGER_INSTANCE: Option<NetworkManager> = None;
static NETWORK_MANAGER_INIT: spin::Once<()> = spin::Once::new();

pub fn get_network_manager() -> &'static NetworkManager {
    unsafe {
        NETWORK_MANAGER_INIT.call_once(|| {
            NETWORK_MANAGER_INSTANCE = Some(NetworkManager::new());
        });
        NETWORK_MANAGER_INSTANCE.as_ref().unwrap()
    }
}

pub fn get_network_manager_mut() -> &'static mut NetworkManager {
    unsafe {
        NETWORK_MANAGER_INIT.call_once(|| {
            NETWORK_MANAGER_INSTANCE = Some(NetworkManager::new());
        });
        NETWORK_MANAGER_INSTANCE.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_manager_creation() {
        let manager = NetworkManager::new();
        assert!(manager.network_server_epid.is_none());
        assert_eq!(manager.sockets.len(), 0);
        assert_eq!(manager.interfaces.len(), 0);
        assert_eq!(manager.next_socket_fd.load(Ordering::SeqCst), 1000);
    }

    #[test]
    fn test_socket_creation() {
        let mut manager = NetworkManager::new();
        let req = SocketRequest {
            domain: AF_INET,
            sock_type: SOCK_STREAM,
            protocol: IPPROTO_TCP,
        };
        
        let result = manager.create_socket(req);
        assert!(result.is_ok());
        let sockfd = result.unwrap();
        assert!(sockfd >= 1000);
        
        // Check that socket was added to the map
        assert!(manager.sockets.contains_key(&sockfd));
    }
}
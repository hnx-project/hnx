//! Network Manager for HNX kernel
//!
//! This module implements the kernel-side network manager that routes packets
//! between hardware interfaces and user-space network servers through IPC.

extern crate alloc;

use crate::network::{NetworkError, ipc_protocol::*};
use crate::kernel_core::ipc::{endpoint_send_sync, IpcMessage, Priority};
use crate::drivers::ipc_protocol::EndpointId;

/// Network manager responsible for routing packets between hardware and user-space
/// This is a minimal implementation that only handles IPC forwarding
pub struct NetworkManager {
    network_server_epid: Option<EndpointId>,
}

impl NetworkManager {
    /// Create a new network manager
    pub const fn new() -> Self {
        Self {
            network_server_epid: None,
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
    
    /// Send packet to network interface (stub implementation)
    fn send_to_interface(&self, _interface_id: u32, _packet: &[u8]) -> Result<(), NetworkError> {
        // This would interface with the actual device driver
        // For now, we'll just return success
        Ok(())
    }
}

/// Global network manager instance
static NETWORK_MANAGER: spin::Mutex<NetworkManager> = spin::Mutex::new(NetworkManager::new());

pub fn get_network_manager() -> spin::MutexGuard<'static, NetworkManager> {
    NETWORK_MANAGER.lock()
}
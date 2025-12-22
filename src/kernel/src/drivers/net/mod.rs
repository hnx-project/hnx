//! Network device driver interface
//!
//! This module defines the interface for network device drivers to integrate
//! with the kernel network manager.

extern crate alloc;

use crate::network::manager::get_network_manager;
use crate::network::NetworkError;
use crate::drivers::ipc_protocol::{DeviceId, DriverId};

/// Trait for network drivers
pub trait NetworkDriver {
    /// Handle received packet from hardware
    fn handle_rx_packet(&mut self, packet: &[u8]);
    
    /// Send packet through hardware
    fn send_packet(&mut self, packet: &[u8]) -> Result<(), NetworkError>;
    
    /// Get device information
    fn get_device_info(&self) -> NetworkDeviceInfo;
    
    /// Handle device-specific control operations
    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<u64, NetworkError>;
}

/// Network device information structure
#[derive(Debug, Clone)]
pub struct NetworkDeviceInfo {
    pub device_id: DeviceId,
    pub driver_id: DriverId,
    pub mac_address: [u8; 6],
    pub mtu: u32,
    pub device_name: alloc::string::String,
    pub device_type: NetworkDeviceType,
    pub link_speed: u32, // in Mbps
    pub duplex: DuplexMode,
}

/// Types of network devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDeviceType {
    Ethernet,
    Wifi,
    Bluetooth,
    VirtioNet,
    UsbEthernet,
    Other,
}

/// Duplex modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplexMode {
    Half,
    Full,
}

/// Example implementation for Virtio network driver
pub struct VirtioNetDriver {
    pub interface_id: u32,
    pub device_info: NetworkDeviceInfo,
    // Other driver-specific fields would go here
}

impl NetworkDriver for VirtioNetDriver {
    /// Handle received packet from hardware
    fn handle_rx_packet(&mut self, packet: &[u8]) {
        // Forward received packet to network manager
        let interface_id = self.interface_id;
        if let Err(e) = get_network_manager().forward_packet_in(interface_id, packet) {
            crate::error!("Failed to forward packet to network manager: {:?}", e);
        }
    }
    
    /// Send packet through hardware
    fn send_packet(&mut self, packet: &[u8]) -> Result<(), NetworkError> {
        // Send packet through hardware
        self.send_raw_packet(packet)
    }
    
    /// Get device information
    fn get_device_info(&self) -> NetworkDeviceInfo {
        self.device_info.clone()
    }
    
    /// Handle device-specific control operations
    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<u64, NetworkError> {
        match cmd {
            // Get link status
            0x2001 => Ok(self.device_info.link_speed as u64),
            // Get MAC address
            0x2002 => {
                let mac = &self.device_info.mac_address;
                let mac_val = ((mac[0] as u64) << 40) | 
                             ((mac[1] as u64) << 32) | 
                             ((mac[2] as u64) << 24) | 
                             ((mac[3] as u64) << 16) | 
                             ((mac[4] as u64) << 8) | 
                             (mac[5] as u64);
                Ok(mac_val)
            },
            // Other commands...
            _ => Err(NetworkError::NotSupported),
        }
    }
}

impl VirtioNetDriver {
    /// Send raw packet through the hardware (stub implementation)
    fn send_raw_packet(&mut self, _packet: &[u8]) -> Result<(), NetworkError> {
        // In a real implementation, this would interact with the virtio device
        // For now, we'll just return success
        Ok(())
    }
    
    /// Create a new Virtio network driver instance
    pub fn new(interface_id: u32, device_info: NetworkDeviceInfo) -> Self {
        Self {
            interface_id,
            device_info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::ipc_protocol::{DeviceId, DriverId};

    #[test]
    fn test_virtio_net_driver_creation() {
        let device_info = NetworkDeviceInfo {
            device_id: DeviceId(1),
            driver_id: DriverId(1),
            mac_address: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
            mtu: 1500,
            device_name: "virtio-net".into(),
            device_type: NetworkDeviceType::VirtioNet,
            link_speed: 1000,
            duplex: DuplexMode::Full,
        };
        
        let driver = VirtioNetDriver::new(1, device_info.clone());
        assert_eq!(driver.interface_id, 1);
        assert_eq!(driver.device_info.mac_address, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }
}
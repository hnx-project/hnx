//! Device Manager
//!
//! Central device manager responsible for:
//! - Managing device registration and driver assignment
//! - Routing interrupts to appropriate user-space drivers
//! - Handling MMIO region sharing with drivers
//! - Managing DMA buffer allocation for drivers
//! - Supporting device hotplug detection

extern crate alloc;

use crate::drivers::ipc_protocol::*;
use crate::drivers::gic;
use crate::core::ipc::{endpoint_send_sync, IpcMessage, Priority};
use shared::sync::mutex::Mutex;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::vec::Vec;

/// Global device manager instance
pub static DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager::new());

/// Information about a registered device
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub device_type: DeviceType,
    pub name: alloc::string::String,
    pub driver: Option<DriverId>,
    pub mmio_regions: Vec<MmioRegion>,
    pub irqs: Vec<u32>,
    pub hotplug_capable: bool,
    pub present: bool,
}

impl DeviceInfo {
    /// Create a new device info structure
    pub fn new(
        device_type: DeviceType,
        name: alloc::string::String,
        mmio_regions: Vec<MmioRegion>,
        irqs: Vec<u32>,
    ) -> Self {
        Self {
            id: DeviceId(0), // Will be set by device manager
            device_type,
            name,
            driver: None,
            mmio_regions,
            irqs,
            hotplug_capable: false,
            present: true, // By default, devices are present
        }
    }
    
    /// Create a new hotplug-capable device info structure
    pub fn new_hotplug(
        device_type: DeviceType,
        name: alloc::string::String,
        mmio_regions: Vec<MmioRegion>,
        irqs: Vec<u32>,
    ) -> Self {
        Self {
            id: DeviceId(0), // Will be set by device manager
            device_type,
            name,
            driver: None,
            mmio_regions,
            irqs,
            hotplug_capable: true,
            present: false, // Hotplug devices start as not present
        }
    }
}

/// Hotplug event types
#[derive(Debug, Clone)]
pub enum HotplugEvent {
    DeviceAdded(DeviceId),
    DeviceRemoved(DeviceId),
}

/// Information about a registered driver
#[derive(Debug, Clone)]
pub struct DriverInfo {
    pub id: DriverId,
    pub endpoint: EndpointId,
    pub supported_devices: Vec<DeviceType>,
    pub registered_devices: Vec<DeviceId>,
}

/// MMIO region information
#[derive(Debug, Clone)]
pub struct MmioRegion {
    pub physical_address: u64,
    pub size: usize,
    pub capability: Capability,
}

/// Capability for accessing hardware resources
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: u64,
    pub cap_type: CapabilityType,
}

/// Types of capabilities
#[derive(Debug, Clone)]
pub enum CapabilityType {
    Mmio {
        physical_address: u64,
        size: usize,
    },
    DmaBuffer {
        physical_address: u64,
        virtual_address: usize,  // Kernel virtual address
        size: usize,
    },
}

impl Capability {
    /// Create a new MMIO capability
    pub fn new_mmio(physical_address: u64, size: usize) -> Self {
        Self {
            id: generate_capability_id(),
            cap_type: CapabilityType::Mmio { physical_address, size },
        }
    }

    /// Create a new DMA buffer capability
    pub fn new_dma_buffer(physical_address: u64, virtual_address: usize, size: usize) -> Self {
        Self {
            id: generate_capability_id(),
            cap_type: CapabilityType::DmaBuffer { physical_address, virtual_address, size },
        }
    }

    /// Create an invalid capability
    pub fn invalid() -> Self {
        Self {
            id: 0,
            cap_type: CapabilityType::Mmio { physical_address: 0, size: 0 },
        }
    }
    
    /// Get the capability ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Global capability ID generator
static NEXT_CAPABILITY_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a unique capability ID
fn generate_capability_id() -> u64 {
    NEXT_CAPABILITY_ID.fetch_add(1, Ordering::SeqCst)
}

/// Main device manager structure
pub struct DeviceManager {
    devices: BTreeMap<DeviceId, DeviceInfo>,
    drivers: BTreeMap<DriverId, DriverInfo>,
    irq_routing: BTreeMap<u32, EndpointId>,
    mmio_regions: BTreeMap<u64, MmioRegion>,
    next_device_id: AtomicU64,
    hotplug_callbacks: Vec<fn(HotplugEvent)>,
}

impl DeviceManager {
    /// Create a new device manager
    pub const fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            drivers: BTreeMap::new(),
            irq_routing: BTreeMap::new(),
            mmio_regions: BTreeMap::new(),
            next_device_id: AtomicU64::new(1),
            hotplug_callbacks: Vec::new(),
        }
    }

    /// Register a new driver
    pub fn register_driver(&mut self, reg: DeviceRegistration, epid: EndpointId) -> Result<DriverId, DriverError> {
        // Create driver entry
        let driver_id = DriverId(self.next_device_id.fetch_add(1, Ordering::SeqCst));
        
        let driver_info = DriverInfo {
            id: driver_id,
            endpoint: epid,
            supported_devices: alloc::vec![reg.device_type],
            registered_devices: Vec::new(),
        };
        
        self.drivers.insert(driver_id, driver_info);
        crate::info!("Driver registered: ID={:?}, EPID={:?}", driver_id, epid);
        Ok(driver_id)
    }
    
    /// Request an IRQ for a driver
    pub fn request_irq(&mut self, req: IrqRequest) -> Result<(), DriverError> {
        // Check if IRQ is already claimed
        if self.irq_routing.contains_key(&req.irq_number) {
            return Err(DriverError::IrqAlreadyClaimed);
        }
        
        // Register IRQ routing
        self.irq_routing.insert(req.irq_number, req.driver_epid);
        
        // Enable interrupt in GIC
        gic::enable_irq(req.irq_number);
        
        crate::info!("IRQ {} routed to driver EPID {:?}", req.irq_number, req.driver_epid);
        Ok(())
    }
    
    /// Request MMIO mapping for a driver
    pub fn request_mmio_mapping(&mut self, req: MmioRequest) -> Result<Capability, DriverError> {
        // Create capability for MMIO region
        let cap = Capability::new_mmio(req.physical_address, req.size);
        
        // Store mapping
        let region = MmioRegion {
            physical_address: req.physical_address,
            size: req.size,
            capability: cap.clone(),
        };
        
        self.mmio_regions.insert(req.physical_address, region);
        
        // TODO: Actually grant capability to driver through capability manager
        // For now, we'll just log it
        
        crate::info!("MMIO mapping requested: 0x{:x} size={}", req.physical_address, req.size);
        Ok(cap)
    }
    
    /// Forward an interrupt to the appropriate user-space driver
    pub fn forward_interrupt(&self, irq: u32) {
        // Look up driver endpoint for this IRQ
        if let Some(&epid) = self.irq_routing.get(&irq) {
            // Send interrupt notification to driver
            let notification = InterruptNotification {
                irq_number: irq,
                timestamp: get_timestamp(),
            };
            
            // Create a simple byte array for the notification data
            let mut data = [0u8; 256]; // Fixed size to match IpcMessage
            // First 4 bytes for irq_number, next 8 bytes for timestamp
            data[0..4].copy_from_slice(&(notification.irq_number as u32).to_le_bytes());
            data[4..12].copy_from_slice(&notification.timestamp.to_le_bytes());
            
            // Create IPC message
            let msg = IpcMessage {
                msg_id: 0, // Will be filled by IPC layer
                src_pid: 0, // Kernel PID
                dst_epid: epid.0 as u32,
                op: DEV_OP_INTERRUPT as u16,
                priority: Priority::High,
                data_len: 12, // Only the first 12 bytes are meaningful
                data,
                timestamp: get_timestamp(),
            };
            
            if let Err(e) = endpoint_send_sync(epid.0 as u32, msg) {
                crate::error!("Failed to forward IRQ {} to driver: {:?}", irq, e);
            } else {
                crate::debug!("Forwarded IRQ {} to driver EPID {:?}", irq, epid);
            }
        } else {
            crate::warn!("No driver registered for IRQ {}", irq);
        }
    }
    
    /// Check if an IRQ should be handled by user-space
    pub fn is_user_space_irq(&self, irq: u32) -> bool {
        self.irq_routing.contains_key(&irq)
    }
    
    /// Register a hotplug callback
    pub fn register_hotplug_callback(&mut self, callback: fn(HotplugEvent)) {
        self.hotplug_callbacks.push(callback);
    }
    
    /// Simulate device hotplug detection
    /// In a real implementation, this would be called by interrupt handlers
    /// or polling mechanisms that detect device insertion/removal
    pub fn detect_hotplug_event(&mut self, device_id: DeviceId, present: bool) {
        if let Some(device_info) = self.devices.get_mut(&device_id) {
            let old_present = device_info.present;
            device_info.present = present;
            
            // Only trigger events if state actually changed
            if old_present != present && device_info.hotplug_capable {
                let event = if present {
                    HotplugEvent::DeviceAdded(device_id)
                } else {
                    HotplugEvent::DeviceRemoved(device_id)
                };
                
                // Notify all registered callbacks
                for callback in &self.hotplug_callbacks {
                    callback(event.clone());
                }
                
                crate::info!("Hotplug event: {:?} for device {:?}", event, device_id);
            }
        }
    }
    
    /// Register a hotplug-capable device
    pub fn register_hotplug_device(&mut self, mut device_info: DeviceInfo) -> DeviceId {
        device_info.hotplug_capable = true;
        device_info.present = false; // Initially not present
        let device_id = DeviceId(self.next_device_id.fetch_add(1, Ordering::SeqCst));
        device_info.id = device_id;
        self.devices.insert(device_id, device_info);
        device_id
    }
    
    /// Check if a device is currently present
    pub fn is_device_present(&self, device_id: DeviceId) -> bool {
        if let Some(device_info) = self.devices.get(&device_id) {
            device_info.present
        } else {
            false
        }
    }
}

/// Get current timestamp in microseconds
fn get_timestamp() -> u64 {
    // TODO: Implement proper timestamp retrieval
    // For now, we'll just return a dummy value
    0
}

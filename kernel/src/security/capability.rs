//! Capability Management
//!
//! This module implements capability-based security for accessing system resources.
//! Capabilities are unforgeable tokens that grant specific rights to processes.

#![no_std]

extern crate alloc;

use shared::sync::mutex::Mutex;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

/// Global capability manager instance
pub static CAPABILITY_MANAGER: Mutex<CapabilityManager> = Mutex::new(CapabilityManager::new());

/// Unique identifier for a capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(u32);

/// Types of capabilities
#[derive(Debug, Clone)]
pub enum CapabilityType {
    /// MMIO region access
    Mmio {
        physical_address: u64,
        size: usize,
    },
    /// DMA buffer access
    DmaBuffer {
        physical_address: u64,
        virtual_address: usize,
        size: usize,
    },
    /// Endpoint communication
    Endpoint {
        endpoint_id: u32,
        rights: u8,
    },
    /// File access
    File {
        path: alloc::string::String,
        rights: u8,
    },
}

/// Rights that can be granted by capabilities
pub mod rights {
    /// Right to send messages
    pub const SEND: u8 = 0x1;
    /// Right to receive messages
    pub const RECV: u8 = 0x2;
    /// Right to manage resources
    pub const MANAGE: u8 = 0x4;
    /// Right to read data
    pub const READ: u8 = 0x8;
    /// Right to write data
    pub const WRITE: u8 = 0x10;
}

/// A capability granting access to a resource
#[derive(Debug, Clone)]
pub struct Capability {
    id: CapabilityId,
    cap_type: CapabilityType,
}

impl Capability {
    /// Create a new MMIO capability
    pub fn new_mmio(physical_address: u64, size: usize) -> Self {
        Self {
            id: CapabilityManager::generate_id(),
            cap_type: CapabilityType::Mmio { physical_address, size },
        }
    }

    /// Create a new DMA buffer capability
    pub fn new_dma_buffer(physical_address: u64, virtual_address: usize, size: usize) -> Self {
        Self {
            id: CapabilityManager::generate_id(),
            cap_type: CapabilityType::DmaBuffer { physical_address, virtual_address, size },
        }
    }

    /// Get the capability ID
    pub fn id(&self) -> CapabilityId {
        self.id
    }

    /// Get the capability type
    pub fn cap_type(&self) -> &CapabilityType {
        &self.cap_type
    }
}

/// Manages capabilities in the system
pub struct CapabilityManager {
    capabilities: BTreeMap<CapabilityId, Capability>,
    next_id: AtomicU32,
}

impl CapabilityManager {
    /// Create a new capability manager
    pub const fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            next_id: AtomicU32::new(1),
        }
    }

    /// Generate a new unique capability ID
    pub fn generate_id() -> CapabilityId {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        CapabilityId(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }

    /// Create and store a new capability
    pub fn create_capability(&mut self, cap_type: CapabilityType) -> CapabilityId {
        let capability = Capability {
            id: Self::generate_id(),
            cap_type,
        };
        let id = capability.id;
        self.capabilities.insert(id, capability);
        id
    }

    /// Grant a capability to a process (endpoint)
    pub fn grant_capability(&mut self, _endpoint_id: u64, capability: Capability) -> Result<(), ()> {
        // In a full implementation, we would track which endpoints have which capabilities
        // For now, we'll just store the capability
        self.capabilities.insert(capability.id, capability);
        Ok(())
    }

    /// Validate a capability
    pub fn validate_capability(&self, id: CapabilityId) -> Option<&Capability> {
        self.capabilities.get(&id)
    }

    /// Revoke a capability
    pub fn revoke_capability(&mut self, id: CapabilityId) -> Option<Capability> {
        self.capabilities.remove(&id)
    }
}

/// Validate that a capability grants the requested rights
pub fn validate_capability(id: CapabilityId, required_rights: u8) -> bool {
    let manager = CAPABILITY_MANAGER.lock();
    if let Some(cap) = manager.validate_capability(id) {
        match &cap.cap_type {
            CapabilityType::Endpoint { rights, .. } => (rights & required_rights) == required_rights,
            CapabilityType::File { rights, .. } => (rights & required_rights) == required_rights,
            // MMIO and DMA capabilities implicitly grant read/write rights
            CapabilityType::Mmio { .. } | CapabilityType::DmaBuffer { .. } => true,
        }
    } else {
        false
    }
}
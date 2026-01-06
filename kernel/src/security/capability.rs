//! Capability Management
//!
//! This module implements capability-based security for accessing system resources.
//! Capabilities are unforgeable tokens that grant specific rights to processes.

#![no_std]

extern crate alloc;

use shared::sync::mutex::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of capabilities in the system (for fixed-size arrays if used)
const MAX_CAPABILITIES: usize = 256;

/// Unique identifier for a capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(pub u32);

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
    File {
        path: String,
        rights: u8,
    },
    /// Generic Object
    Object {
        object_id: u32,
        rights: u32,
    },
}

/// Rights that can be granted by capabilities
pub mod rights {
    /// Right to send messages
    pub const SEND: u8 = 0x1;
    /// Right to receive messages
    pub const RECV: u8 = 0x2;
    /// Right to manage resources (e.g., create/destroy objects)
    pub const MANAGE: u8 = 0x4;
    /// Right to read data
    pub const READ: u8 = 0x8;
    /// Right to write data
    pub const WRITE: u8 = 0x10;
    /// Right to execute code
    pub const EXECUTE: u8 = 0x20;
    
    /// All rights combined
    pub const ALL: u8 = SEND | RECV | MANAGE | READ | WRITE | EXECUTE;
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
            id: CapabilityId(0), // ID will be set by CapabilityManager
            cap_type: CapabilityType::Mmio { physical_address, size },
        }
    }

    /// Create a new DMA buffer capability
    pub fn new_dma_buffer(physical_address: u64, virtual_address: usize, size: usize) -> Self {
        Self {
            id: CapabilityId(0), // ID will be set by CapabilityManager
            cap_type: CapabilityType::DmaBuffer { physical_address, virtual_address, size },
        }
    }

    /// Create a new Endpoint capability
    pub fn new_endpoint(endpoint_id: u32, rights: u8) -> Self {
        Self {
            id: CapabilityId(0),
            cap_type: CapabilityType::Endpoint { endpoint_id, rights },
        }
    }

    /// Create a new File capability
    pub fn new_file(path: String, rights: u8) -> Self {
        Self {
            id: CapabilityId(0),
            cap_type: CapabilityType::File { path, rights },
        }
    }

    /// Create a new generic Object capability
    pub fn new_object(object_id: u32, rights: u32) -> Self {
        Self {
            id: CapabilityId(0),
            cap_type: CapabilityType::Object { object_id, rights },
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
    next_capability_id: AtomicU32,
}

impl CapabilityManager {
    /// Create a new capability manager
    pub const fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            next_capability_id: AtomicU32::new(1),
        }
    }

    /// Generate a new unique capability ID
    pub fn generate_id(&self) -> CapabilityId {
        CapabilityId(self.next_capability_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Create and store a new capability
    pub fn create_capability(&mut self, cap_type: CapabilityType) -> CapabilityId {
        let capability = Capability {
            id: self.generate_id(),
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

    /// Allocates a new capability with the specified rights to an object (from security/mod.rs)
    pub fn allocate_capability_by_object(&mut self, object_id: u32, rights: u32) -> Option<u32> {
        let cap_type = CapabilityType::Object { object_id, rights };
        Some(self.create_capability(cap_type).0)
    }

    /// Validates that a capability grants the requested rights to an object (from security/mod.rs)
    pub fn validate_capability_by_object(&self, cap_id: CapabilityId, object_id: u32, required_rights: u32) -> bool {
        if let Some(cap) = self.capabilities.get(&cap_id) {
            match &cap.cap_type {
                CapabilityType::Object { object_id: obj_id, rights: obj_rights } => {
                    *obj_id == object_id && ((*obj_rights & required_rights) == required_rights)
                },
                _ => false, // Not an object capability
            }
        } else {
            false
        }
    }

    /// Revokes a capability, making it invalid for future use (from security/mod.rs)
    pub fn revoke_capability_by_object(&mut self, cap_id: CapabilityId) -> bool {
        self.capabilities.remove(&cap_id).is_some()
    }

    /// Initialize the capability manager
    pub fn init(&mut self) {
        // Currently nothing to initialize, but kept for consistency
        // with other manager singletons.
    }
}

impl CapabilityManager {
    /// Validate that a capability grants the requested rights
    pub fn validate_capability_with_rights(&self, id: CapabilityId, required_rights: u8) -> bool {
        if let Some(cap) = self.capabilities.get(&id) {
            match &cap.cap_type {
                CapabilityType::Endpoint { rights, .. } => (rights & required_rights) == required_rights,
                CapabilityType::File { rights, .. } => (rights & required_rights) == required_rights,
                CapabilityType::Object { rights: obj_rights, .. } => ((*obj_rights as u8) & required_rights) == required_rights,
                // MMIO and DMA capabilities implicitly grant read/write rights
                CapabilityType::Mmio { .. } | CapabilityType::DmaBuffer { .. } => true,
            }
        } else {
            false
        }
    }
}

/// 全局能力管理器单例实例
///
/// # 安全性
///
/// `static mut` 是不安全的，但我们在初始化时只对其进行一次写操作，
/// 并且之后的所有访问都通过安全的 `get_capability_manager()` 函数进行，因此这种用法是可控的。
#[used]
static mut CAPABILITY_MANAGER: Option<Mutex<CapabilityManager>> = None;

/// 初始化全局能力管理器单例实例
pub fn init_capability_manager() {
    crate::info!("capability: initializing global capability manager singleton");
    let manager = Mutex::new(CapabilityManager::new());
    unsafe {
        CAPABILITY_MANAGER = Some(manager);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
    crate::info!("capability: global capability manager singleton initialized");
}

/// 获取对全局能力管理器单例实例的安全引用
pub fn get_capability_manager() -> &'static Mutex<CapabilityManager> {
    unsafe {
        CAPABILITY_MANAGER.as_ref().expect("Capability manager has not been initialized")
    }
}


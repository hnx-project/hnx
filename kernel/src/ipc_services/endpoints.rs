//! Well-Known Service Endpoints
//!
//! This module defines the well-known endpoint IDs for system services.
//! These endpoints are created by the kernel at boot time and used by
//! user space services to receive requests.

/// Service endpoint identifier
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum ServiceEndpoint {
    /// Virtual File System service
    VFS = 1,
    /// Network stack service
    Network = 2,
    /// Dynamic linker/loader service
    Loader = 3,
    /// Block device service
    BlockDevice = 4,
    /// Character device service
    CharDevice = 5,
    /// Graphics/Display service
    Display = 6,
    /// Audio service
    Audio = 7,
    /// Input device service (keyboard, mouse, etc.)
    Input = 8,
}

impl ServiceEndpoint {
    /// Get the endpoint ID as u32
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
    
    /// Get service name for logging
    pub const fn name(self) -> &'static str {
        match self {
            ServiceEndpoint::VFS => "vfs",
            ServiceEndpoint::Network => "network",
            ServiceEndpoint::Loader => "loader",
            ServiceEndpoint::BlockDevice => "block",
            ServiceEndpoint::CharDevice => "char",
            ServiceEndpoint::Display => "display",
            ServiceEndpoint::Audio => "audio",
            ServiceEndpoint::Input => "input",
        }
    }
    
    /// Convert from u32 endpoint ID
    pub const fn from_u32(id: u32) -> Option<Self> {
        match id {
            1 => Some(ServiceEndpoint::VFS),
            2 => Some(ServiceEndpoint::Network),
            3 => Some(ServiceEndpoint::Loader),
            4 => Some(ServiceEndpoint::BlockDevice),
            5 => Some(ServiceEndpoint::CharDevice),
            6 => Some(ServiceEndpoint::Display),
            7 => Some(ServiceEndpoint::Audio),
            8 => Some(ServiceEndpoint::Input),
            _ => None,
        }
    }
}

/// Well-known service endpoints (for easy access)
pub struct WellKnownServices;

impl WellKnownServices {
    pub const VFS_EPID: u32 = ServiceEndpoint::VFS.as_u32();
    pub const NETWORK_EPID: u32 = ServiceEndpoint::Network.as_u32();
    pub const LOADER_EPID: u32 = ServiceEndpoint::Loader.as_u32();
    pub const BLOCK_EPID: u32 = ServiceEndpoint::BlockDevice.as_u32();
}

/// Global instance for convenience
pub const WELL_KNOWN_SERVICES: WellKnownServices = WellKnownServices;

/// Initialize well-known service endpoints
///
/// This is called during kernel boot to create the well-known endpoints
/// that user space services will use to receive requests.
pub fn init_service_endpoints() -> Result<(), ()> {
    use crate::core::ipc::{endpoint_create, EndpointCapabilities};
    
    crate::info!("ipc_services: creating well-known service endpoints");
    
    // Create capabilities that allow any process to send to these endpoints
    // but only the service itself can receive
    let caps = EndpointCapabilities {
        read: true,   // Service can receive
        write: true,  // Any process can send
        admin: false, // No admin rights by default
    };
    
    // Create endpoints for each service
    let services = [
        ServiceEndpoint::VFS,
        ServiceEndpoint::Network,
        ServiceEndpoint::Loader,
        ServiceEndpoint::BlockDevice,
    ];
    
    for service in &services {
        match endpoint_create(caps) {
            Ok(epid) => {
                if epid == service.as_u32() {
                    crate::info!("ipc_services: created endpoint {} for {}", epid, service.name());
                } else {
                    crate::warn!(
                        "ipc_services: endpoint ID mismatch for {} (expected {}, got {})",
                        service.name(),
                        service.as_u32(),
                        epid
                    );
                }
            }
            Err(_) => {
                crate::error!("ipc_services: failed to create endpoint for {}", service.name());
                return Err(());
            }
        }
    }
    
    crate::info!("ipc_services: all well-known endpoints created");
    Ok(())
}

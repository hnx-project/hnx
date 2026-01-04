//! Driver Syscalls
//!
//! This module implements syscalls for user-space device drivers to interact
//! with the kernel's device manager.

#![no_std]

extern crate alloc;

use crate::drivers::ipc_protocol::*;
use crate::drivers::device_manager::DEVICE_MANAGER;
use crate::process::syscall::{SysResult, user_range_ok};
use crate::security::capability::CAPABILITY_MANAGER;
use crate::memory::dma::DMA_ALLOCATOR;

/// Driver syscall numbers
pub const HNX_SYS_DRIVER_REGISTER: usize = 2001;
pub const HNX_SYS_DRIVER_REQUEST_IRQ: usize = 2002;
pub const HNX_SYS_DRIVER_MAP_MMIO: usize = 2003;
pub const HNX_SYS_DRIVER_DMA_ALLOC: usize = 2004;

/// Register a driver with the device manager
pub fn sys_driver_register(reg_ptr: usize, reg_size: usize) -> SysResult {
    // Validate user buffer
    if !user_range_ok(reg_ptr, reg_size, false) {
        return -1; // InvalidAddress
    }

    // For now, we'll just assume the data is valid
    // In a real implementation, we would deserialize the data properly
    
    // Get current process endpoint
    let current_epid = EndpointId(crate::core::scheduler::current_pid());

    // Create a dummy registration for now
    let registration = DeviceRegistration {
        device_type: DeviceType::Other,
        vendor_id: 0,
        device_id: 0,
        name: [0; 32],
    };

    // Register with device manager
    let result = DEVICE_MANAGER.lock().register_driver(registration, current_epid);
    
    match result {
        Ok(driver_id) => driver_id.0 as SysResult,
        Err(_) => -1, // Registration failed
    }
}

/// Request an IRQ for a driver
pub fn sys_driver_request_irq(irq_num: u32) -> SysResult {
    let current_epid = EndpointId(crate::core::scheduler::current_pid());

    let req = IrqRequest {
        irq_number: irq_num,
        driver_epid: current_epid,
    };

    match DEVICE_MANAGER.lock().request_irq(req) {
        Ok(()) => 0,
        Err(_) => -1, // Request failed
    }
}

/// Map an MMIO region for a driver
pub fn sys_driver_map_mmio(phys_addr: u64, size: usize) -> SysResult {
    let current_epid = EndpointId(crate::core::scheduler::current_pid());

    let req = MmioRequest {
        physical_address: phys_addr,
        size,
        driver_epid: current_epid,
    };

    let result = DEVICE_MANAGER.lock().request_mmio_mapping(req);
    
    match result {
        Ok(capability) => {
            // Convert device capability to security capability
            let security_capability = crate::security::capability::Capability::new_mmio(
                match &capability.cap_type {
                    crate::drivers::device_manager::CapabilityType::Mmio { physical_address, size } => *physical_address,
                    _ => 0,
                },
                match &capability.cap_type {
                    crate::drivers::device_manager::CapabilityType::Mmio { physical_address: _, size } => *size,
                    _ => 0,
                }
            );
            
            // Grant capability to the requesting process
            if CAPABILITY_MANAGER.lock().grant_capability(current_epid.0, security_capability).is_ok() {
                // Return capability ID to user
                capability.id as SysResult
            } else {
                -1 // Failed to grant capability
            }
        },
        Err(_) => -1, // Mapping failed
    }
}

/// Allocate DMA buffer for a driver
pub fn sys_driver_dma_alloc(size: usize, alignment: usize) -> SysResult {
    let current_epid = EndpointId(crate::core::scheduler::current_pid());

    // Allocate DMA buffer
    let result = DMA_ALLOCATOR.lock().allocate_dma_buffer(size, alignment);
    
    match result {
        Ok((phys_addr, capability)) => {
            // Convert DMA capability to security capability
            let security_capability = crate::security::capability::Capability::new_dma_buffer(
                match &capability.cap_type {
                    crate::memory::dma::CapabilityType::Mmio { .. } => 0, // Should not happen
                    crate::memory::dma::CapabilityType::DmaBuffer { physical_address, virtual_address: _, size: _ } => *physical_address,
                },
                match &capability.cap_type {
                    crate::memory::dma::CapabilityType::Mmio { .. } => 0, // Should not happen
                    crate::memory::dma::CapabilityType::DmaBuffer { physical_address: _, virtual_address, size: _ } => *virtual_address,
                },
                match &capability.cap_type {
                    crate::memory::dma::CapabilityType::Mmio { .. } => 0, // Should not happen
                    crate::memory::dma::CapabilityType::DmaBuffer { physical_address: _, virtual_address: _, size } => *size,
                }
            );
            
            // Grant capability to the requesting process
            if CAPABILITY_MANAGER.lock().grant_capability(current_epid.0, security_capability).is_ok() {
                // Return physical address to user (in a real implementation, we'd return a handle)
                phys_addr as SysResult
            } else {
                -1 // Failed to grant capability
            }
        },
        Err(_) => -1, // Allocation failed
    }
}
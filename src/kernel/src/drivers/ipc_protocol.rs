//! Device Driver IPC Protocol
//!
//! This module defines the standardized IPC messages and data structures
//! used for communication between user-space device drivers and the kernel's
//! device manager.

/// Device manager opcodes
pub const DEV_OP_REGISTER_DRIVER: u64 = 1;
pub const DEV_OP_REQUEST_IRQ: u64 = 2;
pub const DEV_OP_MMIO_MAP: u64 = 3;
pub const DEV_OP_DMA_ALLOC: u64 = 4;
pub const DEV_OP_INTERRUPT: u64 = 5;  // Kernel → Driver
pub const DEV_OP_IO_READ: u64 = 6;
pub const DEV_OP_IO_WRITE: u64 = 7;

/// Unique identifier for a device
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceId(pub u64);

/// Unique identifier for a driver
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DriverId(pub u64);

/// Unique identifier for an endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EndpointId(pub u64);

/// Types of devices supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block,
    Character,
    Network,
    Graphics,
    Usb,
    Gpu,
    Audio,
    Storage,
    Input,
    Other,
}

/// Device registration request from a driver
#[derive(Debug, Clone)]
pub struct DeviceRegistration {
    pub device_type: DeviceType,
    pub vendor_id: u32,
    pub device_id: u32,
    pub name: [u8; 32],
}

/// IRQ request from a driver
#[derive(Debug, Clone)]
pub struct IrqRequest {
    pub irq_number: u32,
    pub driver_epid: EndpointId,
}

/// MMIO mapping request from a driver
#[derive(Debug, Clone)]
pub struct MmioRequest {
    pub physical_address: u64,
    pub size: usize,
    pub driver_epid: EndpointId,
}

/// DMA allocation request from a driver
#[derive(Debug, Clone)]
pub struct DmaAllocRequest {
    pub size: usize,
    pub alignment: usize,
    pub driver_epid: EndpointId,
}

/// Interrupt notification sent from kernel to driver
#[derive(Debug, Clone)]
pub struct InterruptNotification {
    pub irq_number: u32,
    pub timestamp: u64,
}

/// Error types for driver operations
#[derive(Debug, Clone)]
pub enum DriverError {
    InvalidArgument,
    OutOfMemory,
    PermissionDenied,
    DeviceNotFound,
    DriverNotRegistered,
    IrqAlreadyClaimed,
    IoError,
}
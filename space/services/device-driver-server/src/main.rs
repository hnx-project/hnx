//! User-space Device Driver Server Implementation
//!
//! This module implements a user-space device driver server that communicates with the kernel
//! through IPC to handle device operations.
//!
//! The server handles:
//! - Device registration
//! - IRQ handling
//! - MMIO region mapping
//! - DMA buffer allocation
//! - Device-specific operations
//!
//! Communication with kernel is through standardized IPC protocol defined in
//! kernel/src/drivers/ipc_protocol.rs

#![no_std]

extern crate alloc;

// Import local modules
mod drivers;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

// Import system call interfaces
use hnx_libc::syscalls::{sys_channel_create, sys_channel_read, sys_channel_write};
use alloc::string::String;

// Define endpoint ID type for compatibility
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndpointId(pub u32);

/// Device manager opcodes (must match kernel definitions)
const DEV_OP_REGISTER_DRIVER: u64 = 1;
const DEV_OP_REQUEST_IRQ: u64 = 2;
const DEV_OP_MMIO_MAP: u64 = 3;
const DEV_OP_DMA_ALLOC: u64 = 4;
const DEV_OP_INTERRUPT: u64 = 5;  // Kernel → Driver
const DEV_OP_IO_READ: u64 = 6;
const DEV_OP_IO_WRITE: u64 = 7;
const DEV_OP_GIC_INIT: u64 = 8;
const DEV_OP_GIC_ENABLE_IRQ: u64 = 9;
const DEV_OP_GIC_DISABLE_IRQ: u64 = 10;
const DEV_OP_GIC_SET_PRIORITY: u64 = 11;

/// Types of devices supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceType {
    Block = 0,
    Character = 1,
    Network = 2,
    Graphics = 3,
    Usb = 4,
    Gpu = 5,
    Audio = 6,
    Storage = 7,
    Input = 8,
    Other = 9,
}

/// Unique identifier for a device
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DeviceId(u64);

/// Unique identifier for a driver
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DriverId(u64);

/// Device registration request from a driver
#[derive(Debug, Clone)]
struct DeviceRegistration {
    device_type: DeviceType,
    vendor_id: u32,
    device_id: u32,
    name: [u8; 32],
}

/// IRQ request from a driver
#[derive(Debug, Clone)]
struct IrqRequest {
    irq_number: u32,
    driver_epid: EndpointId,
}

/// MMIO mapping request from a driver
#[derive(Debug, Clone)]
struct MmioRequest {
    physical_address: u64,
    size: usize,
    driver_epid: EndpointId,
}

/// DMA allocation request from a driver
#[derive(Debug, Clone)]
struct DmaAllocRequest {
    size: usize,
    alignment: usize,
    driver_epid: EndpointId,
}

/// Interrupt notification sent from kernel to driver
#[derive(Debug, Clone)]
struct InterruptNotification {
    irq_number: u32,
    timestamp: u64,
}

/// Device information
#[derive(Debug, Clone)]
struct DeviceInfo {
    id: DeviceId,
    device_type: DeviceType,
    vendor_id: u32,
    device_id: u32,
    name: alloc::string::String,
    mmio_regions: alloc::vec::Vec<MmioRegion>,
    irqs: alloc::vec::Vec<u32>,
}

/// MMIO region information
#[derive(Debug, Clone)]
struct MmioRegion {
    physical_address: u64,
    size: usize,
}

/// User-space device driver server
pub struct DeviceDriverServer {
    /// Server endpoint ID for communication with kernel
    server_epid: EndpointId,
    
    /// Registered devices
    devices: Mutex<BTreeMap<DeviceId, DeviceInfo>>,
    
    /// Next device ID
    next_device_id: AtomicU32,
}

impl DeviceDriverServer {
    /// Create a new device driver server
    pub fn new() -> Result<Self, ()> {
        // Create endpoint for communication with kernel
        let server_epid = endpoint_create().map_err(|_| ())?;
        
        Ok(Self {
            server_epid,
            devices: Mutex::new(BTreeMap::new()),
            next_device_id: AtomicU32::new(1000), // Start from 1000 to avoid conflicts
        })
    }
    
    /// Register with kernel device manager
    pub fn register_with_kernel(&self) -> Result<(), ()> {
        // In a real implementation, this would send a registration message to the kernel
        // For now, we'll just log that registration would happen
        // log_message(&format!("Device driver server registered with kernel using EPID {}", self.server_epid.0));
        Ok(())
    }
    
    /// Main server loop - process incoming IPC messages
    pub fn run(&self) -> ! {
        // log_message(&format!("Device driver server starting with EPID {}", self.server_epid.0));
        
        // Initialize GIC driver
        drivers::gic::GICV2_DRIVER.init();
        
        loop {
            // Receive IPC message from kernel
            match endpoint_recv_sync(self.server_epid, None) {
                Ok(msg) => {
                    self.handle_ipc_message(msg);
                }
                Err(e) => {
                    /* log_message(crate::println!(format!( "Error receiving IPC message: {:?}", e)); */ 
                    // Handle error or continue
                    continue;
                }
            }
        }
    }
    
    /// Handle incoming IPC message
    fn handle_ipc_message(&self, msg: IpcMessage) {
        match msg.op as u64 {
            DEV_OP_REGISTER_DRIVER => {
                self.handle_register_driver(msg);
            }
            DEV_OP_REQUEST_IRQ => {
                self.handle_request_irq(msg);
            }
            DEV_OP_MMIO_MAP => {
                self.handle_mmio_map(msg);
            }
            DEV_OP_DMA_ALLOC => {
                self.handle_dma_alloc(msg);
            }
            DEV_OP_INTERRUPT => {
                self.handle_interrupt(msg);
            }
            DEV_OP_IO_READ => {
                self.handle_io_read(msg);
            }
            DEV_OP_IO_WRITE => {
                self.handle_io_write(msg);
            }
            DEV_OP_GIC_INIT => {
                self.handle_gic_init(msg);
            }
            DEV_OP_GIC_ENABLE_IRQ => {
                self.handle_gic_enable_irq(msg);
            }
            DEV_OP_GIC_DISABLE_IRQ => {
                self.handle_gic_disable_irq(msg); 
            }
            DEV_OP_GIC_SET_PRIORITY => {
                self.handle_gic_set_priority(msg);
            }
            _ => {
                /* log_message(crate::println!(format!( "Device driver server received unknown opcode: {}", msg.op)); */ 
                
                // Send error response back to kernel
                let response_msg = IpcMessage {
                    src_pid: 0,
                    dst_epid: msg.src_pid,
                    op: msg.op,
                    priority: Priority::Normal,
                    data_len: 4,
                    data: {
                        let mut arr = [0u8; 256];
                        arr[0] = 1; // Error code
                        arr
                    },
                    timestamp: crate::arch::timer::now_us(),
                };
                
                if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
                    /* log_message(crate::println!(format!( "Failed to send error response: {:?}", e)); */ 
                }
            }
        }
    }
    
    /// Handle device registration request
    fn handle_register_driver(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!( "Handling device registration request")); */ 
        
        // Parse device registration request from message data
        // In a real implementation, we would deserialize the request properly
        // For now, we'll just send a success response
        
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!( "Failed to send registration response: {:?}", e)); */ 
        }
    }
    
    /// Handle IRQ request
    fn handle_request_irq(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(* "Handling IRQ request")); */ 
        
        // Parse IRQ request from message data
        // In a real implementation, we would deserialize the request properly
        // For now, we'll just send a success response
        
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(* "Failed to send IRQ response: {:?}", e)); */ 
        }
    }
    
    /// Handle MMIO mapping request
    fn handle_mmio_map(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(* "Handling MMIO mapping request")); */ 
        
        // Parse MMIO request from message data
        // In a real implementation, we would deserialize the request properly
        // For now, we'll just send a success response
        
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*"Failed to send MMIO response: {:?}", e)); */ 
        }
    }
    
    /// Handle DMA allocation request
    fn handle_dma_alloc(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling DMA allocation request")); */ 
        
        // Parse DMA request from message data
        // In a real implementation, we would deserialize the request properly
        // For now, we'll just send a success response
        
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send DMA response: {:?}", e)); */ 
        }
    }
    
    /// Handle interrupt notification from kernel
    fn handle_interrupt(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling interrupt notification from kernel")); */ 
        
        // Parse interrupt notification from message data
        if msg.data_len >= 4 {
            let irq_number = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]);
            /* log_message(crate::println!(format!(*Received interrupt for IRQ {}", irq_number)); */ 
            
            // Handle specific interrupts
            match irq_number {
                30 => {
                    // Timer interrupt
                    self.handle_timer_interrupt( ); 
                }
                33 => {
                    // UART interrupt
                    self.handle_uart_interrupt( ); 
                }
                _ => {
                    /* log_message(crate::println!(format!(*Unhandled interrupt for IRQ {}", irq_number)); */ 
                }
            }
        }
        
        // Send acknowledgment response
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send interrupt acknowledgment: {:?}", e)); */ 
        }
    }
    
    /// Handle timer interrupt
    fn handle_timer_interrupt(&self) {
        /* log_message(crate::println!(format!(*Handling timer interrupt")); */ 
        
        // In a real implementation, we would handle timer events
        // For now, we'll just log that we received the interrupt
    }
    
    /// Handle UART interrupt
    fn handle_uart_interrupt(&self) {
        /* log_message(crate::println!(format!(*Handling UART interrupt")); */ 
        
        // In a real implementation, we would read data from the UART FIFO
        // and process it accordingly
        // For now, we'll just log that we received the interrupt
    }
    
    /// Handle I/O read request
    fn handle_io_read(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling I/O read request")); */ 
        
        // In a real implementation, we would perform the I/O read operation
        // For now, we'll just send a success response
        
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send I/O read response: {:?}", e)); */ 
        }
    }
    
    /// Handle I/O write request
    fn handle_io_write(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling I/O write request")); */ 
        
        // In a real implementation, we would perform the I/O write operation
        // For now, we'll just send a success response
        
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send I/O write response: {:?}", e)); */ 
        }
    }
    
    /// Handle GIC initialization request
    fn handle_gic_init(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling GIC initialization request")); */ 
        
        // Initialize GIC driver
        drivers::gic::GICV2_DRIVER.init();
        
        // Send success response
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send GIC init response: {:?}", e)); */ 
        }
    }
    
    /// Handle GIC enable IRQ request
    fn handle_gic_enable_irq(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling GIC enable IRQ request")); */ 
        
        // Parse IRQ number from message data
        if msg.data_len >= 4 {
            let irq_number = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]] ); 
            /* log_message(crate::println!(format!(*Enabling IRQ {}", irq_number)); */ 
            
            // Enable the IRQ in GIC
            drivers::gic::GICV2_DRIVER.enable_irq(irq_number ); 
        }
        
        // Send success response
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send GIC enable IRQ response: {:?}", e)); */ 
        }
    }
    
    /// Handle GIC disable IRQ request
    fn handle_gic_disable_irq(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling GIC disable IRQ request")); */ 
        
        // Parse IRQ number from message data
        if msg.data_len >= 4 {
            let irq_number = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]] ); 
            /* log_message(crate::println!(format!(*Disabling IRQ {}", irq_number)); */ 
            
            // Disable the IRQ in GIC
            drivers::gic::GICV2_DRIVER.disable_irq(irq_number ); 
        }
        
        // Send success response
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send GIC disable IRQ response: {:?}", e)); */ 
        }
    }
    
    /// Handle GIC set priority request
    fn handle_gic_set_priority(&self, msg: IpcMessage) {
        /* log_message(crate::println!(format!(*Handling GIC set priority request")); */ 
        
        // Parse IRQ number and priority from message data
        if msg.data_len >= 5 {
            let irq_number = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]] ); 
            let priority = msg.data[4];
            /* log_message(crate::println!(format!(*Setting priority for IRQ {} to {}", irq_number, priority)); */ 
            
            // Set the IRQ priority in GIC
            drivers::gic::GICV2_DRIVER.set_interrupt_priority(irq_number, priority ); 
        }
        
        // Send success response
        let response_msg = IpcMessage {
            src_pid: 0,
            dst_epid: msg.src_pid,
            op: msg.op,
            priority: Priority::Normal,
            data_len: 4,
            data: {
                let mut arr = [0u8; 256];
                arr[0] = 0; // Success code
                arr
            },
            timestamp: crate::arch::timer::now_us(),
        };
        
        if let Err(e) = endpoint_send_sync(msg.src_pid, response_msg, None) {
            /* log_message(crate::println!(format!(*Failed to send GIC set priority response: {:?}", e)); */ 
        }
    }
}

/// Initialize and start the device driver server
pub fn start_device_driver_server() -> ! {
    /* log_message(crate::println!(format!(*Starting user-space device driver server...")); */ 
    
    // Create device driver server instance
    let server = DeviceDriverServer::new().expect("Failed to create device driver server" ); 
    
    // Register with kernel
    server.register_with_kernel().expect("Failed to register with kernel" ); 
    
    // Start server loop
    server.run( ); 
}
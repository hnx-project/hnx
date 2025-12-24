//! Service Protocol Definitions
//!
//! This module defines the protocol for communicating with user space services
//! via IPC. All services use a common message format.

use crate::core::ipc::{IpcMessage, IpcResponse, Priority};

/// Service operation codes
///
/// Each service defines its own operation codes. These are examples
/// for the VFS and Network services.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum ServiceOp {
    // === VFS Operations (100-199) ===
    VfsOpen = 100,
    VfsRead = 101,
    VfsWrite = 102,
    VfsClose = 103,
    VfsSeek = 104,
    VfsStat = 105,
    VfsReadDir = 106,
    VfsCreate = 107,
    VfsUnlink = 108,
    VfsMkdir = 109,
    VfsRmdir = 110,
    
    // === Network Operations (200-299) ===
    NetSocket = 200,
    NetBind = 201,
    NetConnect = 202,
    NetListen = 203,
    NetAccept = 204,
    NetSend = 205,
    NetRecv = 206,
    NetClose = 207,
    NetGetSockOpt = 208,
    NetSetSockOpt = 209,
    
    // === Loader Operations (300-399) ===
    LoaderOpen = 300,
    LoaderClose = 301,
    LoaderSym = 302,
    LoaderError = 303,
}

impl ServiceOp {
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
    
    pub const fn from_u16(op: u16) -> Option<Self> {
        match op {
            // VFS
            100 => Some(ServiceOp::VfsOpen),
            101 => Some(ServiceOp::VfsRead),
            102 => Some(ServiceOp::VfsWrite),
            103 => Some(ServiceOp::VfsClose),
            104 => Some(ServiceOp::VfsSeek),
            105 => Some(ServiceOp::VfsStat),
            106 => Some(ServiceOp::VfsReadDir),
            107 => Some(ServiceOp::VfsCreate),
            108 => Some(ServiceOp::VfsUnlink),
            109 => Some(ServiceOp::VfsMkdir),
            110 => Some(ServiceOp::VfsRmdir),
            
            // Network
            200 => Some(ServiceOp::NetSocket),
            201 => Some(ServiceOp::NetBind),
            202 => Some(ServiceOp::NetConnect),
            203 => Some(ServiceOp::NetListen),
            204 => Some(ServiceOp::NetAccept),
            205 => Some(ServiceOp::NetSend),
            206 => Some(ServiceOp::NetRecv),
            207 => Some(ServiceOp::NetClose),
            208 => Some(ServiceOp::NetGetSockOpt),
            209 => Some(ServiceOp::NetSetSockOpt),
            
            // Loader
            300 => Some(ServiceOp::LoaderOpen),
            301 => Some(ServiceOp::LoaderClose),
            302 => Some(ServiceOp::LoaderSym),
            303 => Some(ServiceOp::LoaderError),
            
            _ => None,
        }
    }
}

/// Service request builder
///
/// Helps build IPC messages for service calls
pub struct ServiceRequest {
    dst_epid: u32,
    op: u16,
    priority: Priority,
    data: [u8; 256],
    data_len: usize,
}

impl ServiceRequest {
    /// Create a new service request
    pub fn new(dst_epid: u32, op: ServiceOp) -> Self {
        Self {
            dst_epid,
            op: op.as_u16(),
            priority: Priority::Normal,
            data: [0; 256],
            data_len: 0,
        }
    }
    
    /// Set request priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Add a u32 parameter
    pub fn with_u32(mut self, value: u32) -> Self {
        if self.data_len + 4 <= self.data.len() {
            let bytes = value.to_le_bytes();
            self.data[self.data_len..self.data_len + 4].copy_from_slice(&bytes);
            self.data_len += 4;
        }
        self
    }
    
    /// Add a u64 parameter
    pub fn with_u64(mut self, value: u64) -> Self {
        if self.data_len + 8 <= self.data.len() {
            let bytes = value.to_le_bytes();
            self.data[self.data_len..self.data_len + 8].copy_from_slice(&bytes);
            self.data_len += 8;
        }
        self
    }
    
    /// Add a string parameter (with length prefix)
    pub fn with_str(mut self, s: &str) -> Self {
        let len = s.len().min(255); // Max 255 bytes for string
        if self.data_len + 1 + len <= self.data.len() {
            self.data[self.data_len] = len as u8;
            self.data[self.data_len + 1..self.data_len + 1 + len]
                .copy_from_slice(&s.as_bytes()[..len]);
            self.data_len += 1 + len;
        }
        self
    }
    
    /// Add raw bytes
    pub fn with_bytes(mut self, bytes: &[u8]) -> Self {
        let len = bytes.len().min(self.data.len() - self.data_len);
        self.data[self.data_len..self.data_len + len].copy_from_slice(&bytes[..len]);
        self.data_len += len;
        self
    }
    
    /// Build the IPC message
    pub fn build(self) -> IpcMessage {
        IpcMessage {
            src_pid: 0, // Will be filled by IPC layer
            dst_epid: self.dst_epid,
            op: self.op,
            priority: self.priority,
            data_len: self.data_len,
            data: self.data,
            timestamp: 0, // Will be filled by IPC layer
        }
    }
}

/// Service response wrapper
///
/// Wraps IPC response and provides convenient accessors
pub struct ServiceResponse {
    inner: IpcResponse,
}

impl ServiceResponse {
    /// Create from IPC response
    pub fn from_ipc(resp: IpcResponse) -> Self {
        Self { inner: resp }
    }
    
    /// Get response code (0 = success, negative = error)
    pub fn code(&self) -> i32 {
        self.inner.code
    }
    
    /// Check if response indicates success
    pub fn is_ok(&self) -> bool {
        self.inner.code >= 0
    }
    
    /// Get response data
    pub fn data(&self) -> &[u8] {
        &self.inner.data[..self.inner.data_len]
    }
    
    /// Read u32 from response data
    pub fn read_u32(&self) -> Option<u32> {
        if self.inner.data_len >= 4 {
            let bytes = [
                self.inner.data[0],
                self.inner.data[1],
                self.inner.data[2],
                self.inner.data[3],
            ];
            Some(u32::from_le_bytes(bytes))
        } else {
            None
        }
    }
    
    /// Read u64 from response data
    pub fn read_u64(&self) -> Option<u64> {
        if self.inner.data_len >= 8 {
            let bytes = [
                self.inner.data[0],
                self.inner.data[1],
                self.inner.data[2],
                self.inner.data[3],
                self.inner.data[4],
                self.inner.data[5],
                self.inner.data[6],
                self.inner.data[7],
            ];
            Some(u64::from_le_bytes(bytes))
        } else {
            None
        }
    }
}

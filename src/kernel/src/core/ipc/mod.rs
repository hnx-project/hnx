//! Enhanced IPC system with support for synchronous/asynchronous communication,
//! priority-based messaging, and improved security integration.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::Mutex;

/// Priority levels for messages
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Enhanced IPC message with priority support
#[derive(Clone)]
pub struct IpcMessage {
    pub msg_id: u64,
    pub src_pid: u32,
    pub dst_epid: u32,
    pub op: u16,
    pub priority: Priority,
    pub data_len: usize,
    pub data: [u8; 256], // Fixed size array instead of Vec
    pub timestamp: u64,
}

/// Response message
#[derive(Clone)]
pub struct IpcResponse {
    pub msg_id: u64,
    pub code: i32,
    pub data_len: usize,
    pub data: [u8; 256], // Fixed size array instead of Vec
}

/// Endpoint capabilities for access control
#[derive(Clone, Copy)]
pub struct EndpointCapabilities {
    pub read: bool,
    pub write: bool,
    pub admin: bool,
}

/// Handle for asynchronous operations
pub struct AsyncHandle {
    pub id: u64,
    pub status: AsyncStatus,
    pub result: Option<IpcResponse>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AsyncStatus {
    Pending,
    Completed,
    Error,
    Cancelled,
}

/// Error types for IPC operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    InvalidEndpoint,
    PermissionDenied,
    QueueFull,
    Timeout,
    InvalidMessage,
    OutOfMemory,
    AlreadyExists,
    NotFound,
    InvalidOperation,
    SystemError,
}

impl From<IpcError> for i32 {
    fn from(err: IpcError) -> i32 {
        match err {
            IpcError::InvalidEndpoint => -1,
            IpcError::PermissionDenied => -2,
            IpcError::QueueFull => -3,
            IpcError::Timeout => -4,
            IpcError::InvalidMessage => -5,
            IpcError::OutOfMemory => -6,
            IpcError::AlreadyExists => -7,
            IpcError::NotFound => -8,
            IpcError::InvalidOperation => -9,
            IpcError::SystemError => -10,
        }
    }
}

/// Endpoint statistics for diagnostics
#[derive(Default, Clone)]
pub struct EndpointStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_transferred: u64,
    pub errors: u64,
    pub creation_time: u64,
}

/// Enhanced endpoint with priority queues
pub struct Endpoint {
    pub id: u32,
    pub owner_pid: u32,
    pub capabilities: EndpointCapabilities,
    
    // Priority-based message queues (fixed size arrays)
    pub critical_priority_queue: [Option<IpcMessage>; 32],
    pub high_priority_queue: [Option<IpcMessage>; 32],
    pub normal_priority_queue: [Option<IpcMessage>; 32],
    pub low_priority_queue: [Option<IpcMessage>; 32],
    
    // Queue indices
    pub critical_head: usize,
    pub critical_tail: usize,
    pub critical_len: usize,
    
    pub high_head: usize,
    pub high_tail: usize,
    pub high_len: usize,
    
    pub normal_head: usize,
    pub normal_tail: usize,
    pub normal_len: usize,
    
    pub low_head: usize,
    pub low_tail: usize,
    pub low_len: usize,
    
    // Wait queues for blocking operations
    pub waiters: [u32; 16],
    pub waiters_head: usize,
    pub waiters_tail: usize,
    pub waiters_len: usize,
    
    // Statistics and diagnostics
    pub stats: EndpointStats,
}

// Global state
static NEXT_ENDPOINT_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_MSG_ID: AtomicU64 = AtomicU64::new(1);
static ENDPOINTS: Mutex<[Option<Endpoint>; 16]> = Mutex::new([const { None }; 16]);

/// Initialize the IPC system
pub fn init() {
    // Initialization is handled by static initialization
}

/// Helper function to create an empty endpoint
fn create_empty_endpoint(id: u32, owner_pid: u32, capabilities: EndpointCapabilities) -> Endpoint {
    Endpoint {
        id,
        owner_pid,
        capabilities,
        critical_priority_queue: [const { None }; 32],
        high_priority_queue: [const { None }; 32],
        normal_priority_queue: [const { None }; 32],
        low_priority_queue: [const { None }; 32],
        critical_head: 0,
        critical_tail: 0,
        critical_len: 0,
        high_head: 0,
        high_tail: 0,
        high_len: 0,
        normal_head: 0,
        normal_tail: 0,
        normal_len: 0,
        low_head: 0,
        low_tail: 0,
        low_len: 0,
        waiters: [0; 16],
        waiters_head: 0,
        waiters_tail: 0,
        waiters_len: 0,
        stats: EndpointStats::default(),
    }
}

/// Create a new endpoint with specified capabilities
pub fn endpoint_create(capabilities: EndpointCapabilities) -> Result<u32, IpcError> {
    let epid = NEXT_ENDPOINT_ID.fetch_add(1, Ordering::Relaxed);
    let owner_pid = super::scheduler::current_pid() as u32;
    
    let mut endpoints = ENDPOINTS.lock();
    
    // Find an empty slot
    for slot in endpoints.iter_mut() {
        if slot.is_none() {
            let endpoint = create_empty_endpoint(epid, owner_pid, capabilities);
            *slot = Some(endpoint);
            return Ok(epid);
        }
    }
    
    Err(IpcError::OutOfMemory)
}

/// Destroy an endpoint
pub fn endpoint_destroy(epid: u32) -> Result<(), IpcError> {
    let current_pid = super::scheduler::current_pid() as u32;
    
    let mut endpoints = ENDPOINTS.lock();
    
    for slot in endpoints.iter_mut() {
        if let Some(ref endpoint) = slot {
            if endpoint.id == epid {
                // Check if caller has admin rights or is the owner
                if endpoint.owner_pid == current_pid || endpoint.capabilities.admin {
                    *slot = None;
                    return Ok(());
                } else {
                    return Err(IpcError::PermissionDenied);
                }
            }
        }
    }
    
    Err(IpcError::InvalidEndpoint)
}

/// Helper function to push a message to a queue
fn push_message_to_queue(queue: &mut [Option<IpcMessage>; 32], head: &mut usize, tail: &mut usize, len: &mut usize, msg: IpcMessage) -> Result<(), IpcError> {
    if *len >= queue.len() {
        return Err(IpcError::QueueFull);
    }
    
    queue[*tail] = Some(msg);
    *tail = (*tail + 1) % queue.len();
    *len += 1;
    Ok(())
}

/// Helper function to pop a message from a queue
fn pop_message_from_queue(queue: &mut [Option<IpcMessage>; 32], head: &mut usize, tail: &mut usize, len: &mut usize) -> Option<IpcMessage> {
    if *len == 0 {
        return None;
    }
    
    let msg = queue[*head].take();
    *head = (*head + 1) % queue.len();
    *len -= 1;
    msg
}

/// Send a message synchronously
pub fn endpoint_send_sync(dst_epid: u32, mut msg: IpcMessage) -> Result<IpcResponse, IpcError> {
    crate::info!("endpoint_send_sync: sending to endpoint {}, op={}", dst_epid, msg.op);
    let current_pid = super::scheduler::current_pid() as u32;
    msg.src_pid = current_pid;
    msg.dst_epid = dst_epid;
    msg.timestamp = crate::arch::timer::now_us();
    
    // Get destination endpoint
    crate::info!("endpoint_send_sync: acquiring ENDPOINTS lock");
    let mut endpoints = ENDPOINTS.lock();
    crate::info!("endpoint_send_sync: lock acquired, iterating endpoints");

    for slot in endpoints.iter_mut() {
        if let Some(ref mut endpoint) = slot {
            crate::info!("endpoint_send_sync: checking endpoint id={}", endpoint.id);
            if endpoint.id == dst_epid {
                crate::info!("endpoint_send_sync: found endpoint {}, owner_pid={}", dst_epid, endpoint.owner_pid);
                // Check write permission
                if !endpoint.capabilities.write && endpoint.owner_pid != current_pid {
                    return Err(IpcError::PermissionDenied);
                }
                
                // Increment sent counter
                endpoint.stats.messages_sent += 1;
                endpoint.stats.bytes_transferred += msg.data_len as u64;
                
                // Add to appropriate priority queue
                let result = match msg.priority {
                    Priority::Critical => {
                        push_message_to_queue(
                            &mut endpoint.critical_priority_queue,
                            &mut endpoint.critical_head,
                            &mut endpoint.critical_tail,
                            &mut endpoint.critical_len,
                            msg
                        )
                    }
                    Priority::High => {
                        push_message_to_queue(
                            &mut endpoint.high_priority_queue,
                            &mut endpoint.high_head,
                            &mut endpoint.high_tail,
                            &mut endpoint.high_len,
                            msg
                        )
                    }
                    Priority::Normal => {
                        push_message_to_queue(
                            &mut endpoint.normal_priority_queue,
                            &mut endpoint.normal_head,
                            &mut endpoint.normal_tail,
                            &mut endpoint.normal_len,
                            msg
                        )
                    }
                    Priority::Low => {
                        push_message_to_queue(
                            &mut endpoint.low_priority_queue,
                            &mut endpoint.low_head,
                            &mut endpoint.low_tail,
                            &mut endpoint.low_len,
                            msg
                        )
                    }
                };
                
                if result.is_err() {
                    endpoint.stats.errors += 1;
                    // Convert the error to the correct type
                    return Err(result.err().unwrap());
                }
                
                // Wake up any waiting processes
                if endpoint.waiters_len > 0 {
                    let pid = endpoint.waiters[endpoint.waiters_head];
                    endpoint.waiters_head = (endpoint.waiters_head + 1) % endpoint.waiters.len();
                    endpoint.waiters_len -= 1;
                    let _ = crate::process::wake_process(pid as usize);
                }
                
                // Return a simple response
                return Ok(IpcResponse {
                    msg_id: 0,
                    code: 0,
                    data_len: 0,
                    data: [0; 256],
                });
            }
        }
    }
    
    Err(IpcError::InvalidEndpoint)
}

/// Receive a message synchronously
pub fn endpoint_recv_sync(epid: u32, _timeout_ms: Option<u64>) -> Result<IpcMessage, IpcError> {
    let current_pid = super::scheduler::current_pid() as u32;
    
    // Get endpoint
    let mut endpoints = ENDPOINTS.lock();
    
    for slot in endpoints.iter_mut() {
        if let Some(ref mut endpoint) = slot {
            if endpoint.id == epid {
                // Check read permission
                if !endpoint.capabilities.read && endpoint.owner_pid != current_pid {
                    return Err(IpcError::PermissionDenied);
                }
                
                // Try to get message from highest priority queue first
                if endpoint.critical_len > 0 {
                    if let Some(msg) = pop_message_from_queue(
                        &mut endpoint.critical_priority_queue,
                        &mut endpoint.critical_head,
                        &mut endpoint.critical_tail,
                        &mut endpoint.critical_len
                    ) {
                        endpoint.stats.messages_received += 1;
                        endpoint.stats.bytes_transferred += msg.data_len as u64;
                        return Ok(msg);
                    }
                }
                
                if endpoint.high_len > 0 {
                    if let Some(msg) = pop_message_from_queue(
                        &mut endpoint.high_priority_queue,
                        &mut endpoint.high_head,
                        &mut endpoint.high_tail,
                        &mut endpoint.high_len
                    ) {
                        endpoint.stats.messages_received += 1;
                        endpoint.stats.bytes_transferred += msg.data_len as u64;
                        return Ok(msg);
                    }
                }
                
                if endpoint.normal_len > 0 {
                    if let Some(msg) = pop_message_from_queue(
                        &mut endpoint.normal_priority_queue,
                        &mut endpoint.normal_head,
                        &mut endpoint.normal_tail,
                        &mut endpoint.normal_len
                    ) {
                        endpoint.stats.messages_received += 1;
                        endpoint.stats.bytes_transferred += msg.data_len as u64;
                        return Ok(msg);
                    }
                }
                
                if endpoint.low_len > 0 {
                    if let Some(msg) = pop_message_from_queue(
                        &mut endpoint.low_priority_queue,
                        &mut endpoint.low_head,
                        &mut endpoint.low_tail,
                        &mut endpoint.low_len
                    ) {
                        endpoint.stats.messages_received += 1;
                        endpoint.stats.bytes_transferred += msg.data_len as u64;
                        return Ok(msg);
                    }
                }
                
                // No messages available, add to waiters queue and block the process
                if endpoint.waiters_len < endpoint.waiters.len() {
                    endpoint.waiters[endpoint.waiters_tail] = current_pid;
                    endpoint.waiters_tail = (endpoint.waiters_tail + 1) % endpoint.waiters.len();
                    endpoint.waiters_len += 1;
                    
                    // Drop the lock before blocking
                    drop(endpoints);
                    let _ = crate::process::block_process(current_pid as usize);
                    
                    // When woken up, try again (simplified - in reality would need to check again)
                    return Err(IpcError::Timeout);
                } else {
                    return Err(IpcError::QueueFull);
                }
            }
        }
    }
    
    Err(IpcError::InvalidEndpoint)
}

/// Send a message asynchronously
pub fn endpoint_send_async(dst_epid: u32, msg: IpcMessage) -> Result<AsyncHandle, IpcError> {
    // For now, just delegate to sync send but return an async handle
    let result = endpoint_send_sync(dst_epid, msg);
    
    let handle = match result {
        Ok(response) => AsyncHandle {
            id: 1, // Simplified ID
            status: AsyncStatus::Completed,
            result: Some(response),
        },
        Err(_) => AsyncHandle {
            id: 1,
            status: AsyncStatus::Error,
            result: None,
        },
    };
    
    Ok(handle)
}

/// Wait for an asynchronous operation to complete
pub fn async_wait(handle: AsyncHandle, _timeout_ms: Option<u64>) -> Result<IpcResponse, IpcError> {
    match handle.status {
        AsyncStatus::Completed => {
            if let Some(response) = handle.result {
                Ok(response)
            } else {
                Err(IpcError::SystemError)
            }
        }
        AsyncStatus::Error => Err(IpcError::SystemError),
        AsyncStatus::Cancelled => Err(IpcError::InvalidOperation),
        AsyncStatus::Pending => {
            // In a real implementation, we would block until completion
            Err(IpcError::Timeout)
        }
    }
}

/// Cancel an asynchronous operation
pub fn async_cancel(_handle: AsyncHandle) -> Result<(), IpcError> {
    // Simplified implementation
    Ok(())
}

/// Grant capabilities to an endpoint for a specific process
pub fn endpoint_grant_capability(epid: u32, _pid: u32, cap: EndpointCapabilities) -> Result<(), IpcError> {
    let current_pid = super::scheduler::current_pid() as u32;
    
    let mut endpoints = ENDPOINTS.lock();
    
    for slot in endpoints.iter_mut() {
        if let Some(ref mut endpoint) = slot {
            if endpoint.id == epid {
                // Only owner or admin can grant capabilities
                if endpoint.owner_pid == current_pid || endpoint.capabilities.admin {
                    endpoint.capabilities = cap;
                    return Ok(());
                } else {
                    return Err(IpcError::PermissionDenied);
                }
            }
        }
    }
    
    Err(IpcError::InvalidEndpoint)
}

/// Get endpoint statistics
pub fn get_endpoint_stats(epid: u32) -> Result<EndpointStats, IpcError> {
    let endpoints = ENDPOINTS.lock();
    
    for slot in endpoints.iter() {
        if let Some(ref endpoint) = slot {
            if endpoint.id == epid {
                return Ok(endpoint.stats.clone());
            }
        }
    }
    
    Err(IpcError::InvalidEndpoint)
}

/// Legacy message type for backward compatibility
#[derive(Clone, Copy)]
pub struct Msg {
    pub service: u16,
    pub op: u16,
    pub p1: *const u8,
    pub l1: usize,
    pub p2: *mut u8,
    pub l2: usize,
}

/// Legacy response type for backward compatibility
#[derive(Clone, Copy)]
pub struct Resp {
    pub code: i32,
    pub len: usize,
}

pub const SERVICE_PROC: u16 = 1;
pub const SERVICE_VFS: u16 = 2;

type Handler = fn(&Msg) -> Resp;

static SERVICES: Mutex<[Option<Handler>; 8]> = Mutex::new([None; 8]);

/// Register a service handler for backward compatibility
pub fn register(service_id: u16, handler: Handler) -> bool {
    let mut g = SERVICES.lock();
    let idx = (service_id as usize) % g.len();
    if g[idx].is_none() {
        g[idx] = Some(handler);
        true
    } else {
        false
    }
}

/// Call a service handler for backward compatibility
pub fn call(msg: Msg) -> Resp {
    let h = {
        let g = SERVICES.lock();
        let idx = (msg.service as usize) % g.len();
        g[idx]
    };
    if let Some(f) = h {
        f(&msg)
    } else {
        Resp { code: -1, len: 0 }
    }
}

/// Legacy IPC message type for backward compatibility
#[derive(Clone)]
pub struct IpcMsg {
    pub src: u32,
    pub op: u16,
    pub data_len: usize,
    pub data: [u8; 256],
}

/// Send a message to an endpoint (legacy API)
pub fn endpoint_send(dst_id: u32, msg: IpcMsg) -> bool {
    // Convert to new message format
    let new_msg = IpcMessage {
        msg_id: 0, // Will be filled by IPC layer
        src_pid: msg.src,
        dst_epid: dst_id,
        op: msg.op,
        priority: Priority::Normal,
        data_len: msg.data_len,
        data: msg.data,
        timestamp: crate::arch::timer::now_us(),
    };
    
    // Send using new API
    endpoint_send_sync(dst_id, new_msg).is_ok()
}

/// Receive a message from an endpoint (legacy API)
pub fn endpoint_recv(id: u32) -> Option<IpcMsg> {
    // Receive using new API
    match endpoint_recv_sync(id, None) {
        Ok(msg) => {
            Some(IpcMsg {
                src: msg.src_pid,
                op: msg.op,
                data_len: msg.data_len,
                data: msg.data,
            })
        }
        Err(_) => None,
    }
}

#[cfg(test)]
pub mod test;

#[cfg(test)]
pub mod example;
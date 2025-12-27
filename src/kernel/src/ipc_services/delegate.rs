//! IPC Delegation Functions
//!
//! This module provides high-level functions for delegating syscalls
//! to user space services via IPC.

use crate::core::ipc::{endpoint_send_sync, IpcError};
use super::protocol::{ServiceRequest, ServiceResponse, ServiceOp};

/// Error codes for service delegation
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DelegateError {
    /// Service not available (endpoint not responding)
    ServiceUnavailable,
    /// IPC communication error
    IpcError(IpcError),
    /// Service returned an error
    ServiceError(i32),
    /// Invalid response from service
    InvalidResponse,
}

impl From<IpcError> for DelegateError {
    fn from(err: IpcError) -> Self {
        DelegateError::IpcError(err)
    }
}

/// Call a user space service synchronously
///
/// This is the main function for delegating syscalls to user space services.
///
/// # Arguments
/// * `request` - The service request to send
///
/// # Returns
/// * `Ok(ServiceResponse)` - Service responded successfully
/// * `Err(DelegateError)` - Communication or service error
///
/// # Example
/// ```rust,ignore
/// let req = ServiceRequest::new(WELL_KNOWN_SERVICES.VFS_EPID, ServiceOp::VfsOpen)
///     .with_str("/etc/passwd")
///     .with_u32(O_RDONLY);
///     
/// match ipc_call_service(req) {
///     Ok(resp) if resp.is_ok() => {
///         let fd = resp.read_u32().unwrap();
///         // Use file descriptor
///     }
///     Ok(resp) => {
///         // Service returned error
///         let errno = -resp.code();
///     }
///     Err(e) => {
///         // IPC error
///     }
/// }
/// ```
pub fn ipc_call_service(request: ServiceRequest) -> Result<ServiceResponse, DelegateError> {
    let msg = request.build();

    crate::info!(
        "ipc_delegate: sending request to endpoint {} op={}",
        msg.dst_epid,
        msg.op
    );

    // Send IPC message and wait for response
    match endpoint_send_sync(msg.dst_epid, msg) {
        Ok(ipc_resp) => {
            let resp = ServiceResponse::from_ipc(ipc_resp);
            crate::info!("ipc_delegate: created ServiceResponse");

            crate::info!(
                "ipc_delegate: received response code={}",
                resp.code()
            );

            Ok(resp)
        }
        Err(ipc_err) => {
            crate::warn!(
                "ipc_delegate: IPC error: {:?}",
                ipc_err
            );
            Err(DelegateError::from(ipc_err))
        }
    }
}

/// Simplified delegation function for common patterns
///
/// This wraps ipc_call_service and converts the response to a simple isize
/// return value (positive = success/handle, negative = errno).
pub fn ipc_delegate(
    epid: u32,
    op: ServiceOp,
    build_request: impl FnOnce(ServiceRequest) -> ServiceRequest,
) -> isize {
    crate::info!("ipc_delegate: starting delegation to endpoint {}", epid);
    let request = build_request(ServiceRequest::new(epid, op));
    crate::info!("ipc_delegate: request built");

    match ipc_call_service(request) {
        Ok(resp) => {
            crate::info!("ipc_delegate: ipc_call_service returned Ok");
            if resp.is_ok() {
                // Try to read u32 response as file descriptor / handle
                let result = resp.read_u32().unwrap_or(0) as isize;
                crate::info!("ipc_delegate: returning success result {}", result);
                result
            } else {
                // Service returned error code
                let result = resp.code() as isize;
                crate::info!("ipc_delegate: returning error result {}", result);
                result
            }
        }
        Err(DelegateError::ServiceUnavailable) => {
            crate::warn!("ipc_delegate: service unavailable");
            -1 // ENOSYS or similar
        }
        Err(DelegateError::IpcError(_)) => {
            crate::warn!("ipc_delegate: IPC error");
            -2 // EIO or similar
        }
        Err(DelegateError::ServiceError(code)) => {
            let result = code as isize;
            crate::info!("ipc_delegate: service error code {}", result);
            result
        }
        Err(DelegateError::InvalidResponse) => {
            crate::warn!("ipc_delegate: invalid response");
            -3 // EINVAL or similar
        }
    }
}

/// Initialize the IPC service delegation framework
pub fn init() -> Result<(), ()> {
    use super::endpoints::init_service_endpoints;
    
    crate::info!("ipc_services: initializing delegation framework");
    
    // Create well-known service endpoints
    init_service_endpoints()?;
    
    crate::info!("ipc_services: delegation framework ready");
    Ok(())
}

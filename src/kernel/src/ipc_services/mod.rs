//! IPC Service Delegation Framework
//!
//! This module provides the infrastructure for delegating kernel functionality
//! to user space services via IPC. This is a core component of the microkernel
//! architecture.
//!
//! # Architecture
//!
//! ```text
//! User Process
//!     |
//!     | syscall(SYS_OPEN, ...)
//!     v
//! Kernel Syscall Handler
//!     |
//!     | ipc_delegate(FS_SERVICE, ...)
//!     v
//! IPC Subsystem
//!     |
//!     | ep_send_sync(FS_EPID, msg)
//!     v
//! VFS Service (user space)
//!     |
//!     | process request
//!     v
//! IPC Response
//!     |
//!     | return to syscall handler
//!     v
//! Return to User Process
//! ```

pub mod endpoints;
pub mod protocol;
pub mod delegate;

pub use endpoints::{ServiceEndpoint, WELL_KNOWN_SERVICES};
pub use protocol::{ServiceOp, ServiceRequest, ServiceResponse};
pub use delegate::{ipc_call_service, ipc_delegate};

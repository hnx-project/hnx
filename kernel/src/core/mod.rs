//! Core kernel subsystems
//!
//! This module contains the fundamental building blocks of the HNX microkernel:
//! - **IPC**: Inter-Process Communication with priority-based messaging
//! - **Scheduler**: Task scheduling and context switching
//!
//! These modules work together to provide the foundation for process isolation,
//! communication, and cooperative multitasking.

pub mod ipc;
pub mod scheduler;

pub use ipc::{
    IpcMessage, IpcResponse, IpcError, Priority,
    EndpointCapabilities, AsyncHandle, AsyncStatus,
    endpoint_create, endpoint_destroy,
    endpoint_send_sync, endpoint_recv_sync,
    endpoint_send_async, async_wait,
};

pub use scheduler::{
    run, run_task, run_task_with_args,
    current_pid, current_ttbr0_base,
    on_tick, schedule_rr_tick, schedule_priority,
    exit_current,
};
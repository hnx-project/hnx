# IPC Delegation Framework - Implementation Complete ✅

## Overview
The IPC delegation framework provides the infrastructure for delegating kernel syscalls to user space services. This is the foundation for moving non-essential functionality (filesystem, network, etc.) out of the kernel.

## Implementation Date
2024-12-24

## Architecture

```
┌─────────────┐
│User Process │
└──────┬──────┘
       │ syscall(SYS_OPEN, "/file", O_RDONLY)
       ▼
┌──────────────────────┐
│ Kernel Syscall Stub  │  ◄── process/syscall/mod.rs
│  sys_open(...)       │
└──────┬───────────────┘
       │ ipc_delegate(VFS_EPID, VfsOpen, ...)
       ▼
┌──────────────────────┐
│ IPC Services Layer   │  ◄── ipc_services/ (NEW)
│  - ServiceRequest    │
│  - ServiceResponse   │
│  - ipc_call_service()│
└──────┬───────────────┘
       │ endpoint_send_sync(epid, msg)
       ▼
┌──────────────────────┐
│ Core IPC Subsystem   │  ◄── core/ipc/
│  - Endpoints         │
│  - Message Queues    │
│  - Priority Handling │
└──────┬───────────────┘
       │ IPC message
       ▼
┌──────────────────────┐
│ VFS Service          │  ◄── User Space (future)
│ (user space process) │
│  - ep_recv(VFS_EPID) │
│  - handle_open()     │
│  - ep_send(response) │
└──────────────────────┘
```

## Components

### 1. Well-Known Service Endpoints

**File**: `kernel/src/ipc_services/endpoints.rs`

Defines the fixed endpoint IDs for system services:
- **VFS Service** (EPID 1): File system operations
- **Network Service** (EPID 2): Network stack operations
- **Loader Service** (EPID 3): Dynamic linking/loading
- **Block Device Service** (EPID 4): Block device access

These endpoints are created at kernel boot time and are available for any process to send messages to.

### 2. Service Protocol

**File**: `kernel/src/ipc_services/protocol.rs`

Defines the communication protocol between kernel and services:

#### Service Operations
- **VFS Ops (100-199)**: Open, Read, Write, Close, Seek, Stat, etc.
- **Network Ops (200-299)**: Socket, Bind, Connect, Listen, Accept, Send, Recv
- **Loader Ops (300-399)**: DlOpen, DlClose, DlSym

#### Message Format
```rust
ServiceRequest::new(VFS_EPID, ServiceOp::VfsOpen)
    .with_str("/etc/passwd")    // Path parameter
    .with_u32(O_RDONLY)         // Flags parameter
    .build()
```

### 3. Delegation Functions

**File**: `kernel/src/ipc_services/delegate.rs`

Provides high-level functions for syscall delegation:

#### `ipc_call_service(request)` 
- Low-level: Send request, get ServiceResponse
- Full error handling and logging
- Returns Result<ServiceResponse, DelegateError>

#### `ipc_delegate(epid, op, builder)`
- High-level: Simplified interface
- Converts response to isize (fd or errno)
- Used by syscall implementations

### 4. Integration with Kernel

**File**: `kernel/src/main.rs`

The framework is initialized during boot in `init_phase3_processes()`:

```rust
// Creates well-known endpoints for VFS, Network, Loader
ipc_services::delegate::init()
```

**Boot Log Output**:
```
[INFO] ipc_services: initializing delegation framework
[INFO] ipc_services: creating well-known service endpoints
[INFO] ipc_services: created endpoint 1 for vfs
[INFO] ipc_services: created endpoint 2 for network
[INFO] ipc_services: created endpoint 3 for loader
[INFO] ipc_services: delegation framework ready
```

## Usage Example

### Syscall Implementation (Kernel Side)

```rust
// In process/syscall/mod.rs

use crate::ipc_services::{WELL_KNOWN_SERVICES, ServiceOp, ipc_delegate};

pub fn sys_open(path_ptr: usize, flags: u32) -> isize {
    // 1. Validate user pointer
    if !user_range_ok(path_ptr, 256, false) {
        return -EFAULT;
    }
    
    // 2. Copy path from user space
    let mut path_buf = [0u8; 256];
    let len = copy_from_user(path_ptr, 256, &mut path_buf);
    let path = core::str::from_utf8(&path_buf[..len]).unwrap_or("");
    
    // 3. Delegate to VFS service via IPC
    ipc_delegate(
        WELL_KNOWN_SERVICES.VFS_EPID,
        ServiceOp::VfsOpen,
        |req| req.with_str(path).with_u32(flags)
    )
    
    // Returns: fd (positive) or -errno (negative)
}
```

### Service Implementation (User Space - Future)

```rust
// In space/services/vfs/main.rs (future implementation)

use hnx_ipc::{endpoint_recv, ServiceOp};

fn main() {
    let vfs_epid = 1; // Well-known VFS endpoint
    
    loop {
        // Receive request from kernel
        match endpoint_recv(vfs_epid, None) {
            Ok(msg) if msg.op == ServiceOp::VfsOpen.as_u16() => {
                // Parse request
                let path = parse_string(&msg.data);
                let flags = parse_u32(&msg.data, offset);
                
                // Perform operation
                let fd = vfs_open(path, flags);
                
                // Send response
                let resp = ServiceResponse::new(fd);
                endpoint_send(msg.src_pid, resp);
            }
            // Handle other operations...
            _ => {}
        }
    }
}
```

## Current Status

### ✅ Implemented
1. Service endpoint definitions and initialization
2. Service protocol (operations and message format)
3. Request/Response builders with type-safe encoding
4. IPC delegation functions (high and low level)
5. Integration with kernel boot process
6. Well-known endpoints created at boot time

### ⏳ Next Steps
1. **Implement syscall stubs** - Rewrite fs/network syscalls to use delegation
2. **Create user space service framework** - Base library for writing services
3. **Implement VFS service** - First user space service
4. **Implement Network service** - Second user space service
5. **Mark fs/ and network/ as .moved** - Once services are functional

## Benefits

### Security
- ✅ Reduced kernel attack surface
- ✅ Service crashes don't crash kernel
- ✅ Each service runs in isolated address space

### Flexibility
- ✅ Services can be updated without kernel rebuild
- ✅ Multiple filesystem implementations can coexist
- ✅ Easy to add new services

### Performance
- ⚠️ IPC overhead for each syscall
- ✅ But: Better cache locality in user space
- ✅ Services can use modern async patterns

## Design Decisions

### Why Fixed Endpoint IDs?
- Simplifies service discovery
- No need for service registry
- Services are system-critical and well-known
- Similar to UNIX well-known ports

### Why Synchronous IPC for Syscalls?
- Matches syscall semantics (blocking)
- Simpler implementation
- Async IPC available for other uses

### Why 256-byte Message Limit?
- Fits in cache line
- Encourages shared memory for large transfers
- Prevents memory exhaustion

## Testing

### Verify Endpoints Created
```bash
make run-kernel | grep "ipc_services"
```

Expected output:
```
[INFO] ipc_services: created endpoint 1 for vfs
[INFO] ipc_services: created endpoint 2 for network
[INFO] ipc_services: created endpoint 3 for loader
```

### Future: Test VFS Delegation
```bash
# Once VFS service is implemented
make test-vfs-delegation
```

## Related Documents
- `SYSCALL_ANALYSIS.md` - Which syscalls to delegate
- `MIGRATION_STATUS.md` - Overall migration progress
- `fs/MIGRATION_PLAN.md` - VFS service migration plan
- `network/MIGRATION_PLAN.md` - Network service migration plan

---
*Last Updated: 2024-12-24*
*Status: IPC Framework Complete ✅ | Services Pending ⏳*

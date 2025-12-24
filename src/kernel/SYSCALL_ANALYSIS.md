# System Call Analysis for Module Migration

## Overview
This document analyzes which system calls must remain in the kernel vs. which can be delegated to user space services.

## Microkernel System Call Categories

### Category 1: Core Kernel Syscalls (Must Stay)
These syscalls access kernel-only resources and MUST remain in the kernel:

#### Process Management
- `fork()` - Create process (requires kernel PCB manipulation)
- `exit()` - Terminate process  
- `wait4()` - Wait for child process
- `getpid()` / `getppid()` - Get process IDs
- `kill()` - Send signal to process
- `setpgid()` / `getpgid()` - Process group management
- `yield()` - Voluntarily yield CPU

#### Memory Management  
- `mmap()` - Map memory (requires page table manipulation)
- `munmap()` - Unmap memory
- `mprotect()` - Change memory protection

#### IPC (Core Functionality)
- `ep_create()` - Create IPC endpoint
- `ep_send()` - Send IPC message
- `ep_recv()` - Receive IPC message
- `ipc_wait()` - Wait for IPC
- `ipc_wake()` - Wake process

**Count: ~15 core syscalls**

### Category 2: Delegated Syscalls (Via IPC)
These syscalls should be handled by user space services via IPC:

#### File System (→ FS Service)
- `open()` - Delegate to VFS service
- `read()` - Delegate to VFS service  
- `write()` - Delegate to VFS service
- `close()` - Delegate to VFS service
- `creat()` - Delegate to VFS service
- `unlink()` - Delegate to VFS service
- `mkdir()` - Delegate to VFS service
- `rmdir()` - Delegate to VFS service

**Kernel Implementation**: 
```rust
pub fn sys_open(path: &str, flags: u32) -> isize {
    // 1. Validate path pointer
    // 2. Send IPC to VFS service
    // 3. Return file descriptor handle
    ipc_call(VFS_SERVICE, OPEN_OP, path, flags)
}
```

#### Network (→ Network Service)
- `socket()` - Delegate to network service
- `bind()` - Delegate to network service
- `connect()` - Delegate to network service
- `listen()` - Delegate to network service
- `accept()` - Delegate to network service
- `send()` - Delegate to network service
- `recv()` - Delegate to network service

**Kernel Implementation**:
```rust
pub fn sys_socket(domain: i32, type: i32, protocol: i32) -> isize {
    // Send IPC to network service
    ipc_call(NET_SERVICE, SOCKET_OP, domain, type, protocol)
}
```

#### Dynamic Linking (→ Loader Service)
- `dlopen()` - Delegate to dynamic linker service
- `dlclose()` - Delegate to dynamic linker service
- `dlsym()` - Delegate to dynamic linker service

#### Device Drivers (→ Driver Framework)
- `driver_register()` - Register driver with kernel
- `driver_request_irq()` - Request interrupt  
- `driver_map_mmio()` - Map MMIO region
- `driver_dma_alloc()` - Allocate DMA buffer

**Note**: Driver syscalls may need to stay in kernel for now, but eventually drivers should be user space too.

### Category 3: Syscalls to Remove
These syscalls expose internal implementation details and should be removed:

- Any syscall that directly exposes VFS internals
- Any syscall that directly exposes network stack internals
- Legacy compatibility syscalls

## Migration Strategy

### Phase 1: Design IPC Delegation Pattern
```rust
// Kernel syscall stub
pub fn sys_open(path_ptr: usize, flags: u32) -> isize {
    let pid = current_pid();
    
    // 1. Validate user pointer
    if !validate_user_pointer(path_ptr) {
        return -EFAULT;
    }
    
    // 2. Copy path from user space
    let path = copy_string_from_user(path_ptr)?;
    
    // 3. Send IPC to VFS service
    let msg = IpcMessage::new(FS_SERVICE_EPID, OP_OPEN)
        .with_str(&path)
        .with_u32(flags);
    
    let resp = ipc_send_sync(msg)?;
    
    // 4. Create file descriptor in process table
    let fd = alloc_fd(pid, resp.handle);
    
    fd as isize
}
```

### Phase 2: Service Registration
```rust
// At boot time:
// 1. Kernel creates well-known endpoints
const FS_SERVICE_EPID: u32 = 1;
const NET_SERVICE_EPID: u32 = 2;
const LOADER_SERVICE_EPID: u32 = 3;

// 2. Init process starts services
//    - FS service calls ep_recv(FS_SERVICE_EPID)
//    - Network service calls ep_recv(NET_SERVICE_EPID)
//    - etc.
```

### Phase 3: Compatibility Layer
For existing code that uses syscalls:
```rust
// libc wrapper (user space)
pub fn open(path: *const c_char, flags: i32) -> i32 {
    unsafe { syscall(SYS_OPEN, path, flags) as i32 }
}

// Kernel delegates to service
pub fn sys_open(...) -> isize {
    ipc_call(FS_SERVICE, ...)
}
```

## Current Syscall Implementation Location

### In `process/syscall/mod.rs`:
- Core IPC syscalls ✓
- Process management ✓
- Memory management ✓

### In `process/syscall/exec.rs`:
- `execve()` - Uses fs::elf (needs delegation)
- `exec_builtin()` - Test code

### In `process/syscall/network.rs`:
- All network syscalls (needs delegation)

### In `process/syscall/process.rs`:
- Process lifecycle management ✓

### In `process/syscall/signal.rs`:
- Signal handling ✓

### In `process/syscall/driver.rs`:
- Driver management (needs review)

## Recommended Syscall Set for Microkernel

### Essential Syscalls (~20 total):
1. `fork()` - Process creation
2. `exit()` - Process termination  
3. `wait()` - Wait for child
4. `yield()` - Yield CPU
5. `getpid()` - Get PID
6. `mmap()` - Memory mapping
7. `munmap()` - Memory unmapping
8. `mprotect()` - Memory protection
9. `send()` - IPC send
10. `receive()` - IPC receive
11. `call()` - IPC call (send+receive)
12. `create_channel()` - Create IPC channel
13. `share_memory()` - Share memory
14. `create_mutex()` - Create mutex
15. `create_semaphore()` - Create semaphore
16. `grant()` - Grant capability
17. `revoke()` - Revoke capability
18. `duplicate()` - Duplicate capability
19. `debug()` - Debug output (kernel only)
20. `shutdown()` - System shutdown

All other syscalls (file, network, etc.) should be delegated to user space services via IPC.

## Implementation Plan

1. ✅ Identify core syscalls
2. ⏳ Design IPC delegation pattern
3. ⏳ Implement syscall stubs that delegate to services
4. ⏳ Create VFS service with IPC interface
5. ⏳ Create Network service with IPC interface
6. ⏳ Migrate fs/ syscalls to VFS service
7. ⏳ Migrate network/ syscalls to Network service
8. ⏳ Remove fs/ and network/ modules from kernel

---
*Last Updated: 2024-12-24*

# VFS Service Implementation

## Overview

VFS (Virtual File System) service is the first user space service for HNX microkernel. It handles all filesystem operations via IPC delegation from the kernel.

## Architecture

```
┌─────────────────────────────────────────┐
│          Kernel (EL1)                    │
│  ┌────────────────────────────────────┐ │
│  │ Syscall: sys_open("/bin/sh")       │ │
│  └─────────────┬──────────────────────┘ │
│                ↓                         │
│  ┌────────────────────────────────────┐ │
│  │ fs_stubs::sys_open()               │ │
│  │  - Validates path                  │ │
│  │  - Builds IPC message              │ │
│  └─────────────┬──────────────────────┘ │
│                ↓                         │
│  ┌────────────────────────────────────┐ │
│  │ ipc_delegate(VFS_EPID=1, VfsOpen)  │ │
│  │  - Sends message to endpoint 1     │ │
│  │  - Blocks waiting for response     │ │
│  └─────────────┬──────────────────────┘ │
└────────────────┼────────────────────────┘
                 │ IPC Message
                 ↓
┌─────────────────────────────────────────┐
│       VFS Service (EL0)                  │
│  ┌────────────────────────────────────┐ │
│  │ VfsIpcHandler::wait_and_handle()   │ │
│  │  - Receives IPC message            │ │
│  │  - Parses operation code           │ │
│  └─────────────┬──────────────────────┘ │
│                ↓                         │
│  ┌────────────────────────────────────┐ │
│  │ handle_open(msg)                   │ │
│  │  - Parses path from message        │ │
│  │  - Calls RamFs::open()             │ │
│  └─────────────┬──────────────────────┘ │
│                ↓                         │
│  ┌────────────────────────────────────┐ │
│  │ RamFs::open("/bin/sh")             │ │
│  │  - Lookups path in BTree           │ │
│  │  - Returns file descriptor         │ │
│  └─────────────┬──────────────────────┘ │
│                ↓                         │
│  ┌────────────────────────────────────┐ │
│  │ Send IPC response                  │ │
│  │  - Returns fd or error code        │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## Components

### 1. VFS Service Binary

**Location**: `space/services/vfs-service/`  
**Binary**: `target/aarch64-unknown-none/debug/vfs-service` (3.4MB ELF)

**Modules**:
- `main.rs`: Entry point, allocator, service loop
- `ramfs.rs`: RAM filesystem implementation
- `ipc.rs`: IPC message handlers

### 2. RAM Filesystem (`RamFs`)

Simple in-memory filesystem with:
- **Directory tree**: B-Tree based structure
- **Files**: Vec<u8> backed data
- **Operations**: mkdir, create, read, write, unlink

**Data Structure**:
```rust
enum Node {
    File { data: Vec<u8> },
    Dir { children: BTreeMap<String, usize> },
}
```

**Example**:
```
/              (inode 0) Dir { "/" -> 1, "tmp" -> 2 }
/bin           (inode 1) Dir { "sh" -> 3 }
/tmp           (inode 2) Dir { }
/bin/sh        (inode 3) File { data: [ELF bytes...] }
```

### 3. Heap Allocator

**Type**: Bump allocator (64KB heap)  
**Features**:
- Simple, lock-free allocation
- No deallocation (acceptable for long-running service)
- Atomic pointer for thread safety

### 4. IPC Protocol

**Endpoint ID**: 1 (well-known VFS endpoint)

**Operation Codes**:
```rust
VFS_OP_OPEN   = 100
VFS_OP_READ   = 101
VFS_OP_WRITE  = 102
VFS_OP_CLOSE  = 103
VFS_OP_CREAT  = 104
VFS_OP_UNLINK = 105
VFS_OP_MKDIR  = 106
VFS_OP_RMDIR  = 107
```

**Message Format** (256 bytes):
```
Offset  Size  Field
0       4     Operation code (u16 in u32)
4       4     Priority
8       4     Data length
12      244   Operation-specific data
```

## Build Information

**Target**: `aarch64-unknown-none`  
**Size**: 3.4MB (debug), ~200KB (release expected)  
**Type**: Statically linked ELF64  
**Dependencies**: hnx-abi, spin, alloc

**Build command**:
```bash
cd space/services/vfs-service
cargo build --target aarch64-unknown-none
```

## Integration with Kernel

### Kernel Side (Already Implemented)

1. **IPC Delegation Framework** (`kernel/src/ipc_services/`)
   - Well-known endpoint 1 for VFS
   - Service operation codes (100-199)
   - Delegation helpers

2. **Syscall Stubs** (`kernel/src/process/syscall/fs_stubs.rs`)
   - All FS syscalls delegate to VFS service
   - Example: `sys_open()` → `ipc_delegate(VFS_EPID, VfsOpen, ...)`

3. **User Process Support** (`kernel/src/user/`)
   - Can spawn VFS service as user process
   - Syscall: `HNX_SYS_PROCESS_CREATE`

### Service Side (This Implementation)

1. **Service Initialization**
   - Initialize heap allocator
   - Create root filesystem structure
   - Register endpoint 1

2. **Main Loop**
   - Wait for IPC messages on endpoint 1
   - Dispatch to operation handlers
   - Send responses back to kernel

3. **Operation Handlers**
   - Parse IPC message data
   - Execute filesystem operation
   - Format response

## Next Steps

### 1. Service Loading

**Current limitation**: VFS service is a separate ELF binary that needs to be:
- Embedded in initrd
- Loaded by kernel process loader
- Started at boot time

**Implementation needed**:
```rust
// In kernel init
fn spawn_vfs_service() {
    let vfs_elf = load_from_initrd("/services/vfs-service");
    let pid = sys_process_create_from_elf(vfs_elf);
    // VFS service starts and registers endpoint 1
}
```

### 2. IPC Implementation

**Current limitation**: `wait_and_handle()` is a stub

**Needs**:
- Actual IPC receive syscall (`HNX_SYS_EP_RECV`)
- Message parsing
- Response sending (`HNX_SYS_EP_SEND`)

### 3. Testing

Test end-to-end flow:
1. Kernel boots
2. Spawns VFS service
3. User process calls `open("/tmp/test")`
4. Kernel delegates to VFS service
5. VFS service processes request
6. Returns result to user

### 4. Performance Optimization

- Add caching for frequently accessed files
- Implement proper deallocation
- Optimize IPC message format
- Consider zero-copy for large transfers

## File Locations

**Source**:
- `space/services/vfs-service/src/main.rs`
- `space/services/vfs-service/src/ramfs.rs`
- `space/services/vfs-service/src/ipc.rs`
- `space/services/vfs-service/Cargo.toml`

**Binary**:
- `space/target/aarch64-unknown-none/debug/vfs-service` (3.4MB)

**Documentation**:
- `kernel/USER_PROCESS_IMPLEMENTATION.md`
- `kernel/IPC_DELEGATION_FRAMEWORK.md`
- `kernel/MIGRATION_STATUS.md`

## Status

✅ VFS service skeleton implemented  
✅ RamFs filesystem working  
✅ Heap allocator added  
✅ Binary builds successfully  
⏳ IPC handlers need implementation  
⏳ Service loader needed in kernel  
⏳ End-to-end testing pending  

---
**Created**: 2024-12-25  
**Status**: Skeleton complete, needs IPC integration

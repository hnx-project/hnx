# User Space Process Support Implementation

## Overview

User space process support has been implemented for the HNX microkernel. This enables the kernel to spawn and manage user space processes, which is a critical step toward running user space services (VFS, Network, etc.) via IPC delegation.

## Implementation Summary

### 1. New Module: `src/kernel/src/user/`

Created a dedicated module for user space process management:

- **`user/mod.rs`**: Module exports
- **`user/loader.rs`**: User program loading and memory setup
  - `UserProgram` struct: Contains entry point, stack pointer, and page table base
  - `create_simple_user_program()`: Creates a user process from raw code bytes
- **`user/spawn.rs`**: Process spawning syscall
  - `sys_process_create()`: Syscall handler that creates and schedules user processes

### 2. New Syscall: `HNX_SYS_PROCESS_CREATE` (0x0101 / 257)

Added to ABI and syscall dispatch:

```rust
HNX_SYS_PROCESS_CREATE => {
    crate::debug!("syscall enter process_create");
    crate::user::sys_process_create(x0, x1)
}
```

**Parameters:**
- `x0`: Code pointer (if 0, uses built-in test code `user_store_ok_svc1`)
- `x1`: Code length in bytes (max 4096)

**Returns:**
- PID on success
- -1 on error

### 3. User Program Memory Layout

User processes are created with the following memory layout:

```
0x0000_1000  Code segment    (RX + USER)
0x0000_2000  Data segment    (RW + USER)
0x0010_0000  Stack pointer   (grows down)
```

MMU flags ensure proper isolation:
- Code: `READ | EXECUTE | USER`
- Data: `READ | WRITE | USER`
- Stack: `READ | WRITE | USER`

### 4. Integration Points

**Modified files:**
- `src/kernel/src/main.rs`: Added `mod user;`
- `src/kernel/src/process/syscall/mod.rs`: Added `HNX_SYS_PROCESS_CREATE` import and dispatch
- `src/kernel/src/arch/aarch64/boot/mod.rs`: Enabled `selftest.S` assembly for `user_store_ok_svc1`

### 5. Built-in Test Code

Re-enabled the assembly test snippet `user_store_ok_svc1` from `arch/aarch64/selftest.S`:

```asm
user_store_ok_svc1:
  mov x0, x20
  movz w1, #0x4B4F       ; "OK"
  movk w1, #0x000A, lsl #16  ; newline
  str w1, [x0]
  svc #1
  b .
```

This provides a simple test program for verifying user space execution.

## Architecture

```
User Space Process Creation Flow:
┌─────────────────────────────────────────────────────┐
│ 1. Syscall: HNX_SYS_PROCESS_CREATE(code_ptr, len)  │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 2. user/spawn.rs: sys_process_create()             │
│    - Validate parameters                            │
│    - Get code bytes (builtin or user-provided)      │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 3. user/loader.rs: create_simple_user_program()    │
│    - Allocate user page table (TTBR0)               │
│    - Allocate code and data pages                   │
│    - Map pages with USER permissions                │
│    - Copy code to physical memory                   │
│    - Clean D-cache, flush I-cache                   │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 4. process/mod.rs: create_process(priority)        │
│    - Allocate PID                                   │
│    - Create PCB entry                               │
│    - Set initial state: Created                     │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 5. process/task.rs: Task::new_kernel()             │
│    - Create task structure                          │
│    - Set TTBR0 (user page table)                    │
│    - Set user SP and entry point                    │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 6. core/scheduler: run_task()                      │
│    - Set process state: Ready                       │
│    - Switch to EL0 (user mode)                      │
│    - Execute user code                              │
└─────────────────────────────────────────────────────┘
```

## Security Considerations

1. **Privilege Level Separation**: User code runs at EL0, kernel at EL1
2. **Memory Protection**: Page tables enforce USER bit for all user pages
3. **MMU Flags**:
   - UXN (User Execute Never) prevents user from executing kernel pages
   - PXN (Privileged Execute Never) prevents kernel from executing user pages
4. **Address Space Isolation**: Each process has its own TTBR0 page table
5. **Cache Coherency**: D-cache clean and I-cache flush ensure code integrity

## Next Steps

With user space process support in place, the following can be implemented:

1. **VFS Service**: First user space service
   - Responds to IPC messages on endpoint 1
   - Handles filesystem operations delegated by kernel

2. **Network Service**: Second user space service
   - Responds to IPC messages on endpoint 2
   - Handles network operations

3. **Process Loader Service**: Advanced process spawning
   - Load ELF binaries from initrd
   - Dynamic linking support
   - Proper argument/environment passing

4. **Service Manager**: Coordinate user space services
   - Start/stop services
   - Service health monitoring
   - Service dependencies

## Testing

To test user process creation:

```c
// From user space (once available):
int pid = syscall(HNX_SYS_PROCESS_CREATE, 0, 0);  // Use builtin test code

// From kernel (for testing):
let pid = crate::user::sys_process_create(0, 0);
```

## Build Status

✅ Kernel builds successfully with user process support  
✅ `user_store_ok_svc1` assembly symbol resolved  
✅ `HNX_SYS_PROCESS_CREATE` syscall integrated  
✅ All module dependencies satisfied  

## Files Modified/Created

**Created:**
- `src/kernel/src/user/mod.rs`
- `src/kernel/src/user/loader.rs`
- `src/kernel/src/user/spawn.rs`

**Modified:**
- `src/kernel/src/main.rs`
- `src/kernel/src/process/syscall/mod.rs`
- `src/kernel/src/arch/aarch64/boot/mod.rs`

## Compilation Output

```
Compiling hnx-abi v0.0.1
Compiling hnx-kernel v0.0.1
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.01s
```

No errors, ready for testing and VFS service implementation.

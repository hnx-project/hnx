# HNX Microkernel - Development Guide

## Project Overview

**HNX** is a modern microkernel operating system written in Rust, designed with principles of minimalism, security, and modularity. The kernel provides only essential functionality (process management, memory management, IPC, interrupt handling), while all other services (filesystem, networking, device drivers) run in user space as isolated processes communicating via IPC.

### Key Characteristics
- **Microkernel Architecture**: Minimal trusted computing base (TCB)
- **Capability-Based Security**: Fine-grained access control for resources
- **Modern IPC**: Priority-based message passing with zero-copy support
- **Multi-Architecture**: Primary target is AArch64 (ARMv8), with support planned for RISC-V and x86_64
- **Rust-Based**: Memory safety, no undefined behavior, no data races in kernel code

### Current Status
- **Version**: 0.1.0-alpha.1 (see `VERSION` file)
- **State**: Actively developed, basic kernel functionality operational
- **Recent Focus**: System call implementation, IPC delegation, user-space process loading

## Technology Stack

### Core Languages & Tools
- **Kernel**: Rust (no_std, 2021 edition)
- **User Space**: Rust (with libc compatibility layer)
- **Build System**: Cargo + Make + Python scripts
- **Target Architecture**: `aarch64-unknown-none` (bare metal)
- **Emulation**: QEMU virt machine for testing

### Key Dependencies
- **Kernel**: `spin` (synchronization), `cortex-a` (ARM), `heapless` (no-std collections), `bitflags`
- **User Space**: `log`, `anyhow`, `thiserror`, `hnx-libc` (custom libc)
- **Build Tools**: Poetry (Python environment management)

## Project Structure

```
hnx-core/
├── src/                          # Source code
│   ├── kernel/                   # Kernel crate (library)
│   │   ├── src/                  # Kernel source
│   │   │   ├── arch/            # Architecture-specific (aarch64)
│   │   │   ├── core/            # IPC, scheduler core
│   │   │   ├── memory/          # Physical/virtual memory management
│   │   │   ├── process/         # Process/task management
│   │   │   ├── ipc_services/    # IPC service delegation framework
│   │   │   ├── drivers/         # Minimal hardware drivers
│   │   │   ├── console/         # Debug console (UART)
│   │   │   └── main.rs          # Kernel entry point
│   │   └── Cargo.toml           # Kernel dependencies
│   ├── space/                    # User space workspace
│   │   ├── libc/                # Custom C library implementation
│   │   ├── hnxlib/              # HNX system library (IPC wrappers)
│   │   ├── services/            # System services (init, VFS, loader)
│   │   └── Cargo.toml           # Workspace configuration
│   ├── abi-bindings/            # Kernel-user ABI definitions
│   └── utils/                   # Build utilities (optional)
├── configs/                      # Build configuration
│   ├── arch/                    # Architecture-specific configs
│   ├── board/                   # Board-specific configs (qemu-virt)
│   └── profile/                 # Build profiles (debug, release)
├── scripts/                      # Python build scripts
│   ├── configure.py             # Generate build configuration
│   ├── create-image.py          # Create bootable system image
│   ├── run-qemu.py              # Launch QEMU with proper args
│   ├── version.py               # Version management
│   └── verify_version.py        # Version consistency checker
├── include/                      # C headers (ABI definitions)
├── targets/                      # Rust target specifications
├── toolchain/                    # Cross-compilation toolchain
├── tests/                        # Test framework
├── tools/                        # Development utilities
├── Cargo.toml                    # Root workspace configuration
├── Makefile                      # Primary build interface
├── pyproject.toml                # Python tooling (Poetry)
├── rust-toolchain.toml           # Rust toolchain specification
└── VERSION                       # Version metadata (TOML)
```

## Building and Running

### Prerequisites
- **Rust Toolchain**: Install via rustup with `rustup target add aarch64-unknown-none`
- **Python 3.11+**: Required for build scripts
- **Poetry**: Python dependency management (`pip install poetry`)
- **QEMU**: System emulator (`brew install qemu` on macOS)
- **Cross-Compilation Tools**: Optional for custom toolchain

### Initial Setup
```bash
# Install Python dependencies
poetry install

# Initialize version file (first time)
make version-init

# Configure for default target (aarch64, qemu-virt)
make configure
```

### Common Build Commands
```bash
# Show help with all available targets
make help

# Build everything (kernel + user space + system image)
make all

# Build kernel only
make kernel

# Build user space components only
make space

# Create system image (requires kernel and space)
make image

# Create simple image for quick testing
make simple-image

# Quick build (kernel only, no configuration)
make quick

# Clean build artifacts (preserves configs)
make clean

# Full clean (including configs and version files)
make distclean
```

### Running in QEMU
```bash
# Run full system image
make run

# Run kernel only with simple initrd (30s timeout)
make run-simple

# Run kernel only (no timeout)
make run-kernel

# Run with GDB debugging
make debug
```

### Version Management
```bash
# Show current version
make version

# Sync version to all subprojects
make version-sync

# Check version consistency
make version-check

# Bump version numbers
make version-bump-major   # 1.0.0 → 2.0.0
make version-bump-minor   # 1.0.0 → 1.1.0
make version-bump-patch   # 1.0.0 → 1.0.1

# Set prerelease tag
make version-set-prerelease TAG=beta.1
```

### Testing
```bash
# Run unit tests for kernel and user space
make test

# Check ABI consistency between kernel and user space
make check-abi
```

## Development Workflow

### Architecture
The system follows a strict layered architecture:

1. **Kernel (EL1/ring 0)**: Minimal core providing:
   - Process scheduling and context switching
   - Virtual memory management (page tables)
   - IPC message passing endpoints
   - Interrupt and exception handling
   - Capability system for resource access control

2. **User Space Services (EL0/ring 3)**: Isolated processes:
   - **VFS Service**: Filesystem operations
   - **Init Process**: First user process, spawns other services
   - **Loader Service**: Process loading and ELF parsing
   - **Device Drivers**: Hardware access in user space

3. **System Libraries**: Shared between user processes:
   - **hnxlib**: IPC wrappers, capability management
   - **libc**: Standard C library compatibility

### System Call Implementation
System calls follow the AArch64 SVC convention:
- System call number in register `x8`
- Arguments in `x0-x5`
- Return value in `x0`
- Error handling via negative return values

Current system calls (see `hnx_abi::syscalls`):
- `HNX_SYS_WRITE` (4097): Write to file descriptor
- `HNX_SYS_EXIT` (4098): Terminate process
- `HNX_SYS_FORK` (4099): Create child process
- `HNX_SYS_EXEC` (4100): Execute new program

### IPC Framework
The IPC system provides:
- **Endpoints**: Communication channels with capabilities
- **Message Queues**: Priority-based (critical, high, normal, low)
- **Service Delegation**: Kernel-assisted RPC to user space services
- **Zero-Copy**: Shared memory regions for large data transfers

Services are identified by well-known endpoint IDs:
- `VFS_EPID = 1`: Filesystem operations
- `LOADER_EPID = 2`: Process loading
- `DEVICE_MANAGER_EPID = 3`: Device access

### Memory Management
- **Physical**: Buddy allocator for page frames
- **Virtual**: 3-level page tables (L1, L2, L3) with 4KB pages
- **User Space**: Separate page tables per process (TTBR0)
- **Kernel Space**: Shared high-half mapping (TTBR1)
- **Security**: User/kernel isolation, permission checking

## Coding Conventions

### Rust Style
- **No Standard Library**: Use `#![no_std]` with core/alloc as needed
- **Error Handling**: Use `Result` with custom error types
- **Unsafe Code**: Minimize unsafe blocks, document invariants
- **Documentation**: Use Rustdoc with examples for public APIs

### Kernel-Specific Patterns
- **Synchronization**: Use `spin::Mutex` for kernel data structures
- **Interrupt Safety**: Disable interrupts when holding locks
- **Memory Safety**: Validate all user pointers before dereferencing
- **Panic Policy**: Kernel panics are fatal; use `panic::abort`

### Logging System
The kernel uses a structured logging system with levels:
- `error!`: Critical failures that may affect system stability
- `warn!`: Non-critical issues that should be addressed
- `info!`: General operational information (default)
- `debug!`: Detailed debugging information
- `trace!`: Very verbose tracing for deep investigation

Set log level via boot command line: `log=debug`

## Debugging and Troubleshooting

### Common Issues
1. **System Call Failures**: Check register values in SVC handler logs
2. **Page Table Issues**: Enable `user_range_ok` debugging in `process/syscall/mod.rs`
3. **IPC Deadlocks**: Add logging to `endpoint_send_sync` in `core/ipc/mod.rs`
4. **Process Creation**: Verify ELF loading in `loader/bootstrap_elf.rs`

### Debugging Commands
```bash
# Run with GDB (port 1234)
make debug

# Check kernel logs in QEMU
# Look for [INFO], [WARN], [ERROR] prefixes

# Examine build configuration
cat build/config/config.json
```

### Log Analysis
Key log patterns to monitor:
- `arch/aarch64 SVC handler entered`: System call entry
- `sys_write: copying`: User buffer access
- `ipc_delegate: starting`: IPC service delegation
- `user_range_ok: TEMPORARY BYPASS`: Page table check disabled (debug)
- `current_ttbr0_base: temporary bypass`: Lock issue workaround

## Configuration

### Architecture and Board
- **Default**: `ARCH=aarch64`, `BOARD=qemu-virt`
- **Override**: `ARCH=riscv64 make configure`
- **Profiles**: `PROFILE=release make kernel`

### Build Configuration
Configuration files are generated in `build/config/`:
- `config.json`: Build parameters and paths
- `env.sh`: Environment variables for build scripts
- `Makefile.inc`: Makefile includes for customization

### Cargo Configuration
The `.cargo/config.toml` file defines custom Rust targets and build settings:
- **Custom Targets**: `aarch64-unknown-hnx-kernel` (kernel) and `aarch64-unknown-hnx` (user space)
- **Linker**: Uses `rust-lld` with custom linker scripts
- **Build Std**: Builds core, alloc, and compiler_builtins from source
- **Environment Variables**: Sets `ARCH`, `KERNEL_TARGET`, `SPACE_TARGET`

Note: The Makefile currently uses the standard `aarch64-unknown-none` target, but custom targets are available for advanced configurations.

### Kernel Configuration
Kernel features can be enabled/disabled via Cargo features:
- `version`: Include version information in kernel binary
- (Future) `smp`: Symmetric multiprocessing support
- (Future) `virtio`: VirtIO device drivers

## Contributing Guidelines

### Workflow
1. **Fork and Branch**: Create feature branches from `main`
2. **Testing**: Run `make test` before committing
3. **Versioning**: Use `make version-bump-*` for release changes
4. **Documentation**: Update QWEN.md for significant changes

### Code Review Checklist
- [ ] No undefined behavior in unsafe blocks
- [ ] User pointer validation before use
- [ ] Proper error handling for all fallible operations
- [ ] Logging at appropriate levels
- [ ] Documentation for public APIs
- [ ] Cross-architecture compatibility considered

### Commit Messages
Follow conventional commit format:
```
type(scope): description

Body explaining the change and rationale.

Fixes #issue
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## Future Roadmap

### Short Term (v0.2.0)
- [ ] Complete system call implementation
- [ ] Fix page table mapping issues
- [ ] Resolve IPC lock deadlocks
- [ ] Add more user space services

### Medium Term (v0.5.0)
- [ ] RISC-V architecture support
- [ ] VirtIO device drivers
- [ ] Network stack service
- [ ] Graphical display server

### Long Term (v1.0.0)
- [ ] SMP support for multi-core
- [ ] Production-ready stability
- [ ] Comprehensive test suite
- [ ] Security audit and certification

## Resources

### Documentation
- `docs/`: Project documentation (in progress)
- `include/hnx/abi/`: ABI header files
- Rustdoc: `cargo doc --open` for API documentation

### External References
- [AArch64 Reference Manual](https://developer.arm.com/documentation/ddi0487/latest)
- [Rust Embedded Book](https://docs.rust-embedded.org/book/)
- [Microkernel Design Patterns](https://www.cs.cmu.edu/~dga/papers/microkernels-newton-2017.pdf)

### Tooling
- `tools/`: Development utilities
- `.vscode/`: IDE configuration for VS Code
- `.github/`: CI/CD workflows

---

*Last Updated: $(date)*  
*Version: $(cat VERSION | grep -E "major|minor|patch" | head -3 | tr '\n' ' ')*
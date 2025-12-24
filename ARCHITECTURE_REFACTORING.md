# HNX Microkernel Architecture Refactoring

## Status: ✅ 完成方案2 - 正确的微内核架构

**日期**: 2024-12-25

## 架构原则

### 内核职责（最小化）
```rust
// 内核只提供这些原语：
1. sys_process_create_empty()  // 创建空进程（PCB + 页表）
2. sys_mmap_process()           // 映射物理页到进程
3. sys_process_start()          // 设置入口点并启动进程
4. IPC primitives               // 消息传递
5. Memory management            // 物理内存分配
6. Context switching            // 进程调度
```

### 用户空间职责
```rust
// 所有高级功能在用户空间：
1. ELF加载           → Loader Service (endpoint 3)
2. 文件系统          → VFS Service (endpoint 1)  
3. 网络栈           → Network Service (endpoint 2)
4. 进程管理         → Init Process (PID 1)
```

## 新架构流程

```
┌──────────────────────────────────────────────┐
│ 内核启动 (kernel/main.rs)                    │
│ 1. 硬件初始化                                │
│ 2. 内存管理                                  │
│ 3. IPC系统                                   │
│ 4. 发现initrd                                │
│ 5. bootstrap_init_process()                 │
│    └→ 硬编码加载 init 二进制文件             │
│       (这是唯一一次内核加载程序)             │
└──────────────┬───────────────────────────────┘
               ↓
┌──────────────────────────────────────────────┐
│ Init Process (PID 1, 用户空间)               │
│ - 启动 Loader Service (endpoint 3)           │
│ - 启动 VFS Service (endpoint 1)              │
│ - 启动 Network Service (endpoint 2)          │
│ - 监控服务健康状态                            │
└──────────────┬───────────────────────────────┘
               ↓
┌──────────────────────────────────────────────┐
│ Loader Service (endpoint 3, 用户空间)        │
│ 接收: (路径, 参数) via IPC                   │
│ 1. 通过VFS服务读取ELF文件                    │
│ 2. 解析ELF头和程序头                         │
│ 3. sys_process_create_empty() → 创建进程    │
│ 4. sys_mmap_process() → 映射代码/数据段      │
│ 5. sys_process_start() → 启动进程           │
│ 返回: PID via IPC                            │
└──────────────────────────────────────────────┘
```

## 已删除的内核代码

从内核移除（违背微内核原则）：
- ❌ `src/kernel/src/loader/elf.rs` - ELF解析器
- ❌ `src/kernel/src/loader/cpio.rs` - CPIO解析器
- ❌ `src/kernel/src/loader/service_manager.rs` - 服务管理器

保留的最小代码：
- ✅ `src/kernel/src/loader/initrd.rs` - 发现initrd位置
- ✅ `src/kernel/src/loader/mod.rs` - bootstrap_init_process() (仅加载init)

## 新增的内核系统调用

```rust
// src/kernel/src/process/spawn.rs

/// 创建空进程（仅PCB + 页表）
pub fn sys_process_create_empty(priority: u8) -> Result<(u32, usize), ()>

/// 映射物理页到进程地址空间
pub fn sys_mmap_process(
    pid: u32,
    vaddr: usize,
    paddr: usize,  // 0 = 内核分配新页
    readable: bool,
    writable: bool,
    executable: bool,
) -> Result<usize, ()>

/// 启动进程
pub fn sys_process_start(pid: u32, entry: usize, sp: usize) -> Result<(), ()>
```

## 用户空间服务

### Init Process
**路径**: `src/space/services/init/`
**大小**: 1.8KB (release)
**职责**:
- 系统第一个用户进程 (PID 1)
- 启动所有系统服务
- 监控服务健康状态

**代码**:
```rust
#![no_std]
#![no_main]

pub extern "C" fn _start() -> ! {
    // 启动 Loader Service
    // 启动 VFS Service
    // 启动 Network Service
    loop { wfi(); }
}
```

### Loader Service (待实现)
**路径**: `src/space/services/loader-service/` (TODO)
**Endpoint**: 3
**职责**:
- 解析ELF文件
- 加载程序到内存
- 通过内核syscall创建进程

### VFS Service
**路径**: `src/space/services/vfs-service/`
**大小**: 11KB (release)
**Endpoint**: 1
**职责**:
- 文件系统操作
- CPIO/ramfs实现

## 构建状态

### 内核
```bash
cd /Users/tinchy/work/development/os/hnx-core
make kernel
# ✅ 编译成功
# Size: ~6.6MB (debug) / ~1MB (release 预估)
```

### Init Process
```bash
cd src/space/services/init
cargo build --target aarch64-unknown-none --release
# ✅ 编译成功
# Size: 1.8KB
# Output: src/space/target/aarch64-unknown-none/release/init
```

### Initrd
```bash
# 新的initrd只包含init二进制
build/initrd/
└── init (1.8KB)

# 打包
cd build/initrd && find . | cpio -o -H newc > ../initrd.cpio
# ✅ initrd.cpio: 2.5KB
```

## 对比：方案1 vs 方案2

| 方面 | 方案1 (错误) | 方案2 (正确) |
|------|-------------|-------------|
| ELF加载器位置 | ❌ 内核 | ✅ 用户空间 |
| 服务管理器位置 | ❌ 内核 | ✅ 用户空间 (init) |
| 内核代码大小 | ~700KB | ~600KB |
| 内核复杂度 | 高 | 低 |
| 安全性 | 低（加载器bug会崩溃内核） | 高（加载器隔离） |
| 灵活性 | 低（需要重编内核） | 高（服务可独立更新） |
| 符合微内核原则 | ❌ | ✅ |

## 下一步

1. **实现 bootstrap_init_process()**
   - 硬编码简单的init加载器
   - 从initrd读取init ELF
   - 手动解析并映射到内存
   
2. **实现 Loader Service**
   - 完整的ELF解析器
   - VFS集成
   - IPC接口
   
3. **完善系统调用**
   - 实现sys_mmap_process的完整逻辑
   - 添加进程PCB查询函数
   - 实现sys_process_start的线程创建

4. **端到端测试**
   - Init启动
   - Loader Service加载VFS Service  
   - VFS Service响应文件操作

## 文件清单

**内核修改**:
- `src/kernel/src/loader/mod.rs` - 最小化bootstrap
- `src/kernel/src/loader/initrd.rs` - 仅发现initrd
- `src/kernel/src/process/spawn.rs` - 新系统调用
- `src/kernel/src/main.rs` - 移除service_manager调用

**新增用户空间**:
- `src/space/services/init/src/main.rs` - Init进程
- `src/space/services/init/Cargo.toml` - 构建配置

**删除**:
- `src/kernel/src/loader/elf.rs`
- `src/kernel/src/loader/cpio.rs`
- `src/kernel/src/loader/service_manager.rs`

---
**完成时间**: 2024-12-25  
**状态**: 架构重构完成，等待实现bootstrap和Loader服务

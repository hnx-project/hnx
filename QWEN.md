# AGENTS.md

This file provides guidance to Qoder (qoder.com) when working with code in this repository.

## 项目概述

HNX是一个基于Rust编写的微内核操作系统，采用类似Zircon的能力安全模型。

### 关键设计原则
- **微内核架构**: 内核仅提供核心功能（IPC、进程管理、内存管理、中断处理），文件系统、网络、驱动运行在用户空间
- **能力安全模型**: 所有操作需要对应能力，系统调用和IPC均需权限验证
- **IPC委托框架**: 系统调用可委托给用户空间服务处理（VFS、网络、加载器等）
- **跨平台支持**: aarch64, x86_64, riscv64

### 项目结构
```
hnx-core/
├── include/hnx/abi/        # C头文件（生成Rust绑定）
├── scripts/                # Python构建脚本（版本管理、镜像创建、QEMU运行）
├── configs/                # 系统配置（TOML格式）
├── targets/                # Rust目标定义（aarch64-unknown-none等）
├── src/
│   ├── abi-bindings/       # ABI定义crate（系统调用、能力、类型）
│   ├── kernel/             # 微内核核心（库crate）
│   │   └── src/
│   │       ├── main.rs     # 内核入口（4阶段启动流程）
│   │       ├── arch/       # 架构相关（aarch64/x86_64/riscv64）
│   │       │   └── aarch64/
│   │       │       ├── boot.S      # 汇编启动代码
│   │       │       ├── mmu/        # MMU初始化和页表管理
│   │       │       ├── exceptions/ # 异常/中断处理
│   │       │       └── context.rs  # 上下文切换
│   │       ├── core/       # IPC和调度器
│   │       │   ├── ipc/    # 消息传递、共享内存
│   │       │   └── scheduler/ # 进程调度
│   │       ├── memory/     # 内存管理
│   │       │   ├── physical/  # 物理内存分配器（buddy allocator）
│   │       │   └── virtual_/  # 虚拟内存管理（页表操作）
│   │       ├── process/    # 进程/线程管理
│   │       ├── ipc_services/ # IPC服务委托框架（well-known endpoints）
│   │       ├── loader/     # ELF/CPIO加载器
│   │       ├── drivers/    # 最小驱动（UART、GIC、DTB解析）
│   │       └── scripts/    # 链接脚本（link.ld.S）
│   └── space/              # 用户空间workspace
│       ├── libc/           # C标准库实现
│       ├── hnxlib/         # HNX系统库（封装系统调用）
│       └── services/       # 系统服务
│           ├── init/       # Init进程（第一个用户态进程）
│           ├── vfs-service/    # 文件系统服务
│           └── loader-service/ # 程序加载器服务
└── build/                  # 构建产物（不提交到Git）
```

## 开发环境设置

### 环境要求
- **Rust toolchain**: nightly版本，支持`no_std`和目标架构
- **Python环境**: Poetry管理 (Python 3.11+)
- **编译目标**: 默认 `aarch64-unknown-none`
- **QEMU**: 用于运行和调试（qemu-system-aarch64）

### 初始化
```bash
make init              # 初始化Poetry Python环境
make configure         # 配置系统（默认ARCH=aarch64 BOARD=qemu-virt）
```

### 环境变量
- `ARCH`: 目标架构 (aarch64, x86_64, riscv64)
- `BOARD`: 开发板 (qemu-virt, raspberry-pi4)
- `PROFILE`: 构建配置 (debug, release)

## 构建命令

### 请勿使用
```bash
timeout xx make run-simple # 因为脚本自带超时 30s
```
### 完整构建流程
```bash
make configure          # 配置系统 (ARCH=aarch64 BOARD=qemu-virt)
make kernel            # 构建微内核（包含版本同步和ABI检查）
make space             # 构建用户空间组件
make image             # 创建完整系统镜像（带压缩）
make simple-image      # 创建简单系统镜像（无压缩）
make initrd-simple     # 创建简单initrd.cpio
```

### 快速构建
```bash
make quick             # 快速构建内核（release模式）
make check-abi         # 仅检查ABI一致性
```

### 测试与运行
```bash
make test              # 运行所有测试（内核+用户空间）
make run               # 在QEMU中运行完整系统
make run-kernel        # 在QEMU中仅运行微内核
make run-simple        # 30秒超时测试运行（最常用）
make debug             # 启动QEMU GDB调试会话
```

### 版本管理
```bash
make version                    # 显示版本信息
make version-init               # 初始化版本文件
make version-sync               # 同步版本到所有子项目
make version-check              # 检查版本一致性
make version-bump-patch         # 递增修订版本
make version-bump-minor         # 递增次版本
make version-bump-major         # 递增主版本
make version-set-prerelease TAG=alpha.1  # 设置预发布标签
```

### 清理
```bash
make clean             # 清理构建产物（保留版本文件）
make distclean         # 完全清理（包括版本文件）
make version-clean     # 清理版本生成的文件
```

## 代码架构

### 微内核职责边界

**内核核心职责**（必须在内核实现）：
- 进程/线程调度与管理（`src/kernel/src/core/scheduler/`）
- IPC消息传递与共享内存（`src/kernel/src/core/ipc/`）
- 内存管理（分页、物理分配）（`src/kernel/src/memory/`）
- 能力安全模型检查（`src/kernel/src/security/`）
- 系统调用分发（`src/kernel/src/syscall/`）
- 中断/异常处理（`src/kernel/src/arch/*/exceptions/`）
- 最小设备驱动（UART调试输出、GIC中断控制器、定时器）

**用户空间职责**（必须移出内核）：
- 文件系统 → `src/space/services/vfs-service/`
- 网络协议栈 → （待实现）
- 设备驱动 → `src/space/drivers/`
- 调试工具 → （待实现）

**注意**: 当前 `src/kernel/src/` 中存在 `fs.moved/`, `network.moved/`, `debug.moved/` 等目录，这些是待迁移到用户空间的遗留代码，不应在内核中继续使用。

### 内核启动流程（4阶段）

内核从 `src/kernel/src/main.rs` 的 `rust_main()` 开始，经过4个初始化阶段：

1. **Phase 1: Hardware** (`init_phase1_hardware`)
   - 初始化UART调试控制台
   - 解析Device Tree Blob (DTB)
   - 初始化GIC中断控制器和定时器
   - 架构相关初始化（异常向量表、MMU基本配置）

2. **Phase 2: Memory** (`init_phase2_memory`)
   - 初始化物理内存分配器（buddy allocator）
   - 设置虚拟内存管理（页表操作）
   - 初始化内核堆分配器

3. **Phase 3: Processes** (`init_phase3_processes`)
   - 初始化进程控制块（PCB）管理
   - 初始化IPC服务委托框架（well-known endpoints）
   - 初始化initrd访问器（加载用户空间程序）
   - 加载init进程（第一个用户态进程）

4. **Phase 4: Scheduler** (`init_phase4_scheduler`)
   - 创建idle任务
   - 启动调度器（永不返回）

### MMU和内存布局（aarch64）

**当前实现**: 恒等映射（Identity Mapping）

- **内核地址空间**: 虚拟地址 = 物理地址
  - 物理基址: 0x40000000
  - 内核代码/数据映射: 0x40000000-0x4FFFFFFF (256MB)
  - 页表位置: 
    - `L1_TABLE_KERNEL`: 0x40081000 (TTBR1_EL1)
    - `L2_TABLE_KERNEL`: 0x40084000
    - `L1_TABLE_USER`: 0x40082000 (TTBR0_EL1 初始值)
    - `L2_TABLE_USER`: 0x40083000

- **用户地址空间**: 每个进程独立页表（TTBR0_EL1）
  - 进程代码/数据: 0x00000000-低地址区域
  - 进程栈: 通常在高用户地址
  - ASID支持: 进程隔离使用Address Space ID

**关键代码**:
- MMU启动: `src/kernel/src/arch/aarch64/mmu/mmu_boot.rs`
- Boot汇编: `src/kernel/src/arch/aarch64/boot.S`
- 链接脚本: `src/kernel/src/scripts/link.ld.S`

### IPC服务委托框架

HNX使用**well-known endpoint**机制将部分系统调用委托给用户空间服务处理：

**架构**:
```
用户进程 → syscall(SYS_OPEN, ...) → 内核 → ipc_delegate(EPID_VFS, OP_OPEN, msg)
                                            ↓
                                    VFS服务接收消息并处理
                                            ↓
                                    返回结果给用户进程
```

**Well-known Endpoints**:
- `EPID_VFS` (1): 文件系统服务
- `EPID_NETWORK` (2): 网络服务
- `EPID_LOADER` (3): 程序加载器服务

**关键代码**:
- 框架实现: `src/kernel/src/ipc_services/delegate.rs`
- 服务定义: `src/kernel/src/ipc_services/endpoints.rs`
- 文档: `src/kernel/src/ipc_services/IPC_DELEGATION_FRAMEWORK.md`

## 测试策略

### 内核测试
```bash
cd src/kernel && cargo test --lib
```
- 单元测试位于各模块的 `#[cfg(test)]` 块中
- 主要测试内存分配器、页表操作、IPC消息构建等

### 用户空间测试
```bash
cd src/space && cargo test --workspace
```
- 测试系统库（hnxlib）封装
- 测试服务逻辑

### QEMU集成测试
```bash
make run-simple  # 30秒超时，检查启动和init进程执行
```

### GDB调试
```bash
make debug  # 启动QEMU并等待GDB连接
# 在另一个终端:
lldb-mi2 -- build/kernel/debug/hnx-kernel-*.bin
(lldb) gdb-remote localhost:1234
(lldb) b rust_main
(lldb) c
```

## 版本管理

- **语义化版本** (SemVer): `major.minor.patch[-prerelease]`
- **版本文件**: 项目根目录的 `VERSION` 文件
- **同步机制**: `scripts/version.py` 自动同步到：
  - `include/hnx/abi/version.h` (C头文件)
  - `src/kernel/Cargo.toml`
  - `src/space/Cargo.toml`
  - 所有子项目的 `Cargo.toml`

**工作流**:
1. 修改代码后运行 `make version-bump-patch`
2. 运行 `make version-sync` 同步版本号
3. 运行 `make version-check` 验证一致性
4. 构建时自动使用新版本号（镜像命名: `hnx-{version}-{arch}-{board}.img`）

## 安全注意事项

- **能力模型**: 所有操作（系统调用、IPC）需要验证能力
- **IPC安全检查**: 消息传递时验证发送者权限
- **内存隔离**: 进程间地址空间完全隔离（通过ASID和独立页表）
- **系统调用验证**: 参数有效性和权限双重检查
- **UART仅用于调试**: 生产环境应移除或禁用UART输出

## 常见开发任务

### 添加新的系统调用
1. 在 `include/hnx/abi/syscalls.h` 定义系统调用号
2. 运行 `make check-abi` 生成Rust绑定
3. 在 `src/kernel/src/syscall/mod.rs` 实现系统调用处理
4. 如需委托，在 `src/kernel/src/ipc_services/delegate.rs` 添加委托逻辑
5. 在 `src/space/hnxlib/src/` 添加用户空间封装

### 添加新的用户空间服务
1. 在 `src/space/services/` 创建新crate
2. 在 `src/space/Cargo.toml` 添加为workspace成员
3. 实现服务逻辑（监听well-known endpoint）
4. 在 `src/kernel/src/ipc_services/endpoints.rs` 注册endpoint
5. 更新initrd构建脚本将服务包含到镜像

### 修改MMU页表布局
1. 修改 `src/kernel/src/arch/aarch64/mmu/mmu_boot.rs`
2. 同步更新 `src/kernel/src/scripts/link.ld.S`
3. 确保 `boot.S` 中的汇编代码与新布局兼容
4. 运行 `make run-simple` 验证启动流程
5. 使用 `make debug` 调试页表配置

### 调试内核崩溃
1. 添加调试输出: 使用 `info!()`, `warn!()`, `error!()` 宏
2. QEMU调试输出: `make run-simple` 查看串口输出
3. QEMU异常追踪: 在 `scripts/run-qemu.py` 添加 `-d int,in_asm`
4. GDB调试: `make debug` 启动，设置断点追踪执行流
5. 检查页表: 在GDB中 `x/gx <page_table_address>` 查看页表项

## 构建系统细节

### Python脚本说明
- `scripts/configure.py`: 生成板级配置文件（JSON格式）
- `scripts/create-image.py`: 创建系统镜像（包含内核+initrd）
  - `--simple-initrd`: 创建简单CPIO格式initrd
  - `--no-compress`: 不压缩镜像
- `scripts/run-qemu.py`: 启动QEMU模拟器
  - `--headless`: 无图形界面
  - `--gdb`: 启动GDB服务器（端口1234）
  - `--timeout`: 设置超时（秒）
- `scripts/version.py`: 版本管理工具
- `scripts/verify_version.py`: 验证版本一致性
- `scripts/verify_abi.py`: 验证ABI一致性

### Cargo配置文件
- 根工作区: `Cargo.toml` (包含内核)
- 用户空间工作区: `src/space/Cargo.toml` (包含所有服务和库)
- 内核crate: `src/kernel/Cargo.toml`
- 构建配置:
  - `profile.dev`: 调试构建（opt-level=0, debug=true）
  - `profile.release`: 发布构建（opt-level="s", lto=true）
  - `profile.kernel`: 内核专用（opt-level="z", 最小体积）

## 架构特定注意事项

### AArch64 (ARM 64-bit)
- **异常级别**: EL1（内核）、EL0（用户态）
- **系统寄存器**: TTBR0_EL1（用户页表）、TTBR1_EL1（内核页表）
- **ASID支持**: 16-bit ASID用于进程隔离
- **异常向量表**: `src/kernel/src/arch/aarch64/exceptions/vectors.S`
- **上下文切换**: `src/kernel/src/arch/aarch64/context.rs`
- **ERET指令**: 从EL1返回到EL0（`arch_do_exec`函数）

### x86_64 和 riscv64
- 当前主要支持aarch64，其他架构为实验性支持
- 相关代码在 `src/kernel/src/arch/x86_64/` 和 `src/kernel/src/arch/riscv64/`

## 关键文件参考

**内核入口和启动**:
- `src/kernel/src/main.rs` - Rust入口点
- `src/kernel/src/arch/aarch64/boot.S` - 汇编启动代码
- `src/kernel/src/arch/aarch64/mmu/mmu_boot.rs` - MMU初始化

**内存管理**:
- `src/kernel/src/memory/physical/mod.rs` - 物理内存分配器
- `src/kernel/src/memory/virtual_/mod.rs` - 虚拟内存管理
- `src/kernel/src/scripts/link.ld.S` - 链接脚本

**进程和调度**:
- `src/kernel/src/core/scheduler/mod.rs` - 调度器
- `src/kernel/src/process/task/mod.rs` - 任务管理
- `src/kernel/src/arch/aarch64/context.rs` - 上下文切换

**IPC和系统调用**:
- `src/kernel/src/core/ipc/mod.rs` - IPC核心
- `src/kernel/src/syscall/mod.rs` - 系统调用分发
- `src/kernel/src/ipc_services/delegate.rs` - 服务委托框架

**用户空间**:
- `src/space/services/init/src/main.rs` - Init进程
- `src/space/hnxlib/src/lib.rs` - HNX系统库
- `src/space/libc/src/lib.rs` - C标准库实现

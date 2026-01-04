# HNX 微内核操作系统

## 项目概述

HNX 是一个用 Rust 编写的现代微内核操作系统，采用内核空间与用户空间服务的清晰分离设计。项目实现了三层系统调用架构（用户空间 → 内核 → 服务），并强调安全性、模块化和可靠性。

**核心架构:**
- **微内核设计:** 最小化内核，仅提供基本服务（IPC、内存管理、调度）
- **用户空间服务:** 核心功能作为独立服务实现（Loader、VFS、IPC Router 等）
- **基于能力的安全性:** 使用能力进行细粒度访问控制
- **跨平台支持:** 当前主要支持 AArch64 架构，使用 QEMU 仿真

**关键技术:**
- **编程语言:** Rust（内核和用户空间组件）
- **构建系统:** Meson
- **仿真环境:** QEMU 用于测试和开发
- **目标架构:** AArch64 (ARMv8-A)
- **版本管理:** 语义化版本控制，支持自动同步

## 目录结构

```
hnx-core/
├── .cargo/                    # Cargo 配置目录
├── configs/                   # 系统配置文件目录
│   ├── arch/                  # 架构特定配置 (aarch64.toml)
│   ├── board/                 # 开发板特定配置 (qemu-virt.toml)
│   ├── profile/               # 构建配置文件
│   └── scripts/               # 配置脚本
├── shared/                    # 共享库（核心）
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── abi/              # ABI 定义（核心）
│   │   │   ├── mod.rs        # 主模块
│   │   │   ├── syscalls.rs   # 系统调用定义
│   │   │   ├── types.rs      # 类型定义
│   │   │   ├── errors.rs     # 错误定义
│   │   │   └── constants.rs  # 常量定义
│   │   ├── collections/      # 数据结构
│   │   └── sync/            # 同步原语
│   └── README.md
├── kernel/               # 内核核心（库 crate）
│   ├── src/              # 内核源文件
│   ├── include/          # C 头文件
│   ├── linker.ld         # 内核链接脚本
│   └── Cargo.toml        # 内核 crate 配置
├── space/                # 用户空间组件（工作空间）
│   ├── Cargo.toml        # 工作空间配置
│   ├── hnxlib/           # HNX 系统库 crate
│   ├── libc/             # C 标准库实现
│   └── services/         # 系统服务
│       ├── init/         # 初始化进程 (PID 1)
│       ├── loader-service/ # 服务加载器
│       ├── vfs-service/  # 虚拟文件系统服务
│       ├── ipcrouter-service/ # IPC路由服务（服务注册与发现）
│       ├── procmgr-service/ # 进程管理服务
│       └── echo-service/ # 回显测试服务
├── utils/                # 开发工具目录
├── targets/                   # Rust 目标定义目录
├── toolchain/                 # 工具链构建配置目录
├── tools/                     # 开发工具目录
├── tests/                     # 测试框架目录
├── meson.build                # Meson 构建配置文件
├── meson_options.txt          # Meson 构建选项
├── Cargo.toml                 # 根工作空间配置文件
├── pyproject.toml            # Python 工具配置 (Poetry)
├── rust-toolchain.toml       # Rust 工具链规范文件
└── VERSION                   # 版本元数据文件
```

## 构建与运行

### 前置要求
- **Rust:** Nightly 工具链（在 `rust-toolchain.toml` 中指定）
- **Python:** 3.11+ 并安装 Poetry（用于配置脚本）
- **QEMU:** 用于系统仿真 (`qemu-system-aarch64`)
- **交叉编译工具:** AArch64 目标 (`aarch64-unknown-none`)

### 构建命令

```bash
# 配置构建（必须先执行）
meson setup build --option ARCH=aarch64 --option BOARD=qemu-virt --option PROFILE=debug

# 构建所有默认目标
meson compile -C build

# 仅构建内核
meson compile -C build kernel

# 构建用户空间组件
meson compile -C build space

# 构建完整的系统镜像
meson compile -C build image

# 构建简单镜像（仅包含 init 和核心服务）
meson compile -C build simple-image
```

### 运行与测试

```bash
# 在 QEMU 中运行完整 initrd
meson run -C build run

# 使用简单 initrd 运行
meson run -C build run-simple

# 使用 GDB 调试器运行
meson run -C build debug

# 运行测试
meson test -C build
```

### 清理操作

```bash
# 清理构建产物
rm -rf build
```

## 开发约定

### 代码风格
- **Rust:** 遵循 Rust 标准格式化 (`cargo fmt`)
- **C:** 内核头文件遵循 Unix 内核约定
- **Python:** 遵循 PEP 8 配置脚本规范

### 跨架构开发约定
HNX 设计为跨架构微内核操作系统，为支持多种处理器架构（AArch64、x86_64、RISC-V等），请遵守以下约定：

1. **条件编译架构代码：**
   - 使用 `#[cfg(target_arch = "aarch64")]` 等属性标记架构特定代码
   - 通用代码中严禁出现特定架构的硬编码或假设
   - 架构相关代码应放在 `kernel/src/arch/` 对应子目录中

2. **架构抽象层：**
   - 通用代码通过 trait 和抽象接口调用架构特定功能
   - 内存管理、中断处理、上下文切换等必须有通用接口
   - 架构特定实现必须实现通用接口的所有方法

3. **文件组织：**
   ```
   kernel/src/arch/
   ├── mod.rs              # 架构抽象模块导出
   ├── common/             # 跨架构通用代码
   ├── aarch64/           # AArch64 特定实现
   │   ├── mod.rs
   │   ├── interrupt.rs
   │   ├── mmu.rs
   │   └── boot.rs
   ├── x86_64/            # x86_64 特定实现（未来）
   └── riscv64/           # RISC-V 特定实现（未来）
   ```

4. **构建系统支持：**
   - `configs/arch/` 目录包含架构特定配置
   - `meson.build` 支持 `ARCH` 变量切换目标架构
   - 工具链自动选择对应架构的编译器和链接器

5. **测试与调试：**
   - 所有测试日志都包含测试标志 `[TEST]`并使用 `test!` 打印
   - 包含测试标志 `[TEST]`的日志在测试完成后需要移除或者注释
   - 测试代码中严禁使用 `println!` 等打印语句，必须使用 `test!` 宏
   - `test!` 宏用于打印测试信息，格式为 `[TEST] <消息>`
   - `test!`等级高于 `warn!`
   - 非 kernel 代码中严禁使用 `test!` 宏，允许使用 `println!` 等打印宏

### 文档注释规范
为支持后续自动生成文档系统，请严格遵守以下注释规范：

1. **中文文档注释：**
   - 所有公共 API、模块、结构体、函数必须包含中文文档注释
   - 使用标准的 Rust 文档注释格式 `///` 和 `//!`
   - 代码内部解释性注释也使用中文

2. **文档注释内容：**
   ```rust
   /// 从 initrd 加载并启动服务
   ///
   /// # 参数
   /// - `path`: 服务在 initrd 中的路径（如 `/bin/loader-service`）
   ///
   /// # 返回值
   /// - `Ok(pid)`: 成功启动的服务进程 ID
   /// - `Err(error)`: 错误代码和描述
   ///
   /// # 示例
   /// ```no_run
   /// let pid = spawn_service_from_initrd("/bin/loader-service")?;
   /// ```
   ///
   /// # 注意
   /// - 此函数仅在内核空间调用
   /// - 服务必须为有效的 ELF 可执行文件
   pub fn spawn_service_from_initrd(path: &str) -> Result<usize, SysError> {
       // 实现代码...
   }
   ```

3. **注释层级：**
   - **模块级:** `//!` 注释描述模块整体功能和架构位置
   - **结构体/枚举级:** `///` 描述数据类型用途和字段含义
   - **函数/方法级:** `///` 包含参数、返回值、示例、注意事项
   - **代码块级:** `//` 解释复杂算法或关键实现细节

4. **自动化文档生成：**
   - 运行 `cargo doc` 可生成完整的中文 API 文档
   - 文档包含跳转链接和代码示例
   - 确保所有公共项都有完整文档，避免 `missing_docs` 警告

5. **命名约定：**
   - 函数、变量、类型使用英文命名（遵循 Rust 命名规范）
   - 配置文件、脚本使用有意义的英文名称
   - 仅在注释和文档中使用中文进行解释说明

### 服务开发规范
服务开发遵循统一架构，详细指南见 `space/services/SERVICE_DEVELOPMENT.md`：

1. **服务命名:** 使用 `-service` 后缀（如 `ipcrouter-service`）
2. **服务框架:** 使用 `hnxlib::ipc::ServiceFramework` 进行自动注册和消息处理
3. **IPC 通信:** 使用类型安全的 `Endpoint` 进行进程间通信
4. **错误处理:** 返回统一的 `IpcError` 错误类型

### 系统调用架构
HNX 实现了三层系统调用架构：

1. **用户空间 → 内核:** 基本系统调用（写、读、生成服务）
2. **内核 → 服务委托:** 复杂操作委托给服务处理
3. **服务 → 服务:** 通过 IPC 进行服务间通信

**关键系统调用:**
- `HNX_SYS_WRITE` (0x1001): 控制台输出
- `HNX_SYS_SPAWN_SERVICE` (0x0103): 从 initrd 加载服务
- `HNX_SYS_PROCESS_CREATE` (0x0101): 创建空进程
- `HNX_SYS_IPC_WAIT` / `HNX_SYS_IPC_WAKE`: IPC 同步

### 服务设计
- **初始化进程 (PID 1):** 第一个用户进程，启动和监控核心服务
- **加载器服务:** 加载和管理用户空间程序
- **虚拟文件系统服务:** 虚拟文件系统管理
- **IPC路由服务:** 服务注册、发现和消息路由
- **进程管理服务:** 进程生命周期管理和服务健康监控
- **回显测试服务:** 演示 IPC 通信的测试服务
- **服务通信:** 通过基于能力的 IPC 端点进行通信，使用统一的服务开发框架

### 构建系统
- **Meson:** 顶层构建编排
- **Python 脚本:** 配置和镜像创建
- **Rust 工作空间:** 内核和用户空间的独立 crate
- **交叉编译:** 针对 `aarch64-unknown-none` 目标构建

### 内存管理
- **虚拟内存:** 每个进程的页表，用户/内核分离
- **物理内存:** 使用伙伴系统的页分配器
- **VMA 跟踪:** ELF 加载的虚拟内存区域管理

### Initrd 与引导过程
1. **内核引导:** 从内存加载压缩或未压缩的 initrd
2. **引导加载器:** 仅用于 init 进程的最小 ELF 加载器
3. **Init 启动:** 从 initrd 的 `/init` 加载，启动服务链
4. **服务加载:** `sys_spawn_service` 从 initrd 的 `/bin/` 目录加载服务

**Initrd 创建:**
- **简单 initrd:** 仅包含 init 和核心服务 (`meson run -C build run-simple`)
- **完整 initrd:** 包含设备节点的完整目录结构
- **压缩:** 默认使用 gzip 压缩（内核尚不支持解压）

## 开发工作流程

### 设置开发环境
```bash
# 安装 Rust nightly 工具链
rustup install nightly
rustup default nightly

# 安装 Python 依赖
poetry install

# 安装交叉编译目标
rustup target add aarch64-unknown-none

# 安装 QEMU（macOS 示例）
brew install qemu
```

### 典型开发周期
```bash
# 1. 配置构建
meson setup build --option ARCH=aarch64 --option BOARD=qemu-virt --option PROFILE=debug

# 2. 构建组件
meson compile -C build

# 3. 创建测试镜像
meson compile -C build simple-image

# 4. 使用超时快速测试运行
meson run -C build run-simple

# 5. 使用内核日志调试问题
# （内核日志出现在 QEMU 输出中）
```

### 调试
- **内核日志:** 内核源代码中的详细 INFO/ERROR 日志记录
- **QEMU GDB:** 使用 `meson run -C build debug` 进行 GDB 调试
- **系统调用跟踪:** 内核记录系统调用入口/出口及参数
- **页面错误调试:** 详细的 VMA 和页表日志记录

### 测试
- **单元测试:** Rust crate 内的单元测试 (`meson test -C build`)
- **集成测试:** QEMU 环境中的系统级测试
- **手动测试:** 使用 `meson run -C build run-simple` 进行 30 秒超时的快速验证

## 关键实现细节

### 当前状态
- **版本:** 0.2.0-alpha.1
- **架构:** 支持 AArch64 和 QEMU virt 机器
- **核心功能:**
  - 具有基本调度的微内核
  - 虚拟内存管理
  - 带超时机制的 IPC 基础设施
  - 从 initrd 生成服务
  - 通过 sys_write 的控制台输出
  - 统一的服务开发框架（ServiceFramework）
  - 类型安全的 IPC 通信抽象
  - 服务注册与发现机制
- **进行中:**
  - 系统调用返回值传递问题修复
  - 现有服务适配新的 IPC 框架
  - 进程管理服务功能完善
  - Initrd gzip 解压支持
---

*此开发状态部分跟踪当前进度、阻塞问题和未来计划。随着项目发展定期更新。最后更新：2026年1月2日*

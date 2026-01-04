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
- **构建系统:** Make + Python 配置脚本
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
├── include/                   # C 头文件目录（ABI 定义）
│   └── hnx/abi/              # 系统 ABI 头文件 (syscalls.h, types.h)
├── scripts/                   # 构建和工具脚本目录
│   ├── create-image.py       # 系统镜像创建工具
│   ├── version.py            # 版本管理脚本
│   └── verify_version.py     # 版本一致性检查脚本
├── abi-bindings/         # Rust ABI 绑定
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
├── Makefile                   # 顶层构建入口文件
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
# 显示可用目标
make help

# 配置构建（必须先执行）
make configure

# 构建工具链
make toolchain

# 仅构建内核
make kernel

# 构建用户空间组件
make space

# 构建完整的系统镜像
make image

# 构建简单镜像（仅包含 init 和核心服务）
make simple-image

# 快速构建（仅内核，无依赖）
make quick
```

### 运行与测试

```bash
# 在 QEMU 中运行完整 initrd
make run

# 使用简单 initrd 运行，自带30秒超时请不要使用 timeout 命令
make run-simple

# 使用 GDB 调试器运行
make debug

# 运行测试
make test
```

### 版本管理

```bash
# 显示版本信息
make version

# 初始化版本文件
make version-init

# 同步版本到所有子项目
make version-sync

# 检查版本一致性
make version-check

# 递增版本号
make version-bump-major
make version-bump-minor
make version-bump-patch

# 设置预发布标签
make version-set-prerelease TAG=<tag>
```

### 清理操作

```bash
# 清理构建产物
make clean

# 清理所有文件（包括版本文件）
make distclean
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
   - Makefile 支持 `ARCH` 变量切换目标架构
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
- **Makefile:** 顶层构建编排
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
- **简单 initrd:** 仅包含 init 和核心服务 (`make run-simple`)
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
make configure ARCH=aarch64 BOARD=qemu-virt PROFILE=debug

# 2. 构建组件
make kernel space

# 3. 创建测试镜像
make simple-image

# 4. 使用超时快速测试运行
make run-simple

# 5. 使用内核日志调试问题
# （内核日志出现在 QEMU 输出中）
```

### 调试
- **内核日志:** 内核源代码中的详细 INFO/ERROR 日志记录
- **QEMU GDB:** 使用 `make debug` 进行 GDB 调试
- **系统调用跟踪:** 内核记录系统调用入口/出口及参数
- **页面错误调试:** 详细的 VMA 和页表日志记录

### 测试
- **单元测试:** Rust crate 内的单元测试 (`cargo test`)
- **集成测试:** QEMU 环境中的系统级测试
- **手动测试:** 使用 `make run-simple` 进行 30 秒超时的快速验证

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

### 文件位置
- **系统调用定义:** `include/hnx/abi/syscalls.h`
- **内核系统调用实现:** `kernel/src/process/syscall/mod.rs`
- **IPC 抽象库:** `space/hnxlib/src/ipc.rs`
- **初始化进程:** `space/services/init/src/main.rs`
- **加载器服务:** `space/services/loader-service/src/main.rs`
- **IPC 路由服务:** `space/services/ipcrouter-service/src/main.rs`
- **进程管理服务:** `space/services/procmgr-service/src/main.rs`
- **回显测试服务:** `space/services/echo-service/src/main.rs`
- **服务开发指南:** `space/services/SERVICE_DEVELOPMENT.md`
- **镜像创建脚本:** `scripts/create-image.py`
- **构建配置:** `Makefile` + `configs/` 目录

### 配置选项
```bash
# 为不同架构构建
make configure ARCH=aarch64
make configure ARCH=x86_64  # 未来支持
make configure ARCH=riscv64 # 未来支持

# 选择不同开发板
make configure BOARD=qemu-virt
make configure BOARD=raspberry-pi4  # 未来支持

# 构建配置文件
make configure PROFILE=debug    # 开发版本，包含符号信息
make configure PROFILE=release  # 优化大小版本
```

## 故障排除

### 常见问题

1. **Initrd 压缩问题:**
   ```
   Error: Initrd is gzip compressed but kernel cannot decompress
   ```
   **解决方案:** 使用 `--no-compress` 标志或 `make run-simple` 创建未压缩的 initrd。

2. **Initrd 中缺少服务:**
   ```
   Failed to start Loader service (error: -1)
   ```
   **解决方案:** 确保服务已构建 (`make space`) 并包含在 initrd 中（检查 `scripts/create-image.py`）。

3. **Init 进程中的页面错误:**
   ```
   page fault at 0x... in init process
   ```
   **解决方案:** 检查栈布局和系统调用参数传递约定。

4. **构建配置问题:**
   ```
   Error: No rule to make target 'services'
   ```
   **解决方案:** 先运行 `make configure` 生成构建配置。

### 性能提示
- 开发过程中使用 `make quick` 仅重建内核
- `make run-simple` 有 30 秒超时，防止测试挂起
- 调试版本包含详细日志记录（可在内核配置中调整日志级别）
- 发布版本使用 LTO 和 panic="abort" 优化大小

## 贡献指南

在代码库中工作时：

1. **遵循现有模式:** 匹配周围代码的风格和架构
2. **服务开发规范:** 新服务必须遵循统一的服务开发架构（参考 `SERVICE_DEVELOPMENT.md`）
3. **命名约定:** 服务使用 `-service` 后缀，遵循统一命名规范
4. **IPC 通信:** 使用 `hnxlib::ipc` 模块提供的抽象进行服务间通信
5. **更新测试:** 为新功能添加测试
6. **版本管理:** 使用 `make version-*` 命令进行版本更新
7. **文档更新:** 更新相关文档和注释
8. **验证构建:** 提交前运行 `make version-check` 和 `make test`

## 许可与致谢

- **许可证:** MIT（详见各个文件）
- **开发团队:** HNX 开发者团队
- **架构灵感:** 受现代微内核设计（seL4、Fuchsia）启发

---

## 当前开发状态（2026年1月）

### 总体状态

**项目阶段:** 用户空间应用程序开发阶段（优先开发shell应用）
**当前版本:** 0.2.0-alpha.1
**核心成就:**
- 建立了统一的服务开发架构和完整的 IPC 通信基础设施
- 实现了调度器上下文切换，支持进程间协作式多任务
- 验证了基础系统调用（write）和进程创建（spawn_service）功能
- **调试输出大幅优化:** 调度器、IPC、异常处理等模块的冗余调试输出已大幅减少

### 近期关键进展

#### ✅ 已完成的核心工作

1. **统一服务架构确立:**
   - **命名规范统一:** 所有服务遵循 `-service` 后缀命名（`procmgr-service`、`ipcrouter-service`、`echo-service`等）
   - **目录结构更新:** 服务目录包含完整的核心服务集合
   - **构建系统同步:** 工作空间配置和构建流程支持新服务架构

2. **IPC 通信基础设施完成:**
   - **高级 IPC 抽象:** 在 `hnxlib` 中添加 `ipc` 模块，提供类型安全的通信接口
   - **核心组件:**
     - `Endpoint`: 类型安全的端点句柄管理
     - `IpcRouterClient`: 服务发现和注册客户端
     - `ServiceFramework`: 自动化服务注册和消息处理框架
   - **系统调用包装:** 实现完整的 IPC 相关系统调用安全包装函数

3. **核心服务重构完成:**
   - **ipcrouter-service:** 完整的服务注册、发现和消息路由实现，维护服务注册表
   - **procmgr-service:** 进程管理服务框架，提供进程生命周期管理基础
   - **echo-service:** 示例服务，演示请求-响应 IPC 通信模式
   - **init 进程更新:** 按依赖顺序启动所有核心服务，增强错误处理和监控

4. **调度器上下文切换实现:**
   - **问题解决:** 调度器无法正确处理进程间切换，缺少当前进程上下文保存机制
   - **修复措施:**
     - 重构 `run_task()` 函数，在调用 `exec_user()` 前释放 `CURRENT` 锁避免死锁
     - 实现 `update_process_context()` API 用于保存进程PC和SP
     - 在 `switch_to_next_process()` 中添加当前进程上下文保存逻辑
     - 修复 `current_pid()` 使用 `try_lock()` 避免异常上下文中的死锁
   - **结果:** 调度器现在能够正确从init进程（PID 1）切换到loader-service（PID 2），实现基本协作式多任务

5. **调试输出优化完成:**
   - **调度器模块:** 将详细调度日志从 `info!` 降级为 `debug!`，移除直接控制台调试输出
   - **IPC模块:** IPC警告输出包装在 `#[cfg(debug_assertions)]` 中，减少生产日志
   - **异常处理:** 大幅简化 `rust_sync_try_handle` 的调试输出
   - **服务框架:** 减少服务框架错误输出频率（每10次输出一次）
   - **结果:** 运行时日志噪声显著减少，关键日志更清晰

6. **测试系统调用机制建立:**
   - **测试系统调用:** 添加系统调用 `0x9999` 返回固定值 `42`，用于验证返回值传递机制
   - **调试验证:** 为系统调用调试提供标准化测试工具
   - **结果:** 建立了系统调用测试基础设施，便于验证返回值传递问题

7. **图形基础设施验证:**
   - **显示服务端点确认:** `ServiceEndpoint::Display`（端点6）已定义并可用
   - **VirtIO设备支持:** QEMU配置确认支持 `virtio-gpu`、`virtio-keyboard`、`virtio-mouse` 设备
   - **设备驱动框架:** 设备驱动服务器框架支持图形设备类型（`DeviceType::Graphics`、`DeviceType::Gpu`）
   - **内存映射:** VirtIO MMIO基地址（0x1000_0000）已在内核页表中映射
   - **结果:** 系统已具备图形系统开发的基础设施基础

8. **构建系统稳定性:**
   - 所有服务编译通过：`make space` ✅
   - 完整系统镜像包含所有服务：`make image` ✅
   - 调试输出优化后构建正常，无语法错误

#### 🔧 技术架构亮点

**统一服务模板:**
```rust
let framework = ServiceFramework::new("service-name")?;
framework.run(handle_message);

fn handle_message(op: u16, request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 根据操作码处理请求
}
```

**系统调用集成:**
- `HNX_SYS_YIELD` (0x18): 协作式调度器主动让出CPU
- `HNX_SYS_WRITE` (0x1001): 控制台输出（已验证工作正常）
- `HNX_SYS_SPAWN_SERVICE` (0x0103): 从initrd加载服务（返回值传递问题）
- `HNX_SYS_TEST` (0x9999): 测试系统调用返回42

### 当前阻塞问题

#### 🚨 主要阻塞：系统调用返回值传递问题

**问题描述:**
- 内核成功执行 `sys_spawn_service` 并返回正确的 PID（2-6），但用户空间（init 进程）收到返回值 -1
- 导致服务无法正确启动和进行 IPC 通信，所有核心服务启动失败
- 测试系统调用 `0x9999` 返回42的功能尚未验证，但基础系统调用 `sys_write` 工作正常

**详细调试状态:**
- ✅ **服务加载成功:** 内核 `sys_spawn_service` 成功加载所有核心服务并分配正确的 PID（2-6）
- ✅ **栈布局匹配:** 汇编代码与Rust代码的栈偏移量一致（x0在sp+144）
- ✅ **内存写入验证:** `rust_svc_handler` 正确将返回值写入栈中x0位置，调试日志确认写入成功
- ✅ **其他系统调用正常:** `sys_write` 等系统调用返回值传递正常
- ❌ **返回值丢失:** 异常返回后，用户空间恢复的x0寄存器值为-1而不是预期的PID
- ❌ **spawn_service失败:** 虽然内核加载服务成功，但用户空间无法获取正确PID，导致服务启动失败

**技术分析:**
- **核心问题:** AArch64 异常处理返回路径中寄存器恢复逻辑问题
- **可能原因:**
  1. 异常返回前，从错误位置加载x0寄存器
  2. 寄存器破坏：系统调用返回后某个代码修改了x0
  3. 调度器干扰：异常返回后被调度器抢占并修改上下文
  4. 用户空间包装器错误解析返回值
- **影响范围:** 所有依赖系统调用返回值的功能，特别是进程创建和服务启动

#### ⚠️ 次要问题
- **协作式多任务闭环:** 调度器能成功从init切换到loader-service，但yield系统调用后切换回init的闭环尚未完成
- **Initrd 压缩支持:** 内核 CPIO 解析器尚不支持 gzip 解压，需要手动使用未压缩initrd
- **现有服务迁移:** `vfs-service` 和 `loader-service` 需适配新的 IPC 框架
- **调试基础设施:** 需要更详细的异常处理和调度器调试信息

### 下一步计划（Shell应用优先）

**整体策略:** 先开发一个完整的shell应用（如移植zsh或开发轻量级shell），以此为驱动解决底层系统问题，同时为用户提供可交互的界面。

#### 🔥 第一阶段：Shell应用开发准备（3-4天）

1. **解决系统调用返回值问题**
   - 调试 `sys_spawn_service` 返回值传递问题，确保服务能正确启动
   - 验证测试系统调用 `0x9999` 返回值传递机制
   - 完善异常处理返回路径，确保寄存器正确恢复

2. **建立基础服务链**
   - 确保init能成功启动loader-service、ipcrouter-service等核心服务
   - 验证服务间IPC通信机制正常工作
   - 建立稳定的用户空间运行环境

3. **实现基础终端支持**
   - 添加终端设备驱动框架
   - 实现基本的输入/输出系统调用（read/write）
   - 建立控制台交互基础设施

#### 📋 第二阶段：Shell核心功能（4-5天）

1. **移植或开发轻量级shell**
   - **选项A:** 移植zsh核心功能（复杂但功能丰富）
   - **选项B:** 开发新的轻量级shell（简单但可控）
   - 实现基本的命令行解析、命令执行、管道支持

2. **文件系统集成**
   - 集成vfs-service，提供基本的文件访问功能
   - 实现路径解析和当前目录管理
   - 添加基本的文件操作命令（ls、cd、cat等）

3. **进程管理增强**
   - 实现前台/后台进程管理
   - 添加作业控制基础功能
   - 完善进程间通信机制

#### 🔄 第三阶段：Shell功能完善（3-4天）

1. **用户交互增强**
   - 实现命令历史记录
   - 添加命令行编辑功能
   - 实现tab自动补全

2. **脚本支持**
   - 添加基本的shell脚本解释器
   - 实现变量替换和控制结构
   - 支持简单的条件判断和循环

3. **系统集成**
   - 集成环境变量管理
   - 实现命令别名功能
   - 添加基本的系统信息命令

#### 🗺️ 第四阶段：基础图形系统开发（后续计划）

1. **图形基础设施**
   - 实现virtio-gpu设备驱动
   - 开发显示服务（framebuffer管理）
   - 添加基本的2D图形库

2. **用户界面框架**
   - 实现简单的窗口管理器
   - 开发基础GUI组件库
   - 集成输入设备支持

3. **图形shell界面**
   - 开发图形化终端模拟器
   - 实现基本的GUI shell
   - 集成文本和图形混合界面

### 成功指标（当前阶段）

- ✅ 统一的服务开发架构建立
- ✅ IPC 通信基础设施完成
- ✅ 核心服务框架实现
- ✅ 调试输出优化完成
- ✅ 测试系统调用机制建立
- ✅ 图形基础设施验证完成
- 🔄 系统调用返回值问题解决（主要阻塞）
- 🔄 基础服务链建立（进行中）
- 🔄 终端支持实现（TODO）
- 🔄 Shell应用开发（TODO）

### 技术债务与改进点

1. **代码质量:**
   - 需要增加服务单元测试覆盖率
   - 完善错误处理和边界情况处理
   - 优化内存使用和性能

2. **文档完善:**
   - 补充 API 文档和示例代码
   - 创建故障排除指南
   - 完善开发者入门文档

3. **工具链:**
   - 增强调试和性能分析工具
   - 完善持续集成流程
   - 优化开发体验

4. **调试基础设施:**
   - 添加更详细的异常处理调试信息
   - 实现系统调用跟踪和性能分析
   - 完善QEMU GDB调试脚本

---

## Shell应用开发详细计划

### 项目目标
在HNX微内核操作系统上开发一个功能完整的shell应用，提供交互式命令行界面、基本的文件操作、进程管理和脚本支持。

### 技术选择
**方案选择：开发新的轻量级shell（HNX-Shell）**
- **理由**：
  1. 移植zsh需要完整的POSIX环境，当前系统不支持
  2. 新开发可以完全匹配HNX的微内核架构和IPC通信模型
  3. 可以逐步添加功能，从最小可行产品开始
  4. 更好的系统集成和性能优化

**技术栈**：
- **语言**：Rust（与系统其他部分一致）
- **框架**：使用HNX服务开发框架（ServiceFramework）
- **IPC**：通过IPC路由器进行服务间通信
- **依赖**：终端设备服务、VFS服务、进程管理服务

### 架构设计

#### 系统架构
```
┌─────────────────┐
│   HNX-Shell     │ 用户交互层（命令行界面）
├─────────────────┤
│  Shell Core     │ 命令解析、执行、作业控制
├─────────────────┤
│  Service Layer  │ 通过IPC调用底层服务
├─────────────────┤
│  Terminal       │ 终端设备驱动服务
│  VFS Service    │ 文件系统服务
│  ProcMgr        │ 进程管理服务
└─────────────────┘
```

#### 组件设计
1. **终端接口**：与终端设备服务通信，处理输入/输出
2. **命令解析器**：解析命令行，支持参数、管道、重定向
3. **执行引擎**：调用系统调用或服务来执行命令
4. **作业控制**：管理前台/后台进程
5. **内置命令**：实现shell内置功能

### 开发阶段

#### 阶段0：先决条件（2-3天）
**目标**：解决当前阻塞问题，建立稳定的基础环境
1. 修复系统调用返回值传递问题
2. 确保init能成功启动loader-service、ipcrouter-service
3. 验证基础IPC通信机制
4. 建立终端设备驱动框架

#### 阶段1：基础Shell（4-5天）
**目标**：实现最小可行产品（MVP）
1. **项目创建**：
   - 创建`shell`或`hsh`（HNX Shell）项目
   - 集成服务开发框架
   - 设置构建配置

2. **终端交互**：
   - 实现基本的行编辑
   - 支持字符回显
   - 处理特殊键（Enter、Backspace）

3. **命令执行**：
   - 基本命令解析（分割参数）
   - 执行外部程序（通过loader-service）
   - 实现`echo`、`help`等简单内置命令

4. **系统集成**：
   - 与终端设备服务集成
   - 基本的进程创建和等待

#### 阶段2：功能增强（5-6天）
**目标**：添加核心shell功能
1. **文件系统支持**：
   - 集成VFS服务
   - 实现`ls`、`cd`、`pwd`、`cat`命令
   - 路径解析和当前目录管理

2. **管道和重定向**：
   - 支持`|`管道操作
   - 实现`>`、`>>`、`<`重定向
   - 文件描述符管理

3. **作业控制**：
   - 前台/后台进程管理
   - `&`后台执行支持
   - 基本的作业列表

4. **环境变量**：
   - 环境变量存储和扩展
   - `PATH`支持用于命令查找
   - 用户自定义变量

#### 阶段3：高级功能（4-5天）
**目标**：完善用户体验
1. **用户交互增强**：
   - 命令历史记录（上下箭头）
   - Tab自动补全
   - 行编辑增强（Ctrl+A/E等）

2. **脚本支持**：
   - 基本的脚本解释器
   - 变量替换（`$VAR`）
   - 简单控制结构（if、for）

3. **系统命令**：
   - `ps`进程列表
   - `kill`进程终止
   - 系统信息命令

4. **配置和自定义**：
   - Shell配置文件（`.hnxrc`）
   - 命令别名
   - 提示符自定义

#### 阶段4：优化和测试（3-4天）
**目标**：稳定性和性能
1. **错误处理**：
   - 健壮的错误处理和恢复
   - 用户友好的错误消息

2. **性能优化**：
   - 减少内存使用
   - 优化命令执行路径

3. **测试套件**：
   - 单元测试
   - 集成测试
   - 用户场景测试

4. **文档**：
   - 用户手册
   - 开发者指南
   - 内置帮助系统

### 时间估计
- **总时间**：18-23天（约3-4周）
- **关键路径**：阶段0（系统调用修复）是后续所有工作的前提

### 成功标准
1. **MVP标准**（阶段1完成）：
   - 能启动shell并显示提示符
   - 能执行简单内置命令
   - 能启动外部程序
   - 基本行编辑工作

2. **完整功能标准**（阶段3完成）：
   - 支持管道和重定向
   - 文件系统命令工作
   - 作业控制功能正常
   - 基本的脚本支持

3. **生产就绪标准**（阶段4完成）：
   - 稳定的错误处理
   - 良好的性能表现
   - 完整的测试覆盖
   - 用户文档齐全

### 风险与缓解
1. **系统调用问题持续**：
   - **缓解**：优先解决阶段0问题，必要时简化设计

2. **IPC通信性能**：
   - **缓解**：优化消息传递，使用批量操作

3. **内存限制**：
   - **缓解**：轻量级设计，延迟加载

4. **功能范围蔓延**：
   - **缓解**：严格遵循阶段计划，先完成核心功能

### 后续扩展
1. **图形界面**：在基础图形系统完成后，开发图形化终端
2. **网络支持**：集成网络服务后添加网络相关命令
3. **插件系统**：支持动态加载shell扩展
4. **多用户支持**：集成用户管理服务

---

*此开发状态部分跟踪当前进度、阻塞问题和未来计划。随着项目发展定期更新。最后更新：2026年1月2日*

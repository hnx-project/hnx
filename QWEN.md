# HNX 微内核操作系统

## 项目概述

HNX 是一个用 Rust 编写的现代微内核操作系统，采用内核空间与用户空间服务的清晰分离设计。项目实现了三层系统调用架构（用户空间 → 内核 → 服务），并强调安全性、模块化和可靠性。

**核心架构:**
- **微内核设计:** 最小化内核，仅提供基本服务（IPC、内存管理、调度）
- **用户空间服务:** 核心功能作为独立服务实现（Loader、VFS 等）
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
├── .trae/                     # 工具链配置目录
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
├── src/                       # 源代码目录
│   ├── abi-bindings/         # Rust ABI 绑定
│   ├── kernel/               # 内核核心（库 crate）
│   │   ├── src/              # 内核源文件
│   │   ├── include/          # C 头文件
│   │   ├── linker.ld         # 内核链接脚本
│   │   └── Cargo.toml        # 内核 crate 配置
│   ├── space/                # 用户空间组件（工作空间）
│   │   ├── Cargo.toml        # 工作空间配置
│   │   ├── hnxlib/           # HNX 系统库 crate
│   │   ├── libc/             # C 标准库实现
│   │   └── services/         # 系统服务
│   │       ├── init/         # 初始化进程 (PID 1)
│   │       ├── loader-service/ # 服务加载器
│   │       ├── vfs-service/  # 虚拟文件系统服务
│   │       ├── ipcrouter-service/ # IPC路由服务（服务注册与发现）
│   │       ├── procmgr-service/ # 进程管理服务
│   │       └── echo-service/ # 回显测试服务
│   └── utils/                # 开发工具目录
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
   - 架构相关代码应放在 `src/kernel/src/arch/` 对应子目录中

2. **架构抽象层：**
   - 通用代码通过 trait 和抽象接口调用架构特定功能
   - 内存管理、中断处理、上下文切换等必须有通用接口
   - 架构特定实现必须实现通用接口的所有方法

3. **文件组织：**
   ```
   src/kernel/src/arch/
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
- **内核系统调用实现:** `src/kernel/src/process/syscall/mod.rs`
- **IPC 抽象库:** `src/space/hnxlib/src/ipc.rs`
- **初始化进程:** `src/space/services/init/src/main.rs`
- **加载器服务:** `src/space/services/loader-service/src/main.rs`
- **IPC 路由服务:** `src/space/services/ipcrouter-service/src/main.rs`
- **进程管理服务:** `src/space/services/procmgr-service/src/main.rs`
- **回显测试服务:** `src/space/services/echo-service/src/main.rs`
- **服务开发指南:** `SERVICE_DEVELOPMENT.md`
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

## 当前开发状态（2025年12月）

### 总体状态

**项目阶段:** 服务架构完善阶段
**当前版本:** 0.2.0-alpha.1
**核心成就:** 建立了统一的服务开发架构和完整的 IPC 通信基础设施

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

4. **开发文档完善:**
   - 创建 `SERVICE_DEVELOPMENT.md` 完整服务开发指南
   - 包含服务架构、开发模板、IPC 机制、最佳实践等
   - 提供示例代码、调试指南和常见问题解决方案

5. **构建验证通过:**
   - 所有服务编译通过：`make space` ✅
   - 完整系统镜像包含所有服务：`make image` ✅
   - IPC 基础架构就绪，等待系统调用返回值问题解决

#### 🔧 技术架构亮点

**统一服务模板:**
```rust
let framework = ServiceFramework::new("service-name")?;
framework.run(handle_message);

fn handle_message(op: u16, request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 根据操作码处理请求
}
```

**通信模式支持:**
- **请求-响应:** 同步消息交换
- **服务发现:** 通过 IPC 路由器查找服务端点
- **错误处理:** 统一错误响应机制

**系统调用集成:**
- `HNX_SYS_EP_CREATE` (0x012F): 创建 IPC 端点
- `HNX_SYS_EP_SEND` (0x0130): 发送 IPC 消息
- `HNX_SYS_EP_RECV` (0x0131): 接收 IPC 消息
- `HNX_SYS_IPC_WAIT`/`HNX_SYS_IPC_WAKE`: IPC 同步

### 当前阻塞问题

#### 🚨 主要阻塞：系统调用返回值传递问题

**问题描述:**
- 内核成功执行 `sys_spawn_service` 并返回 PID 2
- 但用户空间（init 进程）收到返回值 0 而非 2
- 导致服务无法正确启动和进行 IPC 通信

**技术分析:**
- **根本原因:** AArch64 异常处理中的寄存器保存/恢复逻辑问题
- **影响范围:** 所有依赖系统调用返回值的功能
- **临时解决方案:** 跳过服务启动可使 init 运行到监控循环

**调试状态:**
- 已确认内核端系统调用执行成功
- 已验证 `hnxlib` 安全系统调用包装器的寄存器保护机制
- 需要深入调试异常处理栈布局和寄存器保存逻辑

#### ⚠️ 次要问题
- **Initrd 压缩支持:** 内核 CPIO 解析器尚不支持 gzip 解压
- **现有服务迁移:** `vfs-service` 和 `loader-service` 需适配新的 IPC 框架
- **错误处理完善:** 需要更健壮的服务加载失败处理机制

### 下一步计划

#### 🔥 高优先级（立即行动）

1. **调试系统调用返回值问题**
   - 分析 AArch64 异常处理栈布局
   - 验证寄存器保存/恢复逻辑
   - 修复返回值传递机制

2. **测试 IPC 通信机制**
   - 在系统调用问题解决后，验证服务间 IPC 通信
   - 测试 echo-service 的基本功能
   - 验证 ipcrouter-service 的服务注册和发现

3. **完善进程管理功能**
   - 从 init 进程提取服务监控功能到 procmgr-service
   - 实现服务健康检查和自动重启机制

#### 📋 中优先级（短期目标）

4. **现有服务迁移**
   - 适配 `vfs-service` 使用新的 IPC 框架
   - 更新 `loader-service` 支持服务发现机制
   - 统一所有服务的错误处理模式

5. **开发工具完善**
   - 增强服务调试和监控工具
   - 完善服务开发模板和示例
   - 优化构建和测试流程

#### 🗺️ 长期路线图

**2026年第一季度目标:**
- 稳定的服务生态系统运行
- 完整的 IPC 通信和权限管理
- 基本的文件系统支持

**2026年第二季度目标:**
- 用户空间驱动框架
- 动态应用程序加载支持
- 硬件抽象层完善

### 成功指标（当前阶段）

- ✅ 统一的服务开发架构建立
- ✅ IPC 通信基础设施完成
- ✅ 核心服务框架实现
- 🔄 系统调用返回值问题解决（进行中）
- 🔄 服务间通信验证（待测试）
- 🔄 完整服务生态系统运行（目标）

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

---

*此开发状态部分跟踪当前进度、阻塞问题和未来计划。随着项目发展定期更新。最后更新：2025年12月28日*
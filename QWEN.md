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
│   │       └── vfs-service/  # 虚拟文件系统服务
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
- **C:** 内核头文件遵循 Linux 内核约定
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
- **初始化进程 (PID 1):** 第一个用户进程，启动核心服务
- **加载器服务:** 加载和管理用户空间程序
- **虚拟文件系统服务:** 虚拟文件系统管理
- **服务通信:** 通过基于能力的 IPC 端点进行通信

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
  - 带超时机制的 IPC
  - 从 initrd 生成服务
  - 通过 sys_write 的控制台输出
- **进行中:**
  - 服务 IPC 通信
  - VFS 服务集成
  - Initrd gzip 解压支持

### 文件位置
- **系统调用定义:** `include/hnx/abi/syscalls.h`
- **内核系统调用实现:** `src/kernel/src/process/syscall/mod.rs`
- **初始化进程:** `src/space/services/init/src/main.rs`
- **加载器服务:** `src/space/services/loader-service/src/main.rs`
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
2. **更新测试:** 为新功能添加测试
3. **版本管理:** 使用 `make version-*` 命令进行版本更新
4. **文档更新:** 更新相关文档和注释
5. **验证构建:** 提交前运行 `make version-check` 和 `make test`

## 许可与致谢

- **许可证:** MIT（详见各个文件）
- **开发团队:** HNX 开发者团队
- **架构灵感:** 受现代微内核设计（seL4、Fuchsia）启发

---

## 当前开发状态（2025年12月）

### 进度概览

**近期成果:**

1. **sys_spawn_service 系统调用实现:**
   - 系统调用号 `0x0103` 已添加到 ABI (`include/hnx/abi/syscalls.h`)
   - 内核实现在 `src/kernel/src/process/syscall/mod.rs`
   - 用户空间包装器在 `src/space/hnxlib/src/lib.rs`
   - 成功从 initrd 加载服务并返回 PID

2. **服务加载基础设施:**
   - 增强的加载器模块 (`src/kernel/src/loader/mod.rs`) 支持任意服务加载（不仅是 init）
   - `spawn_service_from_initrd()` 函数从 initrd 的 `/bin/` 目录查找并加载 ELF 服务
   - 引导 ELF 加载器正确映射服务内存、栈和页表

3. **Initrd 创建改进:**
   - 修改 `scripts/create-image.py` 在简单 initrd 中包含核心服务 (`loader-service`, `vfs-service`)
   - `make run-simple` 现在创建包含所有核心二进制文件的未压缩 initrd
   - 服务放置在 `/bin/` 目录中，与内核搜索路径匹配

4. **服务二进制文件增强:**
   - `loader-service` 使用直接系统调用更新了调试输出
   - `init` 进程尝试按顺序启动 Loader 和 VFS 服务
   - IPC 超时机制完全实现并运行正常

5. **构建系统验证:**
   - 完整的构建链工作正常: `make configure → make kernel space → make simple-image → make run-simple`
   - 所有组件版本同步 (v0.2.0-alpha.1)

**系统调用成功演示:**
```
[INFO] => sys_spawn_service: loading service '/bin/loader-service'
[INFO] => loader: found service ELF '/bin/loader-service', size 6152 bytes
[INFO] => process: created empty process pid=2 pt=0x40408000
[INFO] => process: started pid=2 entry=0x210204 sp=0x221000
[INFO] => sys_spawn_service: service '/bin/loader-service' started as PID 2
[INFO] => arch/aarch64 svc#0 ret=0x2
```

### 当前阻塞问题

**主要阻塞：系统调用后的 Init 页面错误**

**症状:**
- Init 进程在 `sys_spawn_service` 返回后因地址 `0x1BB` 的页面错误而崩溃
- 错误发生在指令: `str x0, [x8, #0xe0]`（存储系统调用返回值）
- 寄存器 `x8` 包含值 `0x103`（系统调用号），不是有效的栈指针
- 完整错误地址: init 二进制文件中的 `0x0000000000217904`

**技术细节:**
```
反汇编上下文:
217820: 52802068      mov	w8, #0x103              // =259 (HNX_SYS_SPAWN_SERVICE)
217824: d4000001      svc	#0                      // 系统调用
217828: f94033e8      ldr	x8, [sp, #0x60]         // 从栈偏移 0x60 加载
21782c: f9007100      str	x0, [x8, #0xe0]         // 页面错误发生在此处
```

**根本原因分析:**
1. **栈布局不匹配:** `[sp, #0x60]` 处的值应为有效指针，但包含 `0x103`
2. **编译器优化:** Rust 编译器可能使用与预期不同的栈布局
3. **系统调用约定:** `hnxlib` 包装器与 init 中的内联汇编可能不匹配
4. **寄存器保存:** 系统调用可能未正确保存所有寄存器

**变通方案状态:**
- 跳过服务启动允许 init 成功运行到服务监控循环
- 当 `start_core_services()` 被注释掉时，系统可以启动并运行基本控制台输出

**次要问题:**
- **Initrd 压缩:** 内核 CPIO 解析器不支持 gzip 解压
- **服务 IPC:** Loader 和 VFS 服务需要正确的 IPC 端点设置
- **错误处理:** 需要更健壮的服务加载失败错误报告

### 下一步计划（短期）

**高优先级（解决阻塞问题）:**
1. **调试 Init 栈问题:**
   - 添加调试输出来检查系统调用前后的栈内容
   - 验证栈指针对齐和帧布局
   - 使用 `rust-objdump -d` 检查编译器生成的栈使用情况

2. **修复系统调用约定:**
   - 确保 SVC 处理程序中正确保存/恢复所有寄存器
   - 验证 `hnxlib` 包装器符合内核预期
   - 在 init 中使用简化的内联汇编进行测试

3. **验证服务加载:**
   - 一旦 init 修复，测试完整的服务链: init → Loader → VFS
   - 验证服务 PID 和进程状态
   - 检查 Loader 服务调试输出是否出现

**中优先级（功能完成）:**
4. **服务 IPC 设置:**
   - 为核心服务实现端点创建和连接
   - 测试 init 和 Loader 之间的基本 IPC 消息传递
   - 验证服务中的 IPC 超时机制

5. **Initrd 压缩支持:**
   - 在内核 CPIO 解析器中添加 gzip 解压功能
   - 或将未压缩 initrd 作为开发标准

6. **错误处理增强:**
   - 改进服务加载失败的错误代码和消息
   - 在 init 中添加服务健康监控
   - 实现失败时服务重启

### 路线图（中期）

**2026年第一季度目标:**
1. **稳定的服务生态系统:**
   - 可靠的服务启动和监控
   - 服务发现和注册机制
   - 基本的服务生命周期管理（启动/停止/重启）

2. **IPC 基础设施:**
   - 基于能力的完整端点管理
   - 带超时和错误处理的消息传递
   - 服务到服务的通信模式

3. **VFS 集成:**
   - VFS 服务处理文件操作
   - 通过 IPC 将 sys_write 委托给 VFS
   - 基本的文件系统支持（基于 initrd）

**2026年第二季度目标:**
4. **驱动框架:**
   - 用户空间驱动架构
   - 设备发现和管理
   - 基本的控制台和定时器驱动程序

5. **应用加载:**
   - 能够加载任意 ELF 应用程序的加载器服务
   - 动态链接和重定位支持
   - 基于能力的应用程序沙箱

### 长期愿景（最终目标）

**完整的三层架构:**
- **第一层（用户 → 内核）:** 最小化、安全的系统调用接口
- **第二层（内核 → 服务）:** 复杂操作委托给服务处理
- **第三层（服务 → 服务）:** 基于 IPC 的丰富服务生态系统

**核心目标:**
1. **安全优先的设计:**
   - 全系统基于能力的访问控制
   - 通过硬件内存保护的进程隔离
   - 具有双向认证的安全 IPC

2. **可靠性与容错性:**
   - 服务隔离（一个服务故障不会崩溃其他服务）
   - 自动服务重启和恢复
   - 通过微内核设计实现系统稳定性

3. **性能与效率:**
   - 针对微内核优化的低延迟 IPC
   - 高效的上下文切换和调度
   - 嵌入式使用的最小内存占用

4. **开发者体验:**
   - 内核和服务的清晰、文档完善的 API
   - 全面的测试框架
   - 轻松部署到 QEMU 和真实硬件

5. **硬件支持:**
   - AArch64 作为主要架构，支持 QEMU virt
   - Raspberry Pi 4/5 支持，用于真实硬件测试
   - 潜在的 x86_64 和 RISC-V 端口

**成功指标:**
- 在 QEMU 上 2 秒内启动到功能完整的 shell
- 支持 10+ 个使用 IPC 的并发服务
- 基本系统内存占用低于 16MB
- 99.9% 的服务可用性，支持自动恢复
- 全面的测试套件，代码覆盖率 >80%

### 立即行动项

1. **调试 init 页面错误** - 阻塞所有进展的最高优先级
2. **修复系统调用寄存器处理** - 确保正确的 ABI 合规性
3. **测试完整的服务链** - 验证端到端的微内核架构
4. **实现服务 IPC** - 启用服务间通信
5. **添加 gzip 解压支持** - 支持标准 initrd 格式

---

*此开发状态部分跟踪当前进度、阻塞问题和未来计划。随着项目发展定期更新。*
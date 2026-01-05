# 开发工具与实用程序 (Development Tools and Utilities)

HNX 项目除了其核心操作系统代码外，还包含了一系列辅助开发、构建、测试和维护的工具与实用程序。这些工具旨在简化开发者的工作流程，提高效率，并确保项目质量。本节将介绍这些主要工具和它们的用途。

## 目录结构概述

项目中的开发工具和实用程序主要分布在以下几个顶级目录中：

-   **`configs/`:** 系统配置文件目录，包含架构、开发板和构建配置等。
-   **`scripts/`:** 顶层脚本目录，包含用于配置、镜像创建、QEMU 运行和版本验证的 Python 脚本。
-   **`targets/`:** Rust 目标定义目录，包含自定义的 Rust target JSON 文件。
-   **`toolchain/`:** 工具链构建配置目录。
-   **`tools/`:** 额外的开发工具目录，可能包含一些独立的、非构建流程核心的实用脚本或程序。
-   **`utils/`:** 包含一些通用的 Rust 实用 crate。

## 主要工具与用途

### `configs/` (配置管理)

`configs/` 目录存放了 HNX 系统不同方面的配置信息，这些配置在构建过程中被读取和应用。

-   **`arch/`:** 定义了不同处理器架构（如 `aarch64.toml`）的构建参数和特性。
-   **`board/`:** 定义了不同开发板（如 `qemu-virt.toml`）的硬件配置，例如内存布局、UART 地址等。
-   **`profile/`:** 定义了不同构建剖面（如 `debug.toml`）的优化级别和调试选项。
-   **`scripts/` (配置脚本):** 包含用于生成链接脚本 (`link.ld.S`) 或其他构建相关文件的模板和脚本。

### `scripts/` (自动化脚本)

`scripts/` 目录下的 Python 脚本是 HNX 自动化构建和测试流程的核心。

-   **`configure.py`:** 用于初始化和配置 Meson 构建目录，根据 `configs/` 中的选项设置构建参数。
-   **`create-image.py`:** 负责将编译好的内核、用户空间服务和应用程序打包成可引导的系统镜像 (例如包含 Initrd 的 `hnx-simple.img`)。
-   **`run-qemu.py`:** 封装了 QEMU 模拟器的命令行调用，使得开发者可以方便地启动 HNX 系统进行测试和调试，支持多种 QEMU 参数和选项。
-   **`release.py`:** (如果存在) 用于自动化发布流程，可能包括版本号管理、打包和发布到软件仓库等。
-   **`verify_abi.py` / `verify_version.py`:** 用于在持续集成/持续部署 (CI/CD) 流程中验证 HNX 的 ABI 兼容性或版本信息，确保系统组件之间的一致性。
-   **`version.py`:** 用于生成 HNX 的版本字符串和构建元数据。

### `targets/` (Rust 目标定义)

该目录包含了 HNX 项目定制的 Rust 目标三元组 (target triples) 定义文件，例如 `aarch64-unknown-hnx-kernel.json`。这些 JSON 文件指导 Rust 编译器如何为 HNX 这样的裸机目标生成代码，包括 ABI、链接器设置和特性。

### `toolchain/` (工具链配置)

该目录可能包含用于自动化工具链安装或配置的脚本和文件，确保所有开发者使用一致的开发环境，特别是在交叉编译场景下。

### `tools/` (辅助开发工具)

此目录可能包含一些通用的、辅助开发的工具，例如：

-   **`cargo-binutils`:** Rust 项目常用的 binutils 封装，用于查看 ELF 文件信息、反汇编等。
-   **`objcopy` / `objdump`:** 交叉编译链中的工具，用于处理编译后的二进制文件。

### `utils/` (通用 Rust 实用 crate)

`utils/` 目录下的 crate 包含一些通用的 Rust 实用函数或数据结构，它们可以被 HNX 项目中的多个 Rust crate 共享，但又不属于 `shared` 库的核心 ABI 或同步原语。

## 开发工作流程

这些工具与构建系统紧密集成，支持一个典型的 HNX 开发工作流程：

1.  **环境设置:** 使用 `rustup` 安装 nightly 工具链，安装 Python 依赖（通过 Poetry）。
2.  **构建配置:** `meson setup` 命令根据 `configs/` 和 `scripts/configure.py` 配置构建环境。
3.  **编译运行:** `meson compile` 调用 `cargo build` 编译 Rust 代码，`meson run` 调用 `scripts/run-qemu.py` 启动 QEMU。
4.  **调试测试:** 结合 QEMU 的 GDB 支持和内核日志进行调试，`meson test` 运行单元和集成测试。

通过这一整套工具和流程，HNX 致力于提供一个高效、可靠且易于使用的开发环境。
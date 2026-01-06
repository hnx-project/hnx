# 构建系统 (Build System)

HNX 采用一套灵活且强大的构建系统，旨在简化开发流程、支持多种架构的交叉编译，并高效管理复杂的项目结构。其核心由 [Meson Build System](https://mesonbuild.com/) 驱动，并辅以 Python 脚本和 Rust 工作空间机制。

## 核心组件

### Meson Build System

Meson 是 HNX 的主要构建编排工具。它以其速度快、易于使用和跨平台特性而闻名，非常适合管理像操作系统这样的大型多语言项目。

-   **`meson.build`:** 项目根目录下的主 Meson 配置文件，定义了整个 HNX 项目的构建逻辑、子项目、目标以及依赖关系。
-   **`meson_options.txt`:** 包含了可配置的构建选项，例如目标架构 (`ARCH`)、开发板 (`BOARD`) 和构建配置文件 (`PROFILE`，如 `debug` 或 `release`)。开发者可以通过这些选项定制构建过程。
-   **子项目管理:** Meson 能够优雅地管理多个独立的子项目（例如 `kernel` 和 `space`），并协调它们之间的构建顺序和依赖。

### Python 脚本

HNX 大量使用 Python 脚本来自动化构建过程中的一些特定任务，例如配置管理、镜像创建和 QEMU 运行。

-   **`scripts/configure.py`:** 用于解析 `configs/` 目录下的配置文件，并生成 Meson 构建所需的参数。
-   **`scripts/create-image.py`:** 负责将编译好的内核和用户空间组件打包成最终的系统镜像文件 (例如 Initrd)，供 QEMU 或实际硬件加载运行。
-   **`scripts/run-qemu.py`:** 封装了 QEMU 的调用参数，使得开发者可以方便地在仿真环境中运行 HNX，并支持调试选项。
-   **`scripts/verify_abi.py` / `verify_version.py`:** 用于验证 ABI 兼容性或版本信息，确保系统组件之间的一致性。

### Rust 工作空间 (Rust Workspace)

HNX 利用 Rust 的工作空间 (Workspace) 特性来组织其 Rust 代码。工作空间允许将多个相关的 crate（例如 `hnx-kernel`、`shared`、`hnxlib` 和各种用户空间服务）放在同一个项目中，并统一管理它们的依赖和构建。

-   **`Cargo.toml` (根目录):** 定义了工作空间的成员 (`members`)，并可能包含一些顶层的工作空间级配置。
-   **独立的 Crate:** `kernel/Cargo.toml` 和 `shared/Cargo.toml`，以及 `space/Cargo.toml` 下的各个服务和库，都定义了各自独立的 Rust crate。
-   **依赖管理:** Rust Cargo 负责管理各个 crate 之间的依赖关系，确保所有组件都能正确编译。

## 构建流程

一个典型的 HNX 构建流程如下：

1.  **配置构建环境:** 运行 `meson setup` 命令，指定目标架构、开发板和构建配置文件。例如：
    ```bash
    meson setup build --option ARCH=aarch64 --option BOARD=qemu-virt --option PROFILE=debug
    ```
2.  **编译组件:** 使用 Meson 编译所有或指定的目标。Meson 会调用 Cargo 来编译 Rust crate，并协调其他构建步骤。
    ```bash
    meson compile -C build         # 构建所有默认目标
    meson compile -C build kernel  # 仅构建内核
    ```
3.  **创建系统镜像:** 运行相应的 Meson 目标来生成完整的系统镜像，例如包含 Initrd。
    ```bash
    meson compile -C build image
    meson compile -C build simple-image
    ```
4.  **运行与测试:** 使用 `meson run` 命令在 QEMU 中运行系统或执行测试。
    ```bash
    meson run -C build run
    meson test -C build
    ```

## 交叉编译

HNX 从设计之初就支持交叉编译。通过 Meson 的 `ARCH` 选项和 Rust 的 `target` 配置 (`aarch64-unknown-none`)，可以轻松地为不同目标架构构建系统组件。

-   **`targets/` 目录:** 包含了自定义的 Rust 目标三元组 (target triples) 定义，如 `aarch64-unknown-hnx-kernel.json`，指导 Cargo 如何为 HNX 编译代码。
-   **工具链:** 构建系统会确保使用正确配置的 Rust nightly 工具链，并安装所需的交叉编译目标。

这套构建系统使得 HNX 的开发和维护变得高效，并为未来的功能扩展和平台支持提供了坚实的基础。
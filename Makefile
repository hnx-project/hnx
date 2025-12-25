# hnx/Makefile
.PHONY: all kernel space image clean distclean help configure toolchain \
        version version-sync version-bump version-set-prerelease version-init \
        version-check version-clean

# 配置变量
export PROJECT_ROOT := $(CURDIR)
export BUILD_ROOT := $(PROJECT_ROOT)/build
export CONFIG_DIR := $(PROJECT_ROOT)/configs
export TOOLCHAIN_DIR := $(PROJECT_ROOT)/toolchain

# python 环境
PYTHON = poetry run python

# 版本管理脚本
VERSION_SCRIPT = $(PYTHON) scripts/version.py
VERIFY_VERSION_SCRIPT = $(PYTHON) scripts/verify_version.py

# 目标架构和开发板
ARCH ?= aarch64
BOARD ?= qemu-virt
PROFILE ?= debug

# 导出环境变量
export ARCH BOARD PROFILE
export KERNEL_TARGET = $(ARCH)-unknown-none
export SPACE_TARGET = $(ARCH)-unknown-none

# 版本信息（从脚本获取）
VERSION := $(shell $(VERSION_SCRIPT) read 2>/dev/null || echo "0.0.0")
FULL_VERSION := $(shell $(VERSION_SCRIPT) read --full 2>/dev/null || echo "0.0.0+unknown")
GIT_HASH := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE := $(shell date -u +%Y%m%d)

all: image

# ========== 版本管理目标 ==========

# 显示版本信息
version:
	@echo "========= HNX Version Information ========="
	@echo "Version: $(VERSION)"
	@echo "Full version: $(FULL_VERSION)"
	@echo "Git commit: $(GIT_HASH)"
	@echo "Build date: $(BUILD_DATE)"
	@echo "Architecture: $(ARCH)"
	@echo "Board: $(BOARD)"
	@echo "Profile: $(PROFILE)"
	@echo "==========================================="
	@echo ""

# 初始化版本文件
version-init:
	@echo "========= Initializing version file ========="
	@$(VERSION_SCRIPT) init \
		--major 0 \
		--minor 1 \
		--patch 0 \
		--prerelease alpha.1
	@echo "Version file initialized to $(shell $(VERSION_SCRIPT) read)"
	@echo ""

# 同步版本到所有子项目
version-sync:
	@echo "========= Syncing version to all subprojects ========="
	@$(VERSION_SCRIPT) sync
	@echo "Version synced successfully"
	@echo ""

# 检查版本一致性
version-check:
	@echo "========= Checking version consistency ========="
	@$(VERIFY_VERSION_SCRIPT)
	@echo "Version consistency check passed"
	@echo ""

# 递增版本号
version-bump-major:
	@echo "========= Bumping major version ========="
	@$(VERSION_SCRIPT) bump major
	@echo "Version bumped to $(shell $(VERSION_SCRIPT) read)"
	@echo ""

version-bump-minor:
	@echo "========= Bumping minor version ========="
	@$(VERSION_SCRIPT) bump minor
	@echo "Version bumped to $(shell $(VERSION_SCRIPT) read)"
	@echo ""

version-bump-patch:
	@echo "========= Bumping patch version ========="
	@$(VERSION_SCRIPT) bump patch
	@echo "Version bumped to $(shell $(VERSION_SCRIPT) read)"
	@echo ""

# 设置预发布标签
version-set-prerelease:
ifndef TAG
	$(error TAG is not set. Usage: make version-set-prerelease TAG=<tag>)
endif
	@echo "========= Setting prerelease tag to '$(TAG)' ========="
	@$(VERSION_SCRIPT) set-prerelease "$(TAG)"
	@echo "Prerelease tag set to '$(TAG)'"
	@echo ""

# 清理版本生成的文件
version-clean:
	@echo "========= Cleaning version generated files ========="
	@rm -f include/hnx/abi/version.h 2>/dev/null || true
	@rm -f src/kernel/src/version.rs 2>/dev/null || true
	@echo "Version generated files cleaned"
	@echo ""

# ========== 原构建系统目标 ==========

# 初始化 python 环境
init:
	@echo "========= Initializing python environment ========="
	@poetry env use 3.11 || poetry python install >=3.11 || poetry env use 3.11
	@poetry install
	@echo "========= Python environment initialized ========="
	@echo ""

# 配置系统
configure: init version-check
	@echo "========= Configuring for board: $(BOARD), arch: $(ARCH) ========="
	@mkdir -p $(BUILD_ROOT)/config
	@$(PYTHON) scripts/configure.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--profile $(PROFILE) \
		--output-dir $(BUILD_ROOT)/config
	@echo "========= Configuration completed ========="
	@echo ""

# 构建工具链
toolchain:
	@echo "========= Building toolchain ========="
	@cd $(TOOLCHAIN_DIR) && ./build.sh --arch $(ARCH)
	@echo "========= Toolchain built ========="
	@echo ""

# 构建内核（集成版本信息）
kernel: configure check-abi version-sync
	@echo "========= Building kernel v$(VERSION) for target: $(KERNEL_TARGET) ========="
	@RUSTFLAGS="-A warnings" RUST_BACKTRACE=1 cargo build -p hnx-kernel \
		--target $(KERNEL_TARGET)
	@mkdir -p $(BUILD_ROOT)/kernel/$(PROFILE)
	@cp target/$(KERNEL_TARGET)/$(PROFILE)/hnx-kernel \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel-$(VERSION).bin
	@ln -sf hnx-kernel-$(VERSION).bin \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin
	@echo "========= Kernel built ========="
	@echo "Kernel saved as: $(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel-$(VERSION).bin"
	@echo ""

# 检查 ABI 一致性
check-abi:
	@echo "========= Build ABI consistency ========="
	@RUSTFLAGS="-A warnings" cargo build -p hnx-abi --target $(KERNEL_TARGET) --release
	@echo "========= ABI check passed ========="
	@echo ""

# 构建空间组件（集成版本信息）
space: kernel
	@echo "========= Building space components v$(VERSION) for target: $(SPACE_TARGET) ========="
	@cd src/space && \
		RUSTFLAGS="-A warnings" cargo build \
			--workspace \
			--target $(SPACE_TARGET) 
	@mkdir -p $(BUILD_ROOT)/space/$(PROFILE)
	@cp -r src/space/target/$(SPACE_TARGET)/$(PROFILE)/* \
		$(BUILD_ROOT)/space/$(PROFILE)/
	@echo "========= Space components built ========="
	@echo ""

# 创建系统镜像（集成版本信息）
image: kernel space version-check
	@echo "========= Creating system image v$(VERSION) ========="
	@mkdir -p $(BUILD_ROOT)/images
	@$(PYTHON) scripts/create-image.py \
		--kernel $(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel-$(VERSION).bin \
		--space-dir $(BUILD_ROOT)/space/$(PROFILE) \
		--arch $(ARCH) \
		--board $(BOARD) \
		--output $(BUILD_ROOT)/images/hnx-$(VERSION)-$(ARCH)-$(BOARD).img
	@ln -sf hnx-$(VERSION)-$(ARCH)-$(BOARD).img \
		$(BUILD_ROOT)/images/hnx-$(ARCH)-$(BOARD).img
	@echo "========= System image created ========="
	@echo "Image saved as: $(BUILD_ROOT)/images/hnx-$(VERSION)-$(ARCH)-$(BOARD).img"
	@echo ""

# 创建系统镜像 simple_initrd no_compress
simple-image: kernel space version-check
	@echo "========= Creating simple system image v$(VERSION) ========="
	@mkdir -p $(BUILD_ROOT)/images
	@$(PYTHON) scripts/create-image.py \
		--kernel $(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel-$(VERSION).bin \
		--space-dir $(BUILD_ROOT)/space/$(PROFILE) \
		--arch $(ARCH) \
		--board $(BOARD) \
		--simple-initrd \
		--no-compress \
		--output $(BUILD_ROOT)/images/hnx-simple-$(VERSION)-$(ARCH)-$(BOARD).img
	@ln -sf hnx-simple-$(VERSION)-$(ARCH)-$(BOARD).img \
		$(BUILD_ROOT)/images/hnx-simple-$(ARCH)-$(BOARD).img
	@echo "========= System image created ========="
	@echo "Image saved as: $(BUILD_ROOT)/images/hnx-simple-$(VERSION)-$(ARCH)-$(BOARD).img"
	@echo ""

run-kernel: kernel
	@echo "========= Running kernel v$(VERSION) in QEMU ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--headless \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel-$(VERSION).bin
	@echo "========= QEMU exited ========="
	@echo ""

# 运行 QEMU（使用配置）
run: image
	@echo "========= Running v$(VERSION) in QEMU ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--config-dir $(BUILD_ROOT)/config \
		--headless \
		$(BUILD_ROOT)/images/hnx-$(VERSION)-$(ARCH)-$(BOARD).img
	@echo "========= QEMU exited ========="
	@echo ""

# 运行带超时的 QEMU
run-simple: kernel space
	@echo "========= Running QEMU test v$(VERSION) (30s timeout) ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--headless \
		--timeout 30 \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel-$(VERSION).bin
	@echo "========= QEMU test completed ========="
	@echo ""

# 运行带 GDB 的 QEMU
debug: image
	@echo "========= Running QEMU with GDB debugger v$(VERSION) ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--headless \
		--gdb \
		--timeout 0 \
		$(BUILD_ROOT)/images/hnx-$(VERSION)-$(ARCH)-$(BOARD).img
	@echo "========= Debug session ended ========="
	@echo ""

# 测试（包含版本测试）
test: version-check
	@echo "========= Running tests for v$(VERSION) ========="
	@cd src/kernel && cargo test --lib
	@cd src/space && cargo test --workspace
	@echo "========= Tests completed ========="
	@echo ""

# 快速构建（仅内核）
quick: configure
	@echo "========= Quick build (kernel only) v$(VERSION) ========="
	@cd src/kernel && RUSTFLAGS="-A warnings" cargo build --target $(KERNEL_TARGET) --release
	@echo "========= Quick build completed ========="
	@echo ""

# 清理（保留版本文件）
clean:
	@echo "========= Cleaning build artifacts ========="
	@cargo clean
	@rm -rf $(BUILD_ROOT)/kernel
	@rm -rf $(BUILD_ROOT)/space
	@rm -rf $(BUILD_ROOT)/config
	@rm -f $(BUILD_ROOT)/images/hnx-*.img 2>/dev/null || true
	@rm -f $(BUILD_ROOT)/images/hnx-simple-*.img 2>/dev/null || true
	@find . -type d -name "target" -exec rm -rf {} +
	@echo "========= Build artifacts cleaned ========="
	@echo ""

distclean: clean version-clean
	@echo "========= Cleaning everything ========="
	@cd $(TOOLCHAIN_DIR) && make clean
	@rm -rf targets/*.json
	@rm -rf .mypy_cache
	@rm -rf __pycache__
	@rm -f VERSION
	@echo "========= Everything cleaned ========="
	@echo ""

# 显示配置（包含版本信息）
config: version
	@echo "Configuration details:"
	@echo "  ARCH: $(ARCH)"
	@echo "  BOARD: $(BOARD)"
	@echo "  PROFILE: $(PROFILE)"
	@if [ -f "$(BUILD_ROOT)/config/config.json" ]; then \
		echo "  Config file: $(BUILD_ROOT)/config/config.json"; \
		echo "  QEMU settings:"; \
		$(PYTHON) -c "import json; \
			data = json.load(open('$(BUILD_ROOT)/config/config.json')); \
			qemu = data.get('qemu', {}); \
			print(f'    Machine: {qemu.get(\"machine\", \"N/A\")}'); \
			print(f'    CPU: {qemu.get(\"cpu\", \"N/A\")}'); \
			print(f'    Memory: {qemu.get(\"memory\", \"N/A\")}'); \
		"; \
	else \
		echo "  Config not found. Run 'make configure' first."; \
	fi

# 帮助（更新版）
help:
	@echo "HNX Build System v$(VERSION)"
	@echo ""
	@echo "Usage:"
	@echo "  make [target] [options]"
	@echo ""
	@echo "Build Targets:"
	@echo "  make configure     Configure the build"
	@echo "  make toolchain     Build the toolchain"
	@echo "  make kernel        Build the kernel only"
	@echo "  make space         Build space components"
	@echo "  make image         Build complete system image"
	@echo "  make simple-image  Build simple system image"
	@echo "  make run           Run in QEMU"
	@echo "  make run-simple    Run QEMU with 30s timeout"
	@echo "  make debug         Run QEMU with GDB debugger"
	@echo "  make test          Run tests"
	@echo "  make quick         Quick build (kernel only)"
	@echo "  make clean         Clean build artifacts"
	@echo "  make distclean     Clean everything"
	@echo "  make config        Show current configuration"
	@echo ""
	@echo "Version Targets:"
	@echo "  make version               Show version information"
	@echo "  make version-init          Initialize version file"
	@echo "  make version-sync          Sync version to all subprojects"
	@echo "  make version-check         Check version consistency"
	@echo "  make version-bump-major    Bump major version"
	@echo "  make version-bump-minor    Bump minor version"
	@echo "  make version-bump-patch    Bump patch version"
	@echo "  make version-set-prerelease TAG=<tag>  Set prerelease tag"
	@echo "  make version-clean         Clean version generated files"
	@echo ""
	@echo "Options:"
	@echo "  ARCH=     Target architecture (aarch64, x86_64, riscv64)"
	@echo "  BOARD=    Board name (qemu-virt, raspberry-pi4)"
	@echo "  PROFILE=  Build profile (debug, release)"
	@echo "  TAG=      Prerelease tag for version-set-prerelease"
	@echo ""
	@echo "Current Configuration:"
	@echo "  Version: $(VERSION)"
	@echo "  Arch: $(ARCH)"
	@echo "  Board: $(BOARD)"
	@echo ""
	@echo "Examples:"
	@echo "  make ARCH=aarch64 BOARD=qemu-virt image"
	@echo "  make BOARD=raspberry-pi4 run-simple"
	@echo "  make PROFILE=release image"
	@echo "  make version-bump-minor"
	@echo "  make version-set-prerelease TAG=beta.1"
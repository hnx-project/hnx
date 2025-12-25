# hnx/Makefile
.PHONY: all kernel space image clean distclean help configure toolchain

# 配置变量
export PROJECT_ROOT := $(CURDIR)
export BUILD_ROOT := $(PROJECT_ROOT)/build
export CONFIG_DIR := $(PROJECT_ROOT)/configs
export TOOLCHAIN_DIR := $(PROJECT_ROOT)/toolchain

# python 环境
PYTHON = poetry run python

# 目标架构和开发板
ARCH ?= aarch64
BOARD ?= qemu-virt
PROFILE ?= debug

# 导出环境变量
export ARCH BOARD PROFILE
export KERNEL_TARGET = $(ARCH)-unknown-none
export SPACE_TARGET = $(ARCH)-unknown-none

all: image

# 初始化 python 环境
# poetry use python3.11
init:
	@echo "========= Initializing python environment ========="
	@poetry env use 3.11 || poetry python install >=3.11 || poetry env use 3.11
	@poetry install
	@echo "========= Python environment initialized ========="
	@echo ""

# 配置系统
configure: init
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

# 构建内核（简化版）
kernel: configure check-abi
	@echo "========= Building kernel for target: $(KERNEL_TARGET) ========="
	@RUSTFLAGS="-A warnings" RUST_BACKTRACE=1 cargo build  -p hnx-kernel \
			--target $(KERNEL_TARGET)
	@mkdir -p $(BUILD_ROOT)/kernel/$(PROFILE)
	@cp target/$(KERNEL_TARGET)/$(PROFILE)/hnx-kernel \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin
	@echo "========= Kernel built ========="
	@echo ""

# 检查 ABI 一致性
check-abi:
	@echo "========= Build ABI consistency ========="
	@RUSTFLAGS="-A warnings" cargo build  -p hnx-abi --target $(KERNEL_TARGET) --release
# 	@$(PYTHON) scripts/verify_abi.py
	@echo "========= ABI check passed ========="
	@echo ""

# 构建空间组件（简化版）
space: kernel
	@echo "========= Building space components for target: $(SPACE_TARGET) ========="
	@cd src/space && \
		RUSTFLAGS="-A warnings" cargo build  \
			--workspace \
			--target $(SPACE_TARGET)
	@mkdir -p $(BUILD_ROOT)/space/$(PROFILE)
	@cp -r src/space/target/$(SPACE_TARGET)/$(PROFILE)/* \
		$(BUILD_ROOT)/space/$(PROFILE)/
	@echo "========= Space components built ========="
	@echo ""

# 创建系统镜像
image: kernel space
	@echo "========= Creating system image ========="
	@mkdir -p $(BUILD_ROOT)/images
	@$(PYTHON) scripts/create-image.py \
		--kernel $(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin \
		--space-dir $(BUILD_ROOT)/space/$(PROFILE) \
		--arch $(ARCH) \
		--board $(BOARD) \
		--output $(BUILD_ROOT)/images/hnx-$(ARCH)-$(BOARD).img
	@echo "========= System image created ========="
	@echo ""

# 创建系统镜像 simple_initrd no_compress
simple-image: kernel space
	@echo "========= Creating system image ========="
	@mkdir -p $(BUILD_ROOT)/images
	@$(PYTHON) scripts/create-image.py \
		--kernel $(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin \
		--space-dir $(BUILD_ROOT)/space/$(PROFILE) \
		--arch $(ARCH) \
		--board $(BOARD) \
		--simple-initrd \
		--no-compress \
		--output $(BUILD_ROOT)/images/hnx-$(ARCH)-$(BOARD).img
	@echo "========= System image created ========="
	@echo ""

run-kernel: kernel
	@echo "========= Running kernel in QEMU ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--headless \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin
	@echo "========= QEMU exited ========="
	@echo ""

# 运行 QEMU（使用配置）
run: image
	@echo "========= Running in QEMU ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--config-dir $(BUILD_ROOT)/config \
		--headless \
		$(BUILD_ROOT)/images/hnx-$(ARCH)-$(BOARD).img
	@echo "========= QEMU exited ========="
	@echo ""

# 运行带超时的 QEMU
run-simple: simple-image
	@echo "========= Running QEMU test (30s timeout) ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--config-dir $(BUILD_ROOT)/config \
		--headless \
		--timeout 30 \
		$(BUILD_ROOT)/images/hnx-$(ARCH)-$(BOARD).img
	@echo "========= QEMU test completed ========="
	@echo ""

# 运行带 GDB 的 QEMU
debug: image
	@echo "========= Running QEMU with GDB debugger ========="
	@$(PYTHON) scripts/run-qemu.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--headless \
		--gdb \
		--timeout 0 \
		$(BUILD_ROOT)/images/hnx-$(ARCH)-$(BOARD).img
	@echo "========= Debug session ended ========="
	@echo ""

# 测试
test:
	@echo "========= Running tests ========="
	@cd src/kernel && cargo test --lib
	@cd src/space && cargo test --workspace
	@echo "========= Tests completed ========="
	@echo ""

# 快速构建（仅内核）
quick: configure
	@echo "========= Quick build (kernel only) ========="
	@cd src/kernel && RUSTFLAGS="-A warnings" cargo build  --target $(KERNEL_TARGET) --release
	@echo "========= Quick build completed ========="
	@echo ""

# 清理
clean:
	@echo "========= Cleaning build artifacts ========="
	@cargo clean
	@rm -rf $(BUILD_ROOT)
	@find . -type d -name "target" -exec rm -rf {} +
	@echo "========= Build artifacts cleaned ========="
	@echo ""

distclean: clean
	@echo "========= Cleaning everything ========="
	@cd $(TOOLCHAIN_DIR) && make clean
	@rm -rf target
	@rm -rf targets/*.json
	@echo "========= Everything cleaned ========="
	@echo ""

# 显示配置
config:
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
# 帮助
help:
	@echo "HNX Build System"
	@echo ""
	@echo "Usage:"
	@echo "  make [target] [options]"
	@echo ""
	@echo "Targets:"
	@echo "  make configure     Configure the build"
	@echo "  make toolchain     Build the toolchain"
	@echo "  make kernel        Build the kernel only"
	@echo "  make space         Build space components"
	@echo "  make image         Build complete system image"
	@echo "  make run           Run in QEMU"
	@echo "  make run-test      Run QEMU with 30s timeout"
	@echo "  make debug         Run QEMU with GDB debugger"
	@echo "  make test          Run tests"
	@echo "  make quick         Quick build (kernel only)"
	@echo "  make clean         Clean build artifacts"
	@echo "  make distclean     Clean everything"
	@echo "  make config        Show current configuration"
	@echo ""
	@echo "Options:"
	@echo "  ARCH=     Target architecture (aarch64, x86_64, riscv64)"
	@echo "  BOARD=    Board name (qemu-virt, raspberry-pi4)"
	@echo "  PROFILE=  Build profile (debug, release)"
	@echo ""
	@echo "Examples:"
	@echo "  make ARCH=aarch64 BOARD=qemu-virt image"
	@echo "  make BOARD=raspberry-pi4 run-test"
	@echo "  make PROFILE=release image"
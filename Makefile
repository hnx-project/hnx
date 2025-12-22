# hnx/Makefile
.PHONY: all kernel space image clean distclean help configure toolchain

# 配置变量
export PROJECT_ROOT := $(CURDIR)
export BUILD_ROOT := $(PROJECT_ROOT)/build
export CONFIG_DIR := $(PROJECT_ROOT)/configs
export TOOLCHAIN_DIR := $(PROJECT_ROOT)/toolchain

# python 环境
PYTHON = poetry run python

# 目标架构
ARCH ?= aarch64
BOARD ?= qemu-virt
PROFILE ?= dev

# 导出环境变量
export ARCH BOARD PROFILE
export KERNEL_TARGET =  $(PROJECT_ROOT)/targets/$(ARCH)-unknown-hnx-kernel.json
export SPACE_TARGET =  $(PROJECT_ROOT)/targets/$(ARCH)-unknown-hnx.json

all: image

# 初始化 python 环境
# poetry use python3.11
init:
	@echo "Initializing python environment..."
	@poetry env use 3.11 || poetry python install >=3.11 || poetry env use 3.11
	@poetry install

# 配置系统
configure: init
	@echo "Configuring for board: $(BOARD), arch: $(ARCH)"
	@mkdir -p $(BUILD_ROOT)/config
	@$(PYTHON) scripts/configure.py \
		--arch $(ARCH) \
		--board $(BOARD) \
		--profile $(PROFILE) \
		--output $(BUILD_ROOT)/config

# 构建工具链
toolchain:
	@echo "Building toolchain..."
	@cd $(TOOLCHAIN_DIR) && ./build.sh --arch $(ARCH)

# 构建内核（简化版）
kernel: configure
	@echo "Building kernel for target: $(KERNEL_TARGET)"
	@cd src/kernel && \
		cargo build \
			--target $(KERNEL_TARGET) \
			--profile $(PROFILE)
	@mkdir -p $(BUILD_ROOT)/kernel/$(PROFILE)
	@cp target/$(KERNEL_TARGET)/$(PROFILE)/hnx-kernel \
		$(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin

# 构建空间组件（简化版）
space: kernel
	@echo "Building space components for target: $(SPACE_TARGET)"
	@cd src/space && \
		cargo build \
			--workspace \
			--target $(SPACE_TARGET) \
			--profile $(PROFILE)
	@mkdir -p $(BUILD_ROOT)/space/$(PROFILE)
	@cp -r target/$(SPACE_TARGET)/$(PROFILE)/* \
		$(BUILD_ROOT)/space/$(PROFILE)/

# 创建系统镜像
image: kernel space
	@echo "Creating system image..."
	@mkdir -p $(BUILD_ROOT)/images
	@python3 scripts/create-image.py \
		--kernel $(BUILD_ROOT)/kernel/$(PROFILE)/hnx-kernel.bin \
		--space $(BUILD_ROOT)/space/$(PROFILE) \
		--config $(BUILD_ROOT)/config \
		--output $(BUILD_ROOT)/images/hnx-$(BOARD).img

# 快速构建（开发用）
quick: configure
	@echo "Quick build..."
	@cd src/kernel && cargo build --target $(KERNEL_TARGET) --release
	@cd src/space && cargo build --workspace --target $(SPACE_TARGET) --release
	@echo "Quick build completed"

# 运行 QEMU
run: image
	@echo "Running in QEMU..."
	@python3 scripts/run-qemu.py \
		--image $(BUILD_ROOT)/images/hnx-$(BOARD).img \
		--arch $(ARCH) \
		--board $(BOARD)

# 测试
test:
	@echo "Running tests..."
	@cd src/kernel && cargo test --lib
	@cd src/space && cargo test --workspace

# 清理
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf $(BUILD_ROOT)

distclean: clean
	@echo "Cleaning everything..."
	@cd $(TOOLCHAIN_DIR) && make clean
	@rm -rf target

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
	@echo "  make test          Run tests"
	@echo "  make clean         Clean build artifacts"
	@echo "  make distclean     Clean everything"
	@echo ""
	@echo "Options:"
	@echo "  ARCH=     Target architecture (aarch64, x86_64, riscv64)"
	@echo "  BOARD=    Board name (qemu-virt, raspberry-pi4)"
	@echo "  PROFILE=  Build profile (debug, release)"
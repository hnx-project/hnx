# HNX 内核构建系统

# 配置
ARCH ?= aarch64
CPU ?= cortex-a72
TARGET := $(ARCH)-unknown-none
FEATURES := $(ARCH)
# SELFTEST 已移除，常规构建路径
CARGO_FLAGS := --target $(TARGET) --features "$(FEATURES)"

# 输出文件
KERNEL_ELF := target/$(TARGET)/debug/hnx-kernel
KERNEL_BIN := $(KERNEL_ELF).bin

# QEMU 参数
QEMU := qemu-system-$(ARCH)
QEMU_FLAGS := -machine virt -cpu $(CPU) -smp 1 -m 1G
QEMU_FLAGS += -nographic -monitor none -serial stdio
QEMU_FLAGS += -kernel $(KERNEL_ELF)
QEMU_FLAGS += -no-reboot
INITRD_FILE ?= ../hnx-toolchain/hello/hello.elf
USE_INITRD ?= 0
ifeq ($(USE_INITRD),1)
QEMU_FLAGS += -device loader,file=$(INITRD_FILE),addr=0x42000000,force-raw=on
endif
# hnx-kernel/target/aarch64-unknown-none/debug/hnx-kernel
.PHONY: all build clean run debug

all: build

build:
	@echo "Building HNX kernel for $(ARCH)..."
	cargo build $(CARGO_FLAGS)

lint:
	@echo "Lint (clippy) for $(ARCH)..."
	cargo clippy $(CARGO_FLAGS) || true

check:
	@echo "Typecheck for $(ARCH)..."
	cargo check $(CARGO_FLAGS)

fmt:
	@echo "Formatting..."
	cargo fmt

clean:
	cargo clean

run: kill-qemu build
	@echo "Starting QEMU with auto-timeout..."
	@TIMEOUT=$${HNX_TIMEOUT:-10}; \
	if [ $$TIMEOUT -gt 60 ]; then TIMEOUT=60; fi; \
	echo "Auto-terminate after $${TIMEOUT}s"; \
	echo "======= HNX Kernel Booting ======="; \
	rm -f serial.log; \
	$(QEMU) $(QEMU_FLAGS) & \
	QPID=$$!; \
	{ \
		sleep $$TIMEOUT; \
		if kill -0 $$QPID 2>/dev/null; then \
			echo "Timeout reached ($$TIMEOUT s), sending TERM to QEMU (PID $$QPID)"; \
			kill -TERM $$QPID 2>/dev/null || true; \
			for i in 1 2 3 4 5; do \
				sleep 1; \
				kill -0 $$QPID 2>/dev/null || exit 0; \
			done; \
			if kill -0 $$QPID 2>/dev/null; then \
				echo "QEMU did not exit after TERM, sending KILL to PID $$QPID"; \
				kill -KILL $$QPID 2>/dev/null || true; \
			fi; \
		fi; \
	} & \
	WPID=$$!; \
	wait $$QPID; \
	kill $$WPID 2>/dev/null || true
	@if [ -f serial.log ]; then \
	  echo "----- serial.log (CR stripped) -----"; \
	  LC_ALL=C tr -d '\r' < serial.log; \
	fi

debug: build
	@echo "Starting QEMU with GDB stub..."
	$(QEMU) $(QEMU_FLAGS) -s -S &
	@echo "Now connect GDB to localhost:1234"

objdump:
	cargo objdump --target $(TARGET) -- -d $(KERNEL_ELF)

size:
	cargo size --target $(TARGET) -- -A $(KERNEL_ELF)

# 创建裸金属目标
install-target:
	@echo "Installing target $(TARGET)..."
	rustup target add $(TARGET)
kill-qemu:
	-pkill -9 qemu-system-$(ARCH) 2>/dev/null || true
help:
	@echo "HNX 内核构建命令:"
	@echo "  make build      - 构建内核"
	@echo "  make clean      - 清理构建"
	@echo "  make run        - 在 QEMU 中运行 (支持 HNX_TIMEOUT=<秒> 自动关闭)"
	@echo "  make debug      - 启动调试会话"
	@echo "  make objdump    - 反汇编内核"
	@echo "  make size       - 查看内核大小"
	@echo "  make install-target - 安装目标"
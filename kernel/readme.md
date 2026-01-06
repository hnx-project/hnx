# hnx-kernel

## tree
```shell
hnx-kernel/
├── src/
│   ├── arch/                    # 架构相关代码
│   │   ├── common/             # 跨架构通用代码
│   │   │   ├── boot/           # 通用启动框架
│   │   │   ├── interrupt/      # 中断抽象层
│   │   │   ├── mmu/           # MMU 抽象层
│   │   │   └── timer/         # 定时器抽象层
│   │   │
│   │   ├── aarch64/           # ARMv8 64位实现
│   │   │   ├── boot/          # ARM 启动代码
│   │   │   ├── mmu/           # ARM MMU 实现
│   │   │   ├── interrupt/     # ARM 中断控制器
│   │   │   ├── smp/           # ARM 多核启动
│   │   │   ├── cpu/           # CPU 特性检测
│   │   │   └── mod.rs         # 架构模块导出
│   │   │
│   │   ├── riscv64/           # RISC-V 64位（未来）
│   │   │   ├── boot/
│   │   │   ├── mmu/
│   │   │   └── mod.rs
│   │   │
│   │   ├── x86_64/            # x86_64（未来）
│   │   │   ├── boot/
│   │   │   └── mod.rs
│   │   │
│   │   └── mod.rs             # 架构选择入口
│   │
│   ├── core/                   # 架构无关核心代码
│   │   ├── sync/              # 同步原语（自旋锁、信号量等）
│   │   ├── alloc/             # 内存分配器（buddy + slab）
│   │   ├── scheduler/         # 调度器（通用算法）
│   │   ├── ipc/               # 进程间通信
│   │   ├── time/              # 时间管理（通用部分）
│   │   └── mod.rs
│   │
│   ├── drivers/               # 设备驱动（架构无关接口）
│   │   ├── bus/               # 总线抽象层
│   │   ├── uart/              # 串口抽象
│   │   │   ├── trait.rs       # Uart trait 定义
│   │   │   ├── pl011.rs       # ARM PL011 实现
│   │   │   └── 8250.rs        # x86 8250 实现（未来）
│   │   ├── pci/               # PCI 抽象
│   │   ├── storage/           # 存储设备
│   │   ├── network/           # 网络设备
│   │   ├── input/             # 输入设备
│   │   └── mod.rs
│   │
│   ├── fs/                    # 文件系统
│   │   ├── vfs/               # 虚拟文件系统层
│   │   ├── ramfs/             # 内存文件系统
│   │   ├── initrd/            # initrd 支持
│   │   ├── devfs/             # 设备文件系统
│   │   ├── procfs/            # proc 文件系统
│   │   ├── ohlink/            # ohlink 加载器
│   │   └── mod.rs
│   │
│   ├── memory/                # 内存管理（架构无关接口）
│   │   ├── vm/               # 虚拟内存管理
│   │   ├── map/              # 内存映射
│   │   ├── kheap/            # 内核堆
│   │   └── mod.rs
│   │
│   ├── process/               # 进程管理
│   │   ├── task/             # 任务管理
│   │   ├── syscall/          # 系统调用（ABI 无关部分）
│   │   ├── signal/           # 信号处理
│   │   └── mod.rs
│   │
│   ├── network/               # 网络协议栈
│   │   ├── stack/            # 网络协议栈
│   │   ├── socket/           # socket 接口
│   │   └── mod.rs
│   │
│   ├── security/              # 安全模块
│   │   ├── caps/             # 能力机制
│   │   └── mod.rs
│   │
│   ├── lib/                   # 内核库
│   │   ├── rust/             # Rust 辅助库
│   │   ├── c/                # C 标准库子集
│   │   └── mod.rs
│   │
│   └── main.rs               # 内核主入口
│
├── include/                   # 内核头文件
│   ├── kernel/               # 内核通用头文件
│   └── asm/                  # 汇编宏（架构相关）
│
├── configs/                   # 内核配置
│   ├── aarch64.toml          # ARM 配置
│   ├── riscv64.toml          # RISC-V 配置
│   └── x86_64.toml           # x86 配置
│
├── scripts/                   # 构建脚本
│   ├── link.ld.S             # 链接脚本模板
│   └── make_config.py        # 配置生成器
│
├── build.rs                   # Cargo 构建脚本
├── Cargo.toml                 # 内核 Cargo 配置
├── Kconfig                    # 内核配置菜单
├── Makefile                   # 顶层构建文件
└── README.md
```

## 3-layer abstract model

```shell
┌─────────────────────────────────────┐
│         架构无关核心代码               │ ← src/core/, src/drivers/, src/fs/, etc.
│    (Arch-Independent Core)          │
├─────────────────────────────────────┤
│          架构抽象层                   │ ← src/arch/common/
│      (Arch Abstraction Layer)       │
├─────────────────────────────────────┤
│       架构具体实现                    │ ← src/arch/aarch64/, src/arch/riscv64/
│   (Arch-Specific Implementation)    │
└─────────────────────────────────────┘

```

## conditional-compilation
```rust
// 使用 cfg 属性
#[cfg(target_arch = "aarch64")]
mod aarch64_specific;

// 使用 cfg 宏
if cfg!(target_arch = "aarch64") {
    // aarch64 特定代码
}

// 在函数参数中使用
#[cfg(target_arch = "aarch64")]
fn aarch64_function() { /* ... */ }
```

## module export conventions
```rust
// arch/aarch64/mod.rs
pub mod boot;
pub mod mmu;
pub mod interrupt;

// 重新导出到 arch 模块
pub use self::boot::*;
pub use self::mmu::MmuImpl as Mmu;
```

## build
```shell
# 构建 aarch64 内核
cd hnx-kernel
cargo build --target aarch64-unknown-none --features "aarch64"

# 构建 RISC-V 内核（未来）
cargo build --target riscv64gc-unknown-none-elf --features "riscv64"

# 生成配置
make menuconfig  # 图形界面配置
make defconfig   # 默认配置

# 运行测试
make test ARCH=aarch64
```
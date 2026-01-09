# 架构抽象层 (Architecture Abstraction Layer)

HNX 的架构抽象层位于 `kernel/src/arch`，目标是把“硬件相关的能力”抽象成一组稳定接口，避免内核主体逻辑与具体指令集/寄存器/平台设备强耦合。

## 目录结构

当前代码组织（以 AArch64 为例）：

```text
kernel/src/arch/
  mod.rs
  common/
  traits/
    boot.rs cpu.rs mmu.rs interrupt.rs timer.rs exception.rs mod.rs
  implementations/
    aarch64/
      mod.rs
      boot/ (boot.S + Rust glue)
      cpu/  mmu/ interrupt/ timer/ exception/ smp/ psci/ registers/
```

其中：

- `traits/`：定义跨架构统一接口（trait），内核通用代码只依赖这些接口语义
- `implementations/<arch>/`：把接口落到具体硬件实现（指令、系统寄存器、设备 MMIO）
- `mod.rs`：向上提供稳定门面 API，并选择当前目标架构的实现

## 架构接口（traits）概览

架构抽象层的核心接口位于 `kernel/src/arch/traits`：

- `CpuArch`：CPU 初始化、屏障、WFI、中断开关、特权级等
- `MmuArch`：页表、地址空间、映射/取消映射、TLB 维护等
- `InterruptArch`：中断控制器初始化、IRQ enable/disable、优先级、亲和性等
- `TimerArch`：时钟源/计时、定时器中断、超时回调等
- `ExceptionArch`：异常向量表、同步异常处理、断点/单步（可选）等
- `BootArch`：启动阶段的信息收集与收尾（内存图、设备树、启动参数等）

这些 trait 的目的不是“把一切都抽象到极致”，而是把跨平台所必须的语义边界确定下来：上层只关心“我需要一个可用的页表映射/一个可用的时钟/一个可用的 IRQ 路径”，而不关心“这块芯片的寄存器细节”。

## 门面 API（arch::xxx）

为了让内核其它模块以更少的泛型/trait 约束调用架构能力，`kernel/src/arch/mod.rs` 提供门面 API，例如：

- `arch::platform::init()`：启动早期的架构初始化入口（会触发 Boot early init 与 boot info 初始化）
- `arch::mmu::init()`：初始化 MMU 子系统
- `arch::interrupt::init()`：初始化中断子系统
- `arch::timer::init()`：初始化定时器/时钟源
- `arch::cpu::wait_for_interrupt()`：进入低功耗等待（WFI）

在实现选择上，`arch::current` 会根据 `target_arch` 指向具体实现（例如 `implementations::aarch64`）。

## 启动链路（AArch64）

以 QEMU virt + AArch64 为例，启动过程可理解为三段：

1. `boot.S`：最早期汇编入口 `_start`，完成异常级别切换、栈设置、BSS 清零、向量表设置、早期串口输出等
2. `kernel_main()`：Rust 入口，初始化调试输出与各子系统（arch/mmu/interrupt/timer/对象等）
3. 进入主循环：通过 `arch::cpu::wait_for_interrupt()` 等待事件

目前 AArch64 的 `boot.S` 通过 `global_asm!(include_str!(\"boot.S\"))` 编译进内核二进制，并由链接脚本 `ENTRY(_start)` 指定 ELF 入口点。

## BootInfo 的职责边界

`BootArch/BootInfo` 的定位是“启动阶段的信息汇聚器”，典型包括：

- 内核映像信息：基址、大小、入口点、段信息（.text/.rodata/.data/.bss）
- 物理内存信息：总量/可用量、保留区（例如 initrd、设备保留内存）
- 设备发现信息：设备树/ACPI（可选，当前 AArch64 暂未完整实现）

这些信息通常会被 MMU 初始化、物理内存分配器初始化、驱动初始化等阶段消费。

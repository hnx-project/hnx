# HNX Kernel Development Session Summary - ONGOING

## 问题描述
HNX microkernel 的 init 进程无法成功切换到 EL0 用户态执行。系统在 ERET 后立即触发 Prefetch Abort 异常，无限循环在异常向量表地址 `0xFFFF800000000A00`。

## 根本原因分析 ⚠️ 复杂问题

经过深入调试，发现了一个核心的架构问题：

### L1_TABLE 初始化与地址空间问题 ❌ 未完全解决

**问题根源**：
1. **L1_TABLE 物理地址**: 0x40081000 (在 `.data.boot` 段)
2. **内核运行地址**: 0xFFFF800000000000+ (高半地址空间)
3. **地址访问冲突**:
   - 在 `mmu_enable_boot` (boot阶段): 使用物理地址 (0x4008xxxx)，此时写入成功
   - 在 `_start_high` (运行时): 内核在高半地址，但需要访问物理地址的页表
   - lldb 验证: x10=0x40081800, x11=0x40084003，说明写入指令本身正确执行
   - 但系统运行时 L1_TABLE[256] 仍然为 0

**已尝试的修复方案**：

1. ✅ **在 mmu_enable_boot 中初始化** - 使用 adrp/add 访问 L1_TABLE
   - 问题: 写入成功，但后续可能被覆盖或访问到错误地址

2. ✅ **在 _start_high 中初始化** - 使用 movz/movk 加载物理地址
   ```assembly
   /* Load L1_TABLE physical address = 0x40081000 */
   movz x2, #0x1000, lsl #0
   movk x2, #0x4008, lsl #16
   add x2, x2, #0x800          /* L1_TABLE[256] = 0x40081800 */
   
   /* Load L2_TABLE1 = 0x40084000 */
   movz x3, #0x4000, lsl #0
   movk x3, #0x4008, lsl #16
   orr x3, x3, #3              /* L2_TABLE1 | 0x3 */
   
   /* Write L1_TABLE[256] */
   str x3, [x2]
   dsb ish
   tlbi vmalle1
   dsb ish
   isb
   ```
   - 问题: 系统仍然在 ERET 时触发 Prefetch Abort

3. ❌ **修改链接脚本 VMA** - 将 .data.boot VMA 改为高半地址
   - 问题: 编译失败，relocation out of range (boot代码无法访问高半地址)

### 其他已修复的问题

1. ✅ **arch_do_exec 对齐问题** - 移除 exec_msg1-5 调试字符串，添加 `.section .text` 和 `.balign 4`
2. ✅ **BSS 清零** - 禁用 boot.S 的 BSS 清零，避免潜在的页表破坏
3. ✅ **物理内存保护** - 物理内存分配器从 0x40397000 开始，保护页表区域 (0x40081000-0x40085000)
4. ✅ **链接脚本** - .bss 段标记为 NOLOAD，简化符号定义

## 当前代码修改

### 1. src/kernel/src/arch/aarch64/boot.S
- 禁用 BSS 清零
- 在 `_start_high` 添加 L1_TABLE[256] 初始化（使用物理地址）
- 简化 `arch_do_exec`，移除调试代码和 preflight/postflight 调用

### 2. src/kernel/src/arch/aarch64/mmu/mmu_boot.rs
- 移除 mmu_enable_boot 中的 L1_TABLE[256] 初始化（注释说明将在 _start_high 完成）
- 将 L1_TABLE 等符号改为 `pub`，方便调试

### 3. src/kernel/src/scripts/link.ld.S
- .bss 段标记为 NOLOAD
- 简化 __bss_start/__bss_end 定义

## 下一步调试方向

### 方案 A: 深入调试页表状态

使用 lldb 在 ERET 前验证：
1. L1_TABLE[256] 的值 (应为 0x40084003)
2. TTBR1_EL1 的值 (应为 0x40081000)
3. 异常向量表地址映射是否正确

```bash
# lldb 调试
(lldb) b arch_do_exec
(lldb) c
(lldb) x/gx 0x40081800   # 检查 L1_TABLE[256]
(lldb) p/x $ttbr1_el1    # 检查 TTBR1_EL1
(lldb) p/x $vbar_el1     # 检查异常向量表地址
```

### 方案 B: 重新设计页表布局 🔧 推荐

**问题根源**: `.data.boot` 段在物理地址，但内核代码运行在高半地址，导致访问不一致。

**解决方案**: 在 L2_TABLE1 中**显式映射**页表物理地址到高半虚拟地址

1. **在 mmu_enable_boot 中添加映射**:
   ```
   0xFFFF800000081000 -> 0x40081000 (L1_TABLE)
   0xFFFF800000082000 -> 0x40082000 (L1_TABLE0)
   0xFFFF800000083000 -> 0x40083000 (L2_TABLE0)
   0xFFFF800000084000 -> 0x40084000 (L2_TABLE1)
   ```

2. **修改链接脚本**: `.data.boot` VMA 设为高半地址 (0xFFFF800000081000)

3. **L1_TABLE 初始化**: 使用符号地址（自动指向高半 VMA）
   ```rust
   let l1_table = &mut L1_TABLE as *mut AlignedTable;
   let l2_table1_pa = 0x40084000u64;  // 物理地址
   unsafe { (*l1_table).0[256] = l2_table1_pa | 0x3; }
   ```

### 方案 C: 递归页表映射 🔧 高级方案

使用 AArch64 递归页表技术：
- L1_TABLE[511] 指向 L1_TABLE 自身
- 通过 0xFFFF_FF80_0000_0000 访问所有页表
- 无需显式映射每个页表

## 技术细节

### Init 进程内存布局
```
Entry:    0x210248 (_start)
Code:     0x210000-0x211000 (4KB, R-X)
Data:     0x200000-0x201000 (4KB, R--)
Stack:    0x211000-0x221000 (64KB, RW)
L1 PT:    0x40397000
L2 PT:    0x403B8000
L3 PT:    0x403BA000
ASID:     1
```

### 内核页表布局
```
L1_TABLE   (TTBR1_EL1): 0x40081000 (物理地址)
L1_TABLE0  (TTBR0_EL1): 0x40082000 (boot用)
L2_TABLE0              : 0x40083000 (TTBR0用)
L2_TABLE1              : 0x40084000 (TTBR1用)

L1_TABLE[256] 应指向 L2_TABLE1 (0x40084003)
映射范围: 0xFFFF800000000000 - 0xFFFF8000FFFFFFFF (1GB)
```

### ARM AArch64 地址翻译
```
虚拟地址 0xFFFF800000000A00:
[63:39] = 0x1FF00 >> 9 = 256  -> L1_TABLE[256] -> L2_TABLE1
[38:21] = 0x00000 >> 9 = 0    -> L2_TABLE1[0]  -> 物理 0x40000000 (2MB block)
[20:0]  = 0x00A00             -> 偏移

物理地址 = 0x40000000 + 0x00A00 = 0x40000A00
```

### QEMU 异常输出分析
```
Taking exception 3 [Prefetch Abort]
...from EL1 to EL1
...with ELR 0xffff800000000a00    <- 异常向量表地址
...with FAR 0xffff800000000a00    <- 访问失败地址
```

说明: CPU 试图取指令 0xFFFF800000000A00，但页表翻译失败（L1_TABLE[256]=0）

## 关键文件

- MMU boot: `src/kernel/src/arch/aarch64/mmu/mmu_boot.rs`
- Boot assembly: `src/kernel/src/arch/aarch64/boot.S`
- Linker script: `src/kernel/src/scripts/link.ld.S`
- Physical memory: `src/kernel/src/memory/physical/mod.rs`
- EL0 exec: `src/kernel/src/arch/aarch64/boot.S` (arch_do_exec)

## 测试命令

```bash
# 构建
make kernel

# 运行
timeout 20 qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
    -kernel build/kernel/debug/hnx-kernel-0.1.0-alpha.1.bin \
    -device loader,file=build/initrd.cpio,addr=0x42000000 \
    -nographic -serial mon:stdio -no-reboot

# LLDB 调试
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
    -kernel build/kernel/debug/hnx-kernel-0.1.0-alpha.1.bin \
    -device loader,file=build/initrd.cpio,addr=0x42000000 \
    -nographic -serial mon:stdio -no-reboot -s -S

# 另一终端
lldb target/aarch64-unknown-none/debug/hnx-kernel
(lldb) gdb-remote localhost:1234
(lldb) b arch_do_exec
(lldb) c
(lldb) x/gx 0x40081800  # L1_TABLE[256]
```

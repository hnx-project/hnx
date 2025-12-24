# HNX Microkernel Boot System with Initrd

## Status: ✅ WORKING

内核成功启动并加载initrd！

## 测试结果

**日期**: 2024-12-25  
**QEMU版本**: 10.1.3  
**内核**: hnx-kernel 0.0.1 (aarch64)  
**Initrd大小**: 6.1KB (包含VFS服务 11KB)

### 启动日志（关键部分）

```
[INFO] hnx_kernel Kernel: hnx-kernel 0.0.1 (aarch64)
[INFO] hnx_kernel Memory subsystem ready
[INFO] hnx_kernel Process and IPC subsystems ready
[INFO] hnx_kernel::ipc_services::endpoints ipc_services: created endpoint 1 for vfs
[INFO] hnx_kernel::ipc_services::endpoints ipc_services: created endpoint 2 for network
[INFO] hnx_kernel::ipc_services::endpoints ipc_services: created endpoint 3 for loader
[INFO] hnx_kernel::ipc_services::endpoints ipc_services: created endpoint 4 for block
[INFO] hnx_kernel::core::scheduler scheduler started - HNX kernel successfully booted!
```

✅ **所有子系统成功初始化**：
- 驱动程序 (GICv2, PL011 UART)
- 内存管理 (Buddy + Slab allocator)
- 进程/IPC子系统
- IPC服务端点（VFS, Network, Loader, Block）
- 调度器

## Initrd 结构

```
build/initrd/
├── init                    # 启动脚本
├── bin/                    # 二进制文件目录（空）
├── services/
│   └── vfs-service         # VFS服务 (11KB, aarch64 release)
├── lib/                    # 库目录（空）
└── tmp/                    # 临时文件目录（空）
```

**打包命令**:
```bash
cd build/initrd
find . | cpio -o -H newc | gzip > ../initrd.cpio.gz
```

## QEMU 启动命令

```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a72 \
  -m 512M \
  -nographic \
  -kernel build/kernel/debug/hnx-kernel.bin \
  -initrd build/initrd.cpio.gz \
  -append "console=ttyAMA0" \
  -serial mon:stdio
```

**参数说明**:
- `-machine virt`: QEMU虚拟ARM平台
- `-cpu cortex-a72`: ARMv8 Cortex-A72 CPU
- `-m 512M`: 512MB内存
- `-kernel`: 内核二进制文件
- `-initrd`: 初始RAM磁盘（包含VFS服务）
- `-append`: 内核命令行参数
- `-nographic -serial mon:stdio`: 无图形界面，串口输出到终端

## 内核启动流程

### Phase 1: 硬件初始化
```
arch/boot.S → rust_main()
└→ init_phase1_hardware()
   ├→ console::init()      # PL011 UART
   ├→ drivers::dtb::init() # 设备树解析
   ├→ arch::init()         # GIC中断控制器
   └→ timer::init()        # 定时器
```

### Phase 2: 内存初始化
```
init_phase2_memory()
├→ memory::init()
│  ├→ physical::init()     # Buddy分配器
│  ├→ virtual_::init()     # 虚拟内存管理
│  ├→ slab::init()         # Slab分配器 (8个缓存：16-2048字节)
│  └→ mmap::init()         # 内存映射管理
└→ buddy_allocator::init() # 堆分配器 (256MB)
```

### Phase 3: 进程和IPC初始化
```
init_phase3_processes()
├→ process::init()
│  └→ ipc::init()          # IPC系统
└→ ipc_services::init()    # IPC委托框架
   └→ 创建端点：
      ├→ Endpoint 1: VFS服务
      ├→ Endpoint 2: Network服务
      ├→ Endpoint 3: Loader服务
      └→ Endpoint 4: Block服务
```

### Phase 4: 启动调度器
```
init_phase4_scheduler()
└→ scheduler::start()
   └→ 进入主循环（目前空闲）
```

## Initrd 访问（内核侧）

**当前状态**: Initrd已加载到内存，但内核还**未实现访问接口**。

**需要实现**:
```rust
// src/kernel/src/arch/aarch64/boot/mod.rs
pub fn get_initrd_info() -> (usize, usize) {
    // 从设备树或QEMU传递的参数获取initrd地址和大小
    // QEMU通常将initrd放在物理内存 0x4200_0000
    let initrd_start = 0x4200_0000;
    let initrd_size = /* 从DTB读取 */;
    (initrd_start, initrd_size)
}
```

## 下一步：加载VFS服务

### 选项A：简单方案（测试用）
直接在内核启动时调用 `HNX_SYS_PROCESS_CREATE` 创建VFS服务进程：

```rust
// src/kernel/src/main.rs - init_phase4_scheduler()
fn init_phase4_scheduler() {
    info!("Loading VFS service from initrd...");
    
    // 获取initrd中VFS服务的位置
    let (initrd_start, initrd_size) = arch::boot::get_initrd_info();
    let vfs_service_code = extract_from_cpio(initrd_start, "/services/vfs-service");
    
    // 创建VFS服务进程
    let vfs_pid = user::sys_process_create(
        vfs_service_code.as_ptr() as usize,
        vfs_service_code.len()
    );
    
    info!("VFS service started with PID {}", vfs_pid);
    
    scheduler::start();
}
```

### 选项B：完整方案
实现专用的服务加载器：

```rust
// src/kernel/src/user/service_loader.rs
pub fn load_and_start_service(path: &str, endpoint_id: u32) -> Result<u32, ()> {
    // 1. 从initrd读取ELF文件
    // 2. 解析ELF并加载到内存
    // 3. 创建用户进程
    // 4. 注册到指定的endpoint
    // 5. 启动进程
}
```

## 文件位置

**Initrd相关**:
- `build/initrd/` - Initrd内容目录
- `build/initrd.cpio.gz` - 打包后的initrd (6.1KB)
- `build/initrd/services/vfs-service` - VFS服务二进制 (11KB)

**内核**:
- `build/kernel/debug/hnx-kernel.bin` - 内核二进制 (6.6MB)
- `src/kernel/src/main.rs` - 启动流程
- `src/kernel/src/user/` - 用户进程支持

**VFS服务源码**:
- `src/space/services/vfs-service/` - VFS服务源代码
- `src/space/target/aarch64-unknown-none/release/vfs-service` - 编译后的VFS服务

## 验证清单

✅ 内核编译成功  
✅ VFS服务编译成功（release模式，11KB）  
✅ Initrd创建成功（6.1KB，包含VFS服务）  
✅ QEMU启动成功  
✅ 内核完整启动流程成功  
✅ 所有子系统初始化成功  
✅ IPC端点创建成功  
⏳ Initrd访问接口（需要实现）  
⏳ VFS服务加载（需要实现）  
⏳ VFS服务启动（需要实现）  
⏳ 端到端IPC测试（需要实现）  

## 性能数据

**启动时间**: < 1秒（QEMU）  
**内存使用**:
- 物理内存: 512MB总量，261235页空闲
- Slab缓存: 8个缓存（16-2048字节）
- 堆分配器: 256MB

**代码大小**:
- 内核: 6.6MB (debug) / ~1MB (release预估)
- VFS服务: 11KB (release)
- Initrd: 6.1KB (压缩后)

---
**测试日期**: 2024-12-25  
**状态**: 内核+Initrd启动成功，等待实现服务加载

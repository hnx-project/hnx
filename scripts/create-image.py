#!/usr/bin/env python3
"""
HNX 系统镜像创建工具
创建可启动的系统镜像，包含内核和用户空间程序
"""

import argparse
import subprocess
import os
import sys
import time
import shutil
import tempfile
from pathlib import Path
import json
import struct

class ImageBuilder:
    """系统镜像构建器"""
    
    def __init__(self, kernel_path, space_dir, output_path, arch="aarch64", board="qemu-virt", 
                 simple_initrd=False, no_compress=False):
        """
        初始化镜像构建器
        
        Args:
            kernel_path: 内核二进制文件路径
            space_dir: 用户空间程序目录
            output_path: 输出镜像路径
            arch: 目标架构
            board: 开发板类型
            simple_initrd: 创建简单的 init-only initrd（仅包含 init 二进制）
            no_compress: 不压缩 initrd（使用 .cpio 而不是 .cpio.gz）
        """
        self.kernel_path = Path(kernel_path).resolve()
        self.space_dir = Path(space_dir).resolve()
        self.output_path = Path(output_path).resolve()
        self.arch = arch
        self.board = board
        self.simple_initrd = simple_initrd
        self.no_compress = no_compress
        
        # 验证输入文件
        if not self.kernel_path.exists():
            raise FileNotFoundError(f"Kernel not found: {self.kernel_path}")
        
        if not self.space_dir.exists():
            raise FileNotFoundError(f"Space directory not found: {self.space_dir}")
        
        # 创建输出目录
        self.output_path.parent.mkdir(parents=True, exist_ok=True)
        
        # 临时目录
        self.temp_dir = Path(tempfile.mkdtemp(prefix="hnx_image_"))
        print(f"Using temporary directory: {self.temp_dir}")
    
    def __del__(self):
        """清理临时目录"""
        if hasattr(self, 'temp_dir') and self.temp_dir.exists():
            shutil.rmtree(self.temp_dir)
            print(f"Cleaned up temporary directory: {self.temp_dir}")
    
    def create_initrd(self):
        """创建 initrd（初始 RAM 磁盘）"""
        print("Creating initrd...")
        
        if self.simple_initrd:
            return self._create_simple_initrd()
        else:
            return self._create_full_initrd()
    
    def _create_simple_initrd(self):
        """创建简单的 init-only initrd（仅包含 init 二进制文件）"""
        print("Creating simple init-only initrd...")
        
        # 创建 initrd 目录
        initrd_root = self.temp_dir / "initrd"
        initrd_root.mkdir(parents=True, exist_ok=True)
        print(f"  Created initrd root directory, {initrd_root}")
        
        # 查找并复制 init 二进制文件
        init_found = self._copy_init_binary(initrd_root)
        
        if not init_found:
            raise FileNotFoundError("No init binary found in space directory for simple initrd")
        
        # 打包为 cpio 归档
        if self.no_compress:
            initrd_path = self.temp_dir / "initrd.cpio"
        else:
            initrd_path = self.temp_dir / "initrd.cpio.gz"
        
        self._pack_cpio(initrd_root, initrd_path, compress=not self.no_compress)
        
        print(f"Simple initrd created: {initrd_path}")
        return initrd_path
    
    def _create_full_initrd(self):
        """创建完整的 initrd（包含目录结构、设备节点等）"""
        print("Creating full initrd...")
        
        # 创建 initrd 目录结构
        initrd_root = self.temp_dir / "initrd"
        initrd_root.mkdir(parents=True, exist_ok=True)
        
        # 创建标准目录结构
        dirs = ["bin", "sbin", "lib", "dev", "proc", "sys", "tmp", "root", "etc"]
        for dir_name in dirs:
            (initrd_root / dir_name).mkdir(exist_ok=True)
        
        # 查找用户空间程序
        self._copy_space_programs(initrd_root)
        
        # 创建 init 程序（如果不存在则创建简单版本）
        self._create_init_program(initrd_root)
        
        # 创建设备节点
        self._create_device_nodes(initrd_root)
        
        # 创建基本的配置文件
        self._create_config_files(initrd_root)
        
        # 打包为 cpio 归档
        if self.no_compress:
            initrd_path = self.temp_dir / "initrd.cpio"
        else:
            initrd_path = self.temp_dir / "initrd.cpio.gz"
        
        self._pack_cpio(initrd_root, initrd_path, compress=not self.no_compress)
        
        print(f"Full initrd created: {initrd_path}")
        return initrd_path
    
    def _copy_init_binary(self, initrd_root):
        """复制 init 二进制文件到 initrd（用于 simple_initrd 模式）"""
        print("Looking for init binary...")
        
        # 查找 init 二进制文件
        init_binary = None
        
        # 优先查找 release 版本
        for build_type in ["release", "debug"]:
            init_path = self.space_dir / build_type / "init"
            if init_path.exists() and init_path.is_file():
                init_binary = init_path
                print(f"  Found init binary: {init_path} ({build_type})")
                break
        
        # 如果没有找到，尝试直接在 space_dir 查找
        if not init_binary:
            init_path = self.space_dir / "init"
            if init_path.exists() and init_path.is_file():
                init_binary = init_path
                print(f"  Found init binary: {init_path}")
        
        if not init_binary:
            print("  Warning: No init binary found in space directory")
            return False
        
        # 复制到 initrd 根目录
        dest_path = initrd_root / "init"
        try:
            shutil.copy2(init_binary, dest_path)
            dest_path.chmod(0o755)
            size = dest_path.stat().st_size
            print(f"  Copied init binary: {size:,} bytes")
            return True
        except Exception as e:
            print(f"  Warning: Failed to copy init binary: {e}")
            return False
    
    def _copy_space_programs(self, initrd_root):
        """复制用户空间程序到 initrd"""
        print("Copying user space programs...")
        
        # 查找可执行文件
        executables = []
        for pattern in ["*", "bin/*", "sbin/*", "usr/bin/*"]:
            for path in self.space_dir.glob(pattern):
                if path.is_file() and os.access(path, os.X_OK):
                    executables.append(path)
        
        if not executables:
            print("Warning: No executables found in space directory")
            return
        
        # 复制到 initrd 的 bin 目录
        for exe in executables:
            dest_path = initrd_root / "bin" / exe.name
            
            # 如果是 init 程序，放在根目录
            if exe.name in ["init", "hnx-init", "userboot"]:
                dest_path = initrd_root / "init"
            
            try:
                shutil.copy2(exe, dest_path)
                # 确保可执行权限
                dest_path.chmod(0o755)
                print(f"  Copied: {exe.name} -> {dest_path.relative_to(initrd_root)}")
            except Exception as e:
                print(f"  Warning: Failed to copy {exe.name}: {e}")
    
    def _create_init_program(self, initrd_root):
        """创建 init 程序"""
        init_path = initrd_root / "init"
        
        # 如果已经存在 init 程序，跳过
        if init_path.exists():
            return
        
        print("Creating simple init program...")
        
        # 创建简单的 shell 脚本作为 init
        init_script = """#!/bin/sh
# HNX Simple Init Script
echo "========================================"
echo "    HNX Microkernel System"
echo "========================================"
echo "Kernel: $(uname -s) $(uname -r)"
echo "Init: Starting userspace..."

# 挂载虚拟文件系统
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
mount -t devtmpfs devtmpfs /dev 2>/dev/null

# 设置环境变量
export PATH=/bin:/sbin
export TERM=linux
export HOME=/root

# 创建设备节点（备用）
if [ ! -c /dev/console ]; then
    mknod /dev/console c 5 1 2>/dev/null
fi
if [ ! -c /dev/null ]; then
    mknod /dev/null c 1 3 2>/dev/null
fi

echo "System ready!"

# 如果存在 /bin/sh，启动 shell
if [ -x /bin/sh ]; then
    echo "Starting shell..."
    exec /bin/sh
else
    echo "No shell found. Running test programs..."
    
    # 运行测试程序
    for test in /bin/*; do
        if [ -x "$test" ] && [ "$test" != "/bin/sh" ]; then
            echo "Running: $test"
            $test 2>&1 || true
        fi
    done
    
    echo "All programs executed. System will halt."
    poweroff -f
fi
"""
        
        init_path.write_text(init_script)
        init_path.chmod(0o755)
    
    def _create_device_nodes(self, initrd_root):
        """创建设备节点"""
        dev_dir = initrd_root / "dev"
        dev_dir.mkdir(exist_ok=True)
        
        # 尝试创建设备节点
        try:
            # console
            subprocess.run(["mknod", str(dev_dir / "console"), "c", "5", "1"], 
                         capture_output=True)
            # null
            subprocess.run(["mknod", str(dev_dir / "null"), "c", "1", "3"], 
                         capture_output=True)
            # zero
            subprocess.run(["mknod", str(dev_dir / "zero"), "c", "1", "5"], 
                         capture_output=True)
            # random
            subprocess.run(["mknod", str(dev_dir / "random"), "c", "1", "8"], 
                         capture_output=True)
            # urandom
            subprocess.run(["mknod", str(dev_dir / "urandom"), "c", "1", "9"], 
                         capture_output=True)
            
            print("Created device nodes")
        except Exception as e:
            print(f"Warning: Could not create device nodes: {e}")
    
    def _create_config_files(self, initrd_root):
        """创建配置文件"""
        etc_dir = initrd_root / "etc"
        etc_dir.mkdir(exist_ok=True)
        
        # 创建主机名文件
        (etc_dir / "hostname").write_text("hnx-system\n")
        
        # 创建 hosts 文件
        hosts_content = """127.0.0.1   localhost localhost.localdomain
::1         localhost localhost.localdomain
"""
        (etc_dir / "hosts").write_text(hosts_content)
        
        # 创建 fstab
        fstab_content = """# <file system> <mount point>   <type>  <options>       <dump>  <pass>
proc            /proc           proc    defaults        0       0
sysfs           /sys            sysfs   defaults        0       0
devtmpfs        /dev            devtmpfs defaults       0       0
"""
        (etc_dir / "fstab").write_text(fstab_content)
    
    def _pack_cpio(self, initrd_root, output_path, compress=True):
        """打包为 cpio 归档"""
        print("Packing initrd...")
        
        # 进入 initrd 目录并打包
        if compress:
            cmd = f"cd {initrd_root} && find . -print | cpio -o -H newc | gzip -9 > {output_path}"
            format_desc = "newc format, gzip compressed"
        else:
            cmd = f"cd {initrd_root} && find . -print | cpio -o -H newc > {output_path}"
            format_desc = "newc format, uncompressed"
        
        try:
            result = subprocess.run(cmd, shell=True, check=True, capture_output=True)
            size = output_path.stat().st_size
            print(f"  Packed initrd: {size:,} bytes ({format_desc})")
        except subprocess.CalledProcessError as e:
            print(f"Error packing initrd: {e}")
            print(f"stdout: {e.stdout.decode()}")
            print(f"stderr: {e.stderr.decode()}")
            raise
    
    def create_boot_header(self, kernel_size, initrd_size):
        """创建引导头（可选）"""
        # 简单的引导头结构
        # 魔数 + 内核大小 + initrd偏移 + 命令行
        header_format = "<8sIII256s"
        magic = b"HNXBOOT\x00"
        cmdline = b"console=ttyAMA0 root=/dev/ram0 init=/init quiet\x00"
        
        # 计算偏移量
        header_size = struct.calcsize(header_format)
        kernel_offset = header_size
        initrd_offset = kernel_offset + kernel_size
        
        header = struct.pack(header_format,
                           magic,
                           kernel_size,
                           initrd_offset,
                           len(cmdline),
                           cmdline)
        
        return header
    
    def create_raw_image(self, initrd_path):
        """创建原始镜像（内核 + initrd）"""
        print("Creating raw system image...")
        
        with open(self.output_path, 'wb') as out_img:
            # 读取内核
            with open(self.kernel_path, 'rb') as kernel_file:
                kernel_data = kernel_file.read()
            
            # 读取 initrd
            with open(initrd_path, 'rb') as initrd_file:
                initrd_data = initrd_file.read()
            
            # 创建引导头
            header = self.create_boot_header(len(kernel_data), len(initrd_data))
            
            # 写入镜像
            out_img.write(header)
            out_img.write(kernel_data)
            
            # 对齐到 4K 边界（可选）
            current_pos = out_img.tell()
            align_to = 4096
            padding = (align_to - (current_pos % align_to)) % align_to
            if padding > 0:
                out_img.write(b'\x00' * padding)
                print(f"  Added {padding} bytes padding after kernel")
            
            out_img.write(initrd_data)
            
            # 最终文件大小
            final_size = out_img.tell()
            print(f"  Kernel size: {len(kernel_data):,} bytes")
            print(f"  Initrd size: {len(initrd_data):,} bytes")
            print(f"  Total size: {final_size:,} bytes ({final_size/1024/1024:.2f} MB)")
    
    def create_qcow2_image(self, raw_image_path):
        """转换为 QCOW2 格式（可选）"""
        qcow2_path = self.output_path.with_suffix('.qcow2')
        print(f"Creating QCOW2 image: {qcow2_path}")
        
        cmd = [
            "qemu-img", "convert",
            "-f", "raw",
            "-O", "qcow2",
            str(raw_image_path),
            str(qcow2_path)
        ]
        
        try:
            subprocess.run(cmd, check=True, capture_output=True)
            print(f"QCOW2 image created: {qcow2_path}")
            return qcow2_path
        except subprocess.CalledProcessError as e:
            print(f"Warning: Failed to create QCOW2 image: {e}")
            return None
    
    def build(self, create_qcow2=False):
        """构建系统镜像"""
        print(f"Building HNX system image:")
        print(f"  Architecture: {self.arch}")
        print(f"  Board: {self.board}")
        print(f"  Kernel: {self.kernel_path}")
        print(f"  Space dir: {self.space_dir}")
        print(f"  Output: {self.output_path}")
        print("-" * 50)
        
        start_time = time.time()
        
        try:
            # 创建 initrd
            initrd_path = self.create_initrd()
            
            # 复制 initrd.cpio 到 build/ 目录（供 run-qemu.py 使用）
            build_initrd = Path("build") / "initrd.cpio"
            if initrd_path.suffix == ".cpio":
                print(f"Copying initrd to {build_initrd} for QEMU...")
                shutil.copy2(initrd_path, build_initrd)
            
            # 创建原始镜像
            self.create_raw_image(initrd_path)
            
            # 可选：创建 QCOW2 格式
            if create_qcow2:
                self.create_qcow2_image(self.output_path)
            
            elapsed = time.time() - start_time
            print(f"\n✓ System image created successfully in {elapsed:.2f} seconds")
            print(f"  Output: {self.output_path}")
            
            # 显示镜像信息
            if self.output_path.exists():
                size = self.output_path.stat().st_size
                print(f"  Size: {size:,} bytes ({size/1024/1024:.2f} MB)")
            
            return True
            
        except Exception as e:
            print(f"\n✗ Failed to create system image: {e}")
            import traceback
            traceback.print_exc()
            return False

def main():
    parser = argparse.ArgumentParser(
        description="HNX System Image Builder",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s --kernel build/kernel/hnx-kernel.bin --space-dir build/space --output hnx.img
  %(prog)s --kernel kernel.bin --space-dir programs --arch aarch64 --board qemu-virt --qcow2
  %(prog)s --kernel kernel.bin --space-dir programs --simple-initrd --no-compress
        """
    )
    
    parser.add_argument("--kernel", required=True,
                       help="Path to kernel binary")
    parser.add_argument("--space-dir", required=True,
                       help="Directory containing user space programs")
    parser.add_argument("--output", required=True,
                       help="Output image path")
    parser.add_argument("--arch", default="aarch64",
                       choices=["aarch64", "x86_64", "riscv64"],
                       help="Target architecture")
    parser.add_argument("--board", default="qemu-virt",
                       help="Board type (qemu-virt, raspberry-pi4, etc.)")
    parser.add_argument("--qcow2", action="store_true",
                       help="Also create QCOW2 format image")
    parser.add_argument("--simple-initrd", action="store_true",
                       help="Create simple init-only initrd (no directories/scripts)")
    parser.add_argument("--no-compress", action="store_true",
                       help="Do not compress initrd (use .cpio instead of .cpio.gz)")
    parser.add_argument("--verbose", "-v", action="count", default=0,
                       help="Increase verbosity level")

    
    args = parser.parse_args()
    
    # 设置日志级别
    if args.verbose == 0:
        # 静默运行子进程
        pass
    
    try:
        builder = ImageBuilder(
            kernel_path=args.kernel,
            space_dir=args.space_dir,
            output_path=args.output,
            arch=args.arch,
            board=args.board,
            simple_initrd=args.simple_initrd,
            no_compress=args.no_compress
        )
        
        success = builder.build(create_qcow2=args.qcow2)
        
        if not success:
            sys.exit(1)
            
    except KeyboardInterrupt:
        print("\nInterrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
# scripts/run-qemu.py
#!/usr/bin/env python3
"""
HNX QEMU 运行工具 - 使用 TOML 配置文件
"""

import argparse
import subprocess
import sys
import os
import time
import threading
from pathlib import Path
import tempfile
import atexit
import json
import tomllib

class QEMURunner:
    """QEMU 运行管理器 - 使用 TOML 配置"""
    
    def __init__(self, image_path, arch="aarch64", board="qemu-virt", 
                 config_dir=None, timeout=60, headless=False,
                 gdb=False, monitor=False):
        """
        初始化 QEMU 运行器
        
        Args:
            image_path: 系统镜像路径
            arch: 目标架构
            board: 开发板类型
            config_dir: 配置目录（包含 config.json）
            timeout: 超时时间（秒）
            headless: 无图形界面模式
            gdb: 启用 GDB 调试
            monitor: 启用 QEMU 监控
        """
        self.image_path = Path(image_path).resolve()
        self.arch = arch
        self.board = board
        self.timeout = timeout
        self.headless = headless
        self.gdb = gdb
        self.monitor = monitor
        self.build_dir = Path(os.environ.get("MESON_BUILD_ROOT", "build"))
        
        # 加载配置
        if config_dir:
            self.config_dir = Path(config_dir)
            self.config = self._load_config()
        else:
            # 尝试自动查找配置
            self.config_dir = self._find_config_dir()
            self.config = self._load_config() if self.config_dir else {}
        
        # QEMU 进程
        self.qemu_process = None
        self.timeout_timer = None
        self.killed_by_timeout = False
        
        # 输出文件
        self.temp_dir = Path(tempfile.mkdtemp(prefix="hnx_qemu_"))
        self.stdout_file = self.temp_dir / "qemu_stdout.log"
        self.stderr_file = self.temp_dir / "qemu_stderr.log"
        
        # 验证镜像文件
        if not self.image_path.exists():
            raise FileNotFoundError(f"Image not found: {self.image_path}")
        
        print(f"QEMU Runner initialized:")
        print(f"  Image: {self.image_path}")
        print(f"  Architecture: {arch}")
        print(f"  Board: {board}")
        print(f"  Config: {self.config_dir}")
        print(f"  Timeout: {timeout}s" if timeout > 0 else "  Timeout: disabled")
        print(f"  Headless: {headless}")
        print(f"  GDB: {gdb}")
        print(f"  Monitor: {monitor}")
        print(f"  Temp dir: {self.temp_dir}")
        print(f"  qemu_stdout file: {self.stdout_file}")
        print(f"  qemu_stderr file: {self.stderr_file}")
        
        # 注册退出清理函数
        atexit.register(self.cleanup)
    
    def _find_config_dir(self):
        """自动查找配置目录"""
        # 尝试几个可能的配置目录
        candidates = [
            self.build_dir / "config",
            self.build_dir / f"config-{self.arch}-{self.board}",
            Path(".") / "config",
        ]
        
        for candidate in candidates:
            config_file = candidate / "config.json"
            if config_file.exists():
                return candidate
        
        print(f"Warning: No configuration directory found for {self.arch}/{self.board}")
        return None
    
    def _load_config(self):
        """加载配置"""
        config_file = self.config_dir / "config.json"
        if not config_file.exists():
            return {}
        
        with open(config_file, 'r') as f:
            return json.load(f)
    
    def _get_qemu_config(self):
        """获取 QEMU 配置"""
        qemu_config = self.config.get("qemu", {})
        
        # 默认值
        defaults = {
            "machine": "virt",
            "cpu": "cortex-a72" if self.arch == "aarch64" else "qemu64",
            "memory": "512M",
            "devices": [],
            "kernel_args": ""
        }
        
        # 合并配置
        for key, value in defaults.items():
            if key not in qemu_config:
                qemu_config[key] = value
        
        return qemu_config
    
    def cleanup(self):
        """清理资源"""
        if self.temp_dir.exists():
            print(f"\nQEMU logs saved in: {self.temp_dir}")
    
    def build_qemu_command(self):
        """构建 QEMU 命令行参数"""
        cmd = ["qemu-system-" + self.arch]
        
        # 获取 QEMU 配置
        qemu_config = self._get_qemu_config()
        
        # 机器类型和 CPU
        machine = qemu_config["machine"]
        if isinstance(machine, dict):
            machine_name = machine.get("name", "virt")
        else:
            machine_name = str(machine)
        cmd.extend(["-machine", machine_name])
        cmd.extend(["-cpu", qemu_config["cpu"]])
        
        # 内存
        cmd.extend(["-m", qemu_config["memory"]])
        
        # 镜像文件处理
        # 使用 -kernel 加载内核镜像
        cmd.extend(["-kernel", str(self.image_path)])

        # 添加 DTB 文件（如果配置中指定）
        if "dtb" in qemu_config:
            dtb_path = Path(qemu_config["dtb"])
            if dtb_path.exists():
                cmd.extend(["-dtb", str(dtb_path)])
                print(f"  Using DTB: {dtb_path}")
            else:
                print(f"  Warning: DTB file not found: {dtb_path}")

        # 检查是否有 initrd.cpio 文件
        # 优先在 image 同目录查找，然后在 build/ 目录查找
        initrd_candidates = [
            self.image_path.parent / "initrd.cpio",
            self.build_dir / "initrd.cpio",
        ]
        initrd_path = None
        for candidate in initrd_candidates:
            if candidate.exists():
                initrd_path = candidate
                break

        if initrd_path:
            # 使用 -device loader 加载 initrd 到固定地址 0x42000000
            cmd.extend(["-device", f"loader,file={initrd_path},addr=0x42000000"])
            print(f"  Using initrd: {initrd_path} (loaded at 0x42000000)")
        
        
        # 网络配置
        if self.board == "qemu-virt":
            cmd.extend([
                "-netdev", "user,id=net0,hostfwd=tcp::2222-:22",
                "-device", "virtio-net-device,netdev=net0"
            ])
        
        # 串口和控制台
        if self.headless:
            cmd.extend([
                "-nographic",
                "-serial", "mon:stdio",
            ])
        else:
            # 图形模式
            if self.arch == "aarch64":
                cmd.extend([
                    "-device", "virtio-gpu-pci",
                    "-device", "virtio-keyboard-pci",
                    "-device", "virtio-mouse-pci",
                ])
            cmd.extend(["-serial", "stdio"])
        
        # 内核参数
        if qemu_config.get("kernel_args"):
            cmd.extend(["-append", qemu_config["kernel_args"]])
        
        # 调试选项
        if self.gdb:
            cmd.extend(["-s", "-S"])
            print("GDB server enabled on port 1234")
        
        # QEMU 监控
        if self.monitor:
            cmd.extend(["-monitor", "telnet:127.0.0.1:55555,server,nowait"])
            print("QEMU monitor enabled on telnet://127.0.0.1:55555")
        
        # 其他选项
        cmd.extend(["-no-reboot"])
        
        return cmd

    def timeout_handler(self):
        """超时处理函数"""
        if self.qemu_process and self.qemu_process.poll() is None:
            print(f"\n⚠️  Timeout reached ({self.timeout}s). Terminating QEMU...")
            self.killed_by_timeout = True
            self.qemu_process.terminate()
            
            # 等待进程结束
            try:
                self.qemu_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                print("QEMU did not terminate gracefully, forcing kill...")
                self.qemu_process.kill()
    
    def run(self):
        """运行 QEMU"""
        # 构建命令
        qemu_cmd = self.build_qemu_command()
        
        print("\n" + "=" * 60)
        print("Starting QEMU with command:")
        print("  " + " ".join(qemu_cmd))
        print("=" * 60 + "\n")
        
        # 打开输出文件
        stdout_fd = open(self.stdout_file, "wb")
        stderr_fd = open(self.stderr_file, "wb")
        
        try:
            # 启动 QEMU 进程
            self.qemu_process = subprocess.Popen(
                qemu_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                stdin=subprocess.PIPE,
                bufsize=1,
                universal_newlines=False
            )
            
            print(f"QEMU started with PID: {self.qemu_process.pid}")
            print("Press Ctrl+C to stop QEMU")
            print("-" * 40)
            
            # 设置超时定时器
            if self.timeout > 0:
                self.timeout_timer = threading.Timer(self.timeout, self.timeout_handler)
                self.timeout_timer.start()
                print(f"Timeout timer set for {self.timeout} seconds")
            
            # 创建输出处理线程
            def read_stdout():
                for line in iter(self.qemu_process.stdout.readline, b''):
                    sys.stdout.buffer.write(line)
                    sys.stdout.flush()
                    stdout_fd.write(line)
                    stdout_fd.flush()
            
            def read_stderr():
                for line in iter(self.qemu_process.stderr.readline, b''):
                    sys.stderr.buffer.write(line)
                    sys.stderr.flush()
                    stderr_fd.write(line)
                    stderr_fd.flush()
            
            # 启动输出线程
            stdout_thread = threading.Thread(target=read_stdout, daemon=True)
            stderr_thread = threading.Thread(target=read_stderr, daemon=True)
            stdout_thread.start()
            stderr_thread.start()
            
            # 等待进程结束
            return_code = self.qemu_process.wait()
            
            # 取消超时定时器
            if self.timeout_timer:
                self.timeout_timer.cancel()
            
            # 等待输出线程结束
            stdout_thread.join(timeout=1)
            stderr_thread.join(timeout=1)
            
            # 关闭文件
            stdout_fd.close()
            stderr_fd.close()
            
            print("\n" + "-" * 40)
            
            if self.killed_by_timeout:
                print("⏰ QEMU terminated due to timeout")
                return 0
            elif return_code == 0:
                print("✅ QEMU exited successfully")
            elif return_code < 0:
                # 被信号终止
                sig = -return_code
                print(f"⚠️  QEMU terminated by signal {sig}")
            else:
                print(f"⚠️  QEMU exited with code {return_code}")
            
            # 显示日志文件位置
            print(f"\nLog files:")
            print(f"  stdout: {self.stdout_file}")
            print(f"  stderr: {self.stderr_file}")
            
            return return_code if not self.killed_by_timeout else 0
            
        except KeyboardInterrupt:
            print("\n\n🛑 Received interrupt signal, terminating QEMU...")
            if self.qemu_process:
                self.qemu_process.terminate()
                try:
                    self.qemu_process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    self.qemu_process.kill()
            return 130  # SIGINT exit code
        
        except Exception as e:
            print(f"\n❌ Error running QEMU: {e}")
            import traceback
            traceback.print_exc()
            return 1
        
        finally:
            # 确保文件关闭
            if not stdout_fd.closed:
                stdout_fd.close()
            if not stderr_fd.closed:
                stderr_fd.close()
            
            # 取消定时器
            if self.timeout_timer:
                self.timeout_timer.cancel()
def main():
    parser = argparse.ArgumentParser(
        description="HNX QEMU Runner - Using TOML configuration",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s hnx.img
  %(prog)s --arch aarch64 --board raspberry-pi4 --config-dir build/config hnx.img
  %(prog)s --gdb --timeout 30 hnx.img
  
Configuration files should be in configs/board/<board>.toml
        """
    )
    
    parser.add_argument("image", help="System image file")
    parser.add_argument("--arch", default="aarch64", 
                       choices=["aarch64", "x86_64", "riscv64"])
    parser.add_argument("--board", default="qemu-virt", help="Board name")
    parser.add_argument("--config-dir", help="Configuration directory")
    parser.add_argument("--timeout", type=int, default=60, help="Timeout in seconds")
    parser.add_argument("--headless", action="store_true", help="Headless mode")
    parser.add_argument("--graphics", action="store_true", help="Graphics mode")
    parser.add_argument("--gdb", action="store_true", help="Enable GDB")
    parser.add_argument("--monitor", action="store_true", help="Enable monitor")
    
    args = parser.parse_args()
    
    # 处理 headless/graphics
    if args.graphics:
        args.headless = False
    
    try:
        runner = QEMURunner(
            image_path=args.image,
            arch=args.arch,
            board=args.board,
            config_dir=args.config_dir,
            timeout=args.timeout,
            headless=args.headless,
            gdb=args.gdb,
            monitor=args.monitor
        )
        
        exit_code = runner.run()
        sys.exit(exit_code)
        
    except KeyboardInterrupt:
        print("\nInterrupted by user")
        sys.exit(130)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()

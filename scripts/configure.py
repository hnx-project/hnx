# scripts/configure.py
#!/usr/bin/env python3
import json
# import os
import tomllib
# import tomli
from pathlib import Path
# import sys

def load_board_config(board_name, config_dir="configs"):
    """加载开发板 TOML 配置"""
    board_file = Path(config_dir) / "board" / f"{board_name}.toml"
    
    if not board_file.exists():
        raise FileNotFoundError(f"Board config not found: {board_file}")
    
    with open(board_file, 'rb') as f:
        config = tomllib.load(f)
    
    return config

def generate_qemu_config(board_config, arch, output_dir):
    """从 board 配置生成 QEMU 运行配置"""
    qemu_config = {
        "machine": board_config.get("machine", "virt"),
        "cpu": board_config.get("cpu", "cortex-a72"),
        "devices": board_config.get("devices", []),
        "memory": board_config.get("memory", "512M"),
        "kernel_args": board_config.get("kernel_args", ""),
        "description": board_config.get("description", ""),
    }
    
    # 根据架构调整
    if arch == "x86_64":
        qemu_config["cpu"] = board_config.get("cpu", "qemu64")
    
    # 保存为 JSON（供 Python 脚本使用）
    config_file = Path(output_dir) / "qemu_config.json"
    with open(config_file, 'w') as f:
        json.dump(qemu_config, f, indent=2)
    
    return qemu_config

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="Generate HNX build configuration")
    parser.add_argument("--arch", required=True, choices=["aarch64", "x86_64", "riscv64"])
    parser.add_argument("--board", required=True, help="Board name")
    parser.add_argument("--profile", default="debug", choices=["debug", "release"])
    parser.add_argument("--output-dir", required=True, help="Output directory for config files")
    
    args = parser.parse_args()
    
    # 创建输出目录
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"Generating configuration for:")
    print(f"  Architecture: {args.arch}")
    print(f"  Board: {args.board}")
    print(f"  Profile: {args.profile}")
    print(f"  Output: {output_dir}")
    
    # 1. 加载架构配置
    arch_config = {}
    arch_file = Path("configs/arch") / f"{args.arch}.toml"
    if arch_file.exists():
        with open(arch_file, 'rb') as f:
            arch_config = tomllib.load(f)
    
    # 2. 加载开发板配置
    board_config = load_board_config(args.board)
    
    # 3. 加载构建配置
    profile_config = {}
    profile_file = Path("configs/profile") / f"{args.profile}.toml"
    if profile_file.exists():
        with open(profile_file, 'rb') as f:
            profile_config = tomllib.load(f)
    
    # 4. 合并配置
    merged_config = {
        "arch": arch_config,
        "board": board_config,
        "profile": profile_config,
        "metadata": {
            "arch": args.arch,
            "board": args.board,
            "profile": args.profile,
            "timestamp": time.time()
        }
    }
    
    # 5. 生成 QEMU 运行配置
    qemu_config = generate_qemu_config(board_config, args.arch, output_dir)
    merged_config["qemu"] = qemu_config
    
    # 6. 保存完整配置
    config_file = output_dir / "config.json"
    with open(config_file, 'w') as f:
        json.dump(merged_config, f, indent=2)
    
    # 7. 生成环境文件
    env_file = output_dir / "env.sh"
    with open(env_file, 'w') as f:
        f.write(f"export ARCH={args.arch}\n")
        f.write(f"export BOARD={args.board}\n")
        f.write(f"export PROFILE={args.profile}\n")
        f.write(f"export KERNEL_TARGET={args.arch}-unknown-none\n")
        f.write(f"export SPACE_TARGET={args.arch}-unknown-hnx\n")
        f.write(f"export QEMU_MACHINE={qemu_config['machine']}\n")
        f.write(f"export QEMU_CPU={qemu_config['cpu']}\n")
    
    # 8. 生成 Makefile 片段
    makefile_fragment = output_dir / "Makefile.inc"
    with open(makefile_fragment, 'w') as f:
        f.write(f"# Auto-generated Makefile variables\n")
        f.write(f"QEMU_MACHINE := {qemu_config['machine']}\n")
        f.write(f"QEMU_CPU := {qemu_config['cpu']}\n")
        f.write(f"QEMU_MEMORY := {qemu_config['memory']}\n")
        if qemu_config['kernel_args']:
            f.write(f"QEMU_KERNEL_ARGS := {qemu_config['kernel_args']}\n")
    
    print(f"\nConfiguration files generated:")
    print(f"  {config_file}")
    print(f"  {env_file}")
    print(f"  {makefile_fragment}")
    
    # 显示 QEMU 配置
    print(f"\nQEMU Configuration:")
    print(f"  Machine: {qemu_config['machine']}")
    print(f"  CPU: {qemu_config['cpu']}")
    print(f"  Memory: {qemu_config['memory']}")
    if qemu_config['description']:
        print(f"  Description: {qemu_config['description']}")

if __name__ == "__main__":
    import time
    main()
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

def process_dtb_config(board_config, board_name, output_dir):
    """处理 DTB 配置：复制 DTB 文件到输出目录"""
    dtb_config = board_config.get("dtb", {})

    # 向后兼容：如果 dtb 是布尔值，使用默认文件名
    if isinstance(dtb_config, bool):
        if dtb_config:
            dtb_config = {"filename": f"{board_name}.dtb", "required": True}
        else:
            dtb_config = {"filename": None, "required": False}
    elif isinstance(dtb_config, str):
        # 如果 dtb 是字符串，视为文件名
        dtb_config = {"filename": dtb_config, "required": True}

    # 确保 dtb_config 是字典
    if not isinstance(dtb_config, dict):
        dtb_config = {}

    filename = dtb_config.get("filename")
    required = dtb_config.get("required", False)

    dtb_path = None
    if filename and required:
        # 在 configs/dtbs 目录中查找 DTB 文件
        source_dtb = Path("configs/dtbs") / filename
        if source_dtb.exists():
            # 复制到输出目录
            dest_dtb = output_dir / filename
            import shutil
            shutil.copy2(source_dtb, dest_dtb)
            dtb_path = dest_dtb
            print(f"  DTB file: {filename} (copied from {source_dtb})")
        elif required:
            raise FileNotFoundError(f"Required DTB file not found: {source_dtb}")
        else:
            print(f"  DTB file: {filename} (not found, optional)")

    return dtb_path

def generate_qemu_config(board_config, arch, output_dir, board_name):
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

    # 处理 DTB 配置
    dtb_path = process_dtb_config(board_config, board_name, output_dir)
    if dtb_path:
        qemu_config["dtb"] = str(dtb_path)
        qemu_config["dtb_filename"] = dtb_path.name

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
    qemu_config = generate_qemu_config(board_config, args.arch, output_dir, args.board)
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
        if 'dtb' in qemu_config:
            f.write(f"export QEMU_DTB={qemu_config['dtb']}\n")
            f.write(f"export QEMU_DTB_FILENAME={qemu_config['dtb_filename']}\n")
    
    # 8. 生成 Makefile 片段
    makefile_fragment = output_dir / "Makefile.inc"
    with open(makefile_fragment, 'w') as f:
        f.write(f"# Auto-generated Makefile variables\n")
        f.write(f"QEMU_MACHINE := {qemu_config['machine']}\n")
        f.write(f"QEMU_CPU := {qemu_config['cpu']}\n")
        f.write(f"QEMU_MEMORY := {qemu_config['memory']}\n")
        if qemu_config['kernel_args']:
            f.write(f"QEMU_KERNEL_ARGS := {qemu_config['kernel_args']}\n")
        if 'dtb' in qemu_config:
            f.write(f"QEMU_DTB := {qemu_config['dtb']}\n")
            f.write(f"QEMU_DTB_FILENAME := {qemu_config['dtb_filename']}\n")
    
    print(f"\nConfiguration files generated:")
    print(f"  {config_file}")
    print(f"  {env_file}")
    print(f"  {makefile_fragment}")
    
    # 显示 QEMU 配置
    print(f"\nQEMU Configuration:")
    print(f"  Machine: {qemu_config['machine']}")
    print(f"  CPU: {qemu_config['cpu']}")
    print(f"  Memory: {qemu_config['memory']}")
    if 'dtb' in qemu_config:
        print(f"  DTB: {qemu_config['dtb_filename']} ({qemu_config['dtb']})")
    if qemu_config['description']:
        print(f"  Description: {qemu_config['description']}")

if __name__ == "__main__":
    import time
    main()
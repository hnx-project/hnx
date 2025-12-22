# scripts/configure.py
#!/usr/bin/env python3
import tomllib
import json
import os
from pathlib import Path

def load_config(arch, board, profile):
    """加载配置"""
    config_dir = Path("configs")
    
    # 加载架构配置
    arch_config = {}
    arch_file = config_dir / "arch" / f"{arch}.toml"
    if arch_file.exists():
        with open(arch_file, 'rb') as f:
            arch_config = tomllib.load(f)
    
    # 加载开发板配置
    board_config = {}
    board_file = config_dir / "board" / f"{board}.toml"
    if board_file.exists():
        with open(board_file, 'rb') as f:
            board_config = tomllib.load(f)
    
    # 加载构建配置
    build_config = {}
    profile_file = config_dir / "profile" / f"{profile}.toml"
    if profile_file.exists():
        with open(profile_file, 'rb') as f:
            build_config = tomllib.load(f)
    
    # 合并配置
    config = {**arch_config, **board_config, **build_config}
    
    # 生成 Rust 目标文件
    generate_rust_target(arch, config)
    
    # 生成构建配置
    generate_build_config(config, arch, board, profile)
    
    return config

def generate_rust_target(arch, config):
    """生成 Rust 目标定义文件"""
    target_dir = Path("targets")
    target_dir.mkdir(exist_ok=True)
    
    # 内核目标
    kernel_target = {
        "llvm-target": f"{arch}-unknown-none",
        "data-layout": config.get("data-layout", ""),
        "arch": arch,
        "os": "hnx-kernel",
        "env": "",
        "vendor": "unknown",
        "linker-flavor": "ld.lld",
        "linker": "rust-lld",
        "panic-strategy": "abort",
        "disable-redzone": True,
        "features": ",".join(config.get("features", [])),
        "frame-pointer": "always",
        "stack-protector": False,
        "relocation-model": "static",
        "code-model": "large",
        "target-pointer-width": "64",
        "target-c-int-width": "32",
        "target-endian": "little",
        "target-family": "unix",
        "executables": True,
        "singlethread": True,
        "linker-script": "linker.ld",
    }
    
    # 空间目标
    space_target = kernel_target.copy()
    space_target["os"] = "hnx"
    space_target["relocation-model"] = "pic"
    space_target["position-independent-executables"] = True
    
    # 写入文件
    kernel_file = target_dir / f"{arch}-unknown-hnx-kernel.json"
    space_file = target_dir / f"{arch}-unknown-hnx.json"
    
    with open(kernel_file, 'w') as f:
        json.dump(kernel_target, f, indent=2)
    
    with open(space_file, 'w') as f:
        json.dump(space_target, f, indent=2)

def generate_build_config(config, arch, board, profile):
    """生成构建配置文件"""
    build_dir = Path("build") / "config"
    build_dir.mkdir(parents=True, exist_ok=True)
    
    build_config = {
        "arch": arch,
        "board": board,
        "profile": profile,
        "kernel_target": f"{arch}-unknown-hnx-kernel",
        "space_target": f"{arch}-unknown-hnx",
        "features": config.get("features", []),
        "kernel": config.get("kernel", {}),
        "space": config.get("space", {}),
    }
    
    with open(build_dir / "config.json", 'w') as f:
        json.dump(build_config, f, indent=2)
    
    # 生成环境文件
    env_file = build_dir / "env.sh"
    with open(env_file, 'w') as f:
        f.write(f"export ARCH={arch}\n")
        f.write(f"export BOARD={board}\n")
        f.write(f"export PROFILE={profile}\n")
        f.write(f"export KERNEL_TARGET={arch}-unknown-hnx-kernel\n")
        f.write(f"export SPACE_TARGET={arch}-unknown-hnx\n")

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", required=True)
    parser.add_argument("--board", required=True)
    parser.add_argument("--profile", default="debug")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    
    config = load_config(args.arch, args.board, args.profile)
    print(f"Configuration loaded: {args.arch}/{args.board}/{args.profile}")
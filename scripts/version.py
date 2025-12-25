#!/usr/bin/env python3
"""
HNX 统一版本管理脚本
"""

import os
import sys
import re
import configparser
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional, Tuple
import subprocess
import argparse

class VersionManager:
    def __init__(self, project_root: str = None):
        self.project_root = project_root or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        self.version_file = os.path.join(self.project_root, "VERSION")
        self.config = configparser.ConfigParser(interpolation=None)
        
    def _get_git_hash(self) -> str:
        """获取当前git提交哈希"""
        try:
            result = subprocess.run(
                ["git", "rev-parse", "--short", "HEAD"],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout.strip()
        except (subprocess.CalledProcessError, FileNotFoundError):
            return "unknown"
    
    def _get_build_date(self) -> str:
        """获取构建日期"""
        return datetime.utcnow().strftime("%Y%m%d")
    
    def _expand_variables(self, text: str) -> str:
        """展开变量 ${VAR}"""
        if "${BUILD_DATE}" in text:
            text = text.replace("${BUILD_DATE}", self._get_build_date())
        if "${GIT_HASH}" in text:
            text = text.replace("${GIT_HASH}", self._get_git_hash())
        return text
    
    def load_version(self) -> Dict:
        """加载版本信息"""
        if not os.path.exists(self.version_file):
            raise FileNotFoundError(f"Version file not found: {self.version_file}")
        
        self.config.read(self.version_file)
        
        # 展开变量
        for section in self.config.sections():
            for key in self.config[section]:
                value = self.config[section][key]
                self.config[section][key] = self._expand_variables(value)
        
        return {
            "major": int(self.config["version"]["major"]),
            "minor": int(self.config["version"]["minor"]),
            "patch": int(self.config["version"]["patch"]),
            "prerelease": self.config["version"].get("prerelease", ""),
            "build_date": self.config["metadata"]["build_date"],
            "git_hash": self.config["metadata"]["git_hash"]
        }
    
    def save_version(self, version: Dict):
        """保存版本信息"""
        # 更新config
        self.config["version"] = {
            "major": str(version["major"]),
            "minor": str(version["minor"]),
            "patch": str(version["patch"]),
            "prerelease": version.get("prerelease", "")
        }
        
        self.config["metadata"] = {
            "build_date": "${BUILD_DATE}",
            "git_hash": "${GIT_HASH}"
        }
        
        # 写入文件
        with open(self.version_file, "w") as f:
            self.config.write(f)
    
    def get_version_string(self, include_build: bool = False) -> str:
        """获取版本字符串"""
        version = self.load_version()
        version_str = f"{version['major']}.{version['minor']}.{version['patch']}"
        
        if version["prerelease"]:
            version_str += f"-{version['prerelease']}"
        
        if include_build:
            build_info = f"+{version['build_date']}.{version['git_hash']}"
            version_str += build_info
        
        return version_str
    
    def bump_version(self, part: str, prerelease: Optional[str] = None):
        """递增版本号"""
        version = self.load_version()
        
        if part == "major":
            version["major"] += 1
            version["minor"] = 0
            version["patch"] = 0
            version["prerelease"] = prerelease or "alpha.1"
        elif part == "minor":
            version["minor"] += 1
            version["patch"] = 0
            version["prerelease"] = prerelease or "alpha.1"
        elif part == "patch":
            version["patch"] += 1
            version["prerelease"] = prerelease or ""
        else:
            raise ValueError(f"Invalid version part: {part}")
        
        self.save_version(version)
        print(f"Version bumped to {self.get_version_string()}")
    
    def set_prerelease(self, prerelease: str):
        """设置预发布标签"""
        version = self.load_version()
        version["prerelease"] = prerelease
        self.save_version(version)
        print(f"Prerelease set to '{prerelease}'")
    
    def sync_all(self):
        """同步版本到所有子项目"""
        version = self.load_version()
        version_str = self.get_version_string()
        full_version = self.get_version_string(include_build=True)
        
        print(f"Syncing version {version_str} to all subprojects...")
        
        # 1. 更新内核 Cargo.toml
        kernel_toml = os.path.join(self.project_root, "src", "kernel", "Cargo.toml")
        self._update_toml_version(kernel_toml, version_str)
        
        # 2. 更新用户空间 Cargo.toml
        space_toml = os.path.join(self.project_root, "src", "space", "Cargo.toml")
        self._update_toml_version(space_toml, version_str)
        
        # 3. 生成版本头文件
        self._generate_version_header(full_version, version)
        
        # 4. 生成Rust版本文件
        self._generate_rust_version_file(full_version, version)
        
        print("Sync completed!")
    
    def _update_toml_version(self, toml_path: str, version: str):
        """更新Cargo.toml版本"""
        if not os.path.exists(toml_path):
            print(f"Warning: {toml_path} not found")
            return
        
        with open(toml_path, "r") as f:
            content = f.read()
        
        # 更新版本号
        pattern = r'^version\s*=\s*"[^"]+"'
        replacement = f'version = "{version}"'
        content = re.sub(pattern, replacement, content, flags=re.MULTILINE)
        
        with open(toml_path, "w") as f:
            f.write(content)
        
        print(f"  Updated {os.path.relpath(toml_path, self.project_root)}")
    
    def _generate_version_header(self, full_version: str, version: Dict):
        """生成C版本头文件"""
        header_path = os.path.join(self.project_root, "include", "hnx", "abi", "version.h")
        os.makedirs(os.path.dirname(header_path), exist_ok=True)
        
        header_content = f"""#ifndef _HNX_ABI_VERSION_H
#define _HNX_ABI_VERSION_H

#define HNX_ABI_VERSION_MAJOR {version['major']}
#define HNX_ABI_VERSION_MINOR {version['minor']}
#define HNX_ABI_VERSION_PATCH {version['patch']}
#define HNX_ABI_VERSION "{full_version}"

// 检查兼容性的宏
#define HNX_ABI_CHECK_VERSION(major, minor, patch) \\
    ((major == HNX_ABI_VERSION_MAJOR) && \\
     (minor <= HNX_ABI_VERSION_MINOR))

#endif // _HNX_ABI_VERSION_H
"""
        
        with open(header_path, "w") as f:
            f.write(header_content)
        
        print(f"  Generated {os.path.relpath(header_path, self.project_root)}")
    
    def _generate_rust_version_file(self, full_version: str, version: Dict):
        """生成Rust版本文件"""
        version_rs = os.path.join(self.project_root, "src", "kernel", "src", "version.rs")
        os.makedirs(os.path.dirname(version_rs), exist_ok=True)
        
        prerelease_str = f'"{version["prerelease"]}"' if version["prerelease"] else '""'
        
        rust_content = f"""//! 内核版本信息 - 自动生成，请勿手动修改

/// 主版本号
pub const MAJOR: u32 = {version['major']};

/// 次版本号
pub const MINOR: u32 = {version['minor']};

/// 修订版本号
pub const PATCH: u32 = {version['patch']};

/// 预发布标签
pub const PRERELEASE: &str = {prerelease_str};

/// 完整版本字符串
pub const VERSION_STRING: &str = "{full_version}";

/// 获取版本字符串
#[no_mangle]
pub extern "C" fn hnx_get_version() -> &'static str {{
    VERSION_STRING
}}

/// 获取主版本号
#[no_mangle]
pub extern "C" fn hnx_get_version_major() -> u32 {{
    MAJOR
}}

/// 获取次版本号
#[no_mangle]
pub extern "C" fn hnx_get_version_minor() -> u32 {{
    MINOR
}}

/// 获取修订版本号
#[no_mangle]
pub extern "C" fn hnx_get_version_patch() -> u32 {{
    PATCH
}}
"""
        
        with open(version_rs, "w") as f:
            f.write(rust_content)
        
        print(f"  Generated {os.path.relpath(version_rs, self.project_root)}")

def main():
    parser = argparse.ArgumentParser(description="HNX Version Manager")
    subparsers = parser.add_subparsers(dest="command", help="Commands")
    
    # read 命令
    read_parser = subparsers.add_parser("read", help="Read current version")
    read_parser.add_argument("--full", action="store_true", help="Include build metadata")
    
    # bump 命令
    bump_parser = subparsers.add_parser("bump", help="Bump version")
    bump_parser.add_argument("part", choices=["major", "minor", "patch"], help="Version part to bump")
    bump_parser.add_argument("--prerelease", help="Prerelease tag (e.g., alpha.1, beta.1)")
    
    # set-prerelease 命令
    prerelease_parser = subparsers.add_parser("set-prerelease", help="Set prerelease tag")
    prerelease_parser.add_argument("tag", help="Prerelease tag (empty to remove)")
    
    # sync 命令
    subparsers.add_parser("sync", help="Sync version to all subprojects")
    
    # init 命令
    init_parser = subparsers.add_parser("init", help="Initialize version file")
    init_parser.add_argument("--major", type=int, default=0, help="Major version")
    init_parser.add_argument("--minor", type=int, default=1, help="Minor version")
    init_parser.add_argument("--patch", type=int, default=0, help="Patch version")
    init_parser.add_argument("--prerelease", default="alpha.1", help="Prerelease tag")
    
    args = parser.parse_args()
    manager = VersionManager()
    
    if args.command == "read":
        version_str = manager.get_version_string(include_build=args.full)
        print(version_str)
    
    elif args.command == "bump":
        manager.bump_version(args.part, args.prerelease)
        manager.sync_all()
    
    elif args.command == "set-prerelease":
        tag = args.tag if args.tag != "null" else ""
        manager.set_prerelease(tag)
        manager.sync_all()
    
    elif args.command == "sync":
        manager.sync_all()
    
    elif args.command == "init":
        version = {
            "major": args.major,
            "minor": args.minor,
            "patch": args.patch,
            "prerelease": args.prerelease
        }
        manager.save_version(version)
        print(f"Initialized version to {manager.get_version_string()}")
    
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
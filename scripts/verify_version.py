#!/usr/bin/env python3
"""
版本一致性检查脚本
"""

import os
import re
import sys
from pathlib import Path
from version import VersionManager

def verify_version_consistency():
    """验证所有子项目的版本一致性"""
    print("🔍 Verifying version consistency...")
    
    manager = VersionManager()
    expected_version = manager.get_version_string()
    expected_full = manager.get_version_string(include_build=True)
    
    issues = []
    
    # 检查内核Cargo.toml
    kernel_toml = Path("src/kernel/Cargo.toml")
    if kernel_toml.exists():
        content = kernel_toml.read_text()
        match = re.search(r'version\s*=\s*"([^"]+)"', content)
        if match and match.group(1) != expected_version:
            issues.append(f"src/kernel/Cargo.toml: version mismatch ({match.group(1)} != {expected_version})")
    
    # 检查用户空间Cargo.toml
    space_toml = Path("src/space/Cargo.toml")
    if space_toml.exists():
        content = space_toml.read_text()
        match = re.search(r'version\s*=\s*"([^"]+)"', content)
        if match and match.group(1) != expected_version:
            issues.append(f"src/space/Cargo.toml: version mismatch ({match.group(1)} != {expected_version})")
    
    # 检查版本头文件
    version_h = Path("include/hnx/abi/version.h")
    if version_h.exists():
        content = version_h.read_text()
        if f'"{expected_full}"' not in content:
            issues.append("include/hnx/abi/version.h: version mismatch")
    
    # 检查Rust版本文件
    version_rs = Path("src/kernel/src/version.rs")
    if version_rs.exists():
        content = version_rs.read_text()
        if f'"{expected_full}"' not in content:
            issues.append("src/kernel/src/version.rs: version mismatch")
    
    if issues:
        print("❌ Version inconsistencies found:")
        for issue in issues:
            print(f"  - {issue}")
        sys.exit(1)
    else:
        print("✅ All versions are consistent!")

if __name__ == "__main__":
    verify_version_consistency()
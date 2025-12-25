#!/usr/bin/env python3
"""
发布自动化脚本
"""

import os
import sys
import subprocess
import shutil
from pathlib import Path
from version import VersionManager  # 导入上面的VersionManager

class ReleaseManager:
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.version_manager = VersionManager(str(self.project_root))
        
    def create_release(self, release_type: str):
        """创建发布"""
        # 1. 确保工作目录干净
        self._check_git_status()
        
        # 2. 根据类型递增版本
        if release_type == "major":
            self.version_manager.bump_version("major")
        elif release_type == "minor":
            self.version_manager.bump_version("minor")
        elif release_type == "patch":
            self.version_manager.bump_version("patch")
        else:
            raise ValueError(f"Invalid release type: {release_type}")
        
        # 3. 移除预发布标签
        self.version_manager.set_prerelease("")
        
        # 4. 构建发布版本
        self._build_release()
        
        # 5. 创建git标签
        version = self.version_manager.get_version_string()
        self._create_git_tag(f"v{version}")
        
        print(f"✅ Release {version} created successfully!")
        
        # 6. 开始下一个开发周期
        next_prerelease = "alpha.1"
        self.version_manager.set_prerelease(next_prerelease)
        self.version_manager.sync_all()
        
        subprocess.run(["git", "commit", "-am", f"version: start next development cycle"], check=True)
    
    def _check_git_status(self):
        """检查git状态"""
        result = subprocess.run(
            ["git", "status", "--porcelain"],
            capture_output=True,
            text=True,
            check=True
        )
        
        if result.stdout.strip():
            print("❌ Working directory is not clean. Commit or stash changes first.")
            sys.exit(1)
    
    def _build_release(self):
        """构建发布包"""
        version = self.version_manager.get_version_string()
        build_dir = self.project_root / "build" / f"release-{version}"
        
        # 创建构建目录
        shutil.rmtree(build_dir, ignore_errors=True)
        build_dir.mkdir(parents=True, exist_ok=True)
        
        # 执行构建
        subprocess.run(["make", "clean"], cwd=self.project_root, check=True)
        subprocess.run(["make", "all"], cwd=self.project_root, check=True)
        
        # 复制构建产物
        artifacts = [
            ("bin/hnx-kernel.elf", f"hnx-kernel-{version}.elf"),
            ("bin/initrd.cpio.gz", f"hnx-initrd-{version}.cpio.gz"),
            ("bin/hnx-image.img", f"hnx-image-{version}.img"),
        ]
        
        for src, dst in artifacts:
            src_path = self.project_root / src
            if src_path.exists():
                shutil.copy2(src_path, build_dir / dst)
        
        # 创建发布包
        import tarfile
        tar_path = self.project_root / "releases" / f"hnx-{version}.tar.gz"
        tar_path.parent.mkdir(parents=True, exist_ok=True)
        
        with tarfile.open(tar_path, "w:gz") as tar:
            tar.add(build_dir, arcname=f"hnx-{version}")
        
        print(f"📦 Release package created: {tar_path}")
    
    def _create_git_tag(self, tag: str):
        """创建git标签"""
        subprocess.run(["git", "tag", "-a", tag, "-m", f"Release {tag}"], check=True)
        print(f"🏷️  Git tag created: {tag}")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="HNX Release Manager")
    parser.add_argument("type", choices=["major", "minor", "patch"], help="Release type")
    
    args = parser.parse_args()
    
    manager = ReleaseManager()
    manager.create_release(args.type)
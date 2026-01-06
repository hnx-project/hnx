# HNX
## 项目结构
```shell
hnx/                              # 项目根目录
├── .github/                      # GitHub 工作流
├── .vscode/                      # IDE 配置
├── build/                        # 构建产物（不提交）
├── docs/                         # 项目文档
├── scripts/                      # 构建脚本
├── third_party/                  # 第三方代码（子模块）
├── configs/                      # 系统配置（TOML）
├── targets/                      # Rust 目标定义
├── toolchain/                    # 工具链构建
├── tools/                        # 开发工具
├── tests/                        # 测试框架
├── src/                          # 源代码（Rust 约定）
│   ├── kernel/                   # 内核核心（库 crate）
│   │   ├── src/                  # Rust 源文件
│   │   │   ├── lib.rs           # 内核库入口
│   │   │   ├── arch/            # 架构相关
│   │   │   ├── object/          # 内核对象系统
│   │   │   ├── vm/              # 虚拟内存管理
│   │   │   ├── syscalls/        # 系统调用实现
│   │   │   ├── dev/             # 设备框架
│   │   │   └── platform/        # 平台支持
│   │   ├── include/             # C 头文件（如有需要）
│   │   ├── build.rs             # 内核构建脚本
│   │   ├── linker.ld            # 内核链接脚本
│   │   └── Cargo.toml           # 内核 crate 配置
│   ├── space/                    # 用户空间组件（Workspace）
│   │   ├── Cargo.toml           # Workspace 配置
│   │   ├── abi/                  # ABI 定义 crate
│   │   │   ├── src/
│   │   │   │   ├── lib.rs       # ABI 定义
│   │   │   │   ├── syscalls.rs  # 系统调用号
│   │   │   │   └── types.rs     # 共享类型
│   │   │   ├── build.rs         # 生成绑定
│   │   │   └── Cargo.toml
│   │   ├── libc/                 # C 标准库 crate
│   │   │   ├── src/
│   │   │   │   ├── lib.rs
│   │   │   │   ├── stdio.rs
│   │   │   │   └── stdlib.rs
│   │   │   ├── include/         # C 头文件
│   │   │   └── Cargo.toml
│   │   ├── hnxlib/               # HNX 系统库 crate
│   │   │   ├── src/
│   │   │   │   ├── lib.rs
│   │   │   │   ├── channel.rs
│   │   │   │   └── process.rs
│   │   │   └── Cargo.toml
│   │   ├── services/            # 系统服务（二进制 crate）
│   │   │   ├── init/
│   │   │   │   ├── src/
│   │   │   │   │   └── main.rs  # init 程序入口
│   │   │   │   └── Cargo.toml
│   │   │   ├── display/
│   │   │   └── audio/
│   │   ├── drivers/             # 用户空间驱动
│   │   ├── apps/                # 应用程序
│   │   └── boot/                # 引导相关
│   └── utils/                    # 工具和实用程序（可选）
│       ├── kernel-build/         # 内核构建工具
│       └── image-builder/        # 镜像构建工具
├── Makefile                      # 顶层构建入口
├── Cargo.toml                    # 根工作区配置
├── Cargo.lock                    # 依赖锁定
├── .gitmodules                   # Git 子模块管理
└── README.md                     # 项目说明
```

## Documentation

- Boot Process (AArch64 + Rust):
  https://tinchy.site/hnx/a001-booting-hnx-kernel
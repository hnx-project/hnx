// build.rs
// unuse warnings
#![allow(unused)]
use std::env;
use std::fs;
use std::path::PathBuf;
use toml::Value;

fn main() {
    // 1. 获取目标架构
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target = env::var("TARGET").unwrap();

    println!("Building HNX kernel for {}", target_arch);

    // 2. 生成链接脚本 和 配置头文件
    generate_linker_script_and_config_header(&target_arch, &target);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/linker.ld");
    println!("cargo:rerun-if-changed=../../configs/arch/");
}

// fn link_libc() {
//     // 链接 libc 库
//     let libc_path = "../libc/libhnxc.a";
//     println!("cargo:rustc-link-search=native=../libc");
//     println!("cargo:rustc-link-lib=static=hnxc");
//     println!("cargo:rerun-if-changed={}", libc_path);
// }

fn generate_linker_script_and_config_header(arch: &str, _target: &str) {
    // 读取模板（优先使用仓库模板）
    let template = fs::read_to_string("src/linker.ld").unwrap();
    let config = match arch {
        "aarch64" => {
            r#"
                #define ARCH_AARCH64 1
                #define PAGE_SIZE {{PAGE_SIZE}}
                #define KERNEL_BASE {{KERNEL_BASE}}
                #define STACK_SIZE {{STACK_SIZE}}
                "#
        }
        _ => "",
    };
    // 读取架构配置
    let config_path = format!("../../configs/arch/{}.toml", arch);
    let cfg_str = fs::read_to_string(&config_path).unwrap_or_else(|_| String::new());
    let cfg: Value = toml::from_str(&cfg_str).unwrap_or(Value::Table(Default::default()));

    // 提取配置项，带默认值
    // endian
    let endian = cfg
        .get("arch")
        .and_then(|a| a.get("endian"))
        .and_then(Value::as_str)
        .unwrap_or("little");

    // kernel_base
    let kernel_base = cfg
        .get("arch")
        .and_then(|a| a.get("kernel_base"))
        .and_then(Value::as_str)
        .unwrap_or("0xffff000000000000");

    // page_size
    let page_size = cfg
        .get("arch")
        .and_then(|a| a.get("page_size"))
        .and_then(Value::as_integer)
        .unwrap_or(4096);

    // stack_size
    let stack_size = cfg
        .get("arch")
        .and_then(|a| a.get("stack_size"))
        .and_then(Value::as_integer)
        .unwrap_or(65536);

    // physical_base
    let phys_base = cfg
        .get("arch")
        .and_then(|a| a.get("physical_base"))
        .and_then(Value::as_str)
        .unwrap_or("0x40000000");

    // 输出格式
    let output_format = match (arch, endian) {
        ("aarch64", "little") => "elf64-littleaarch64",
        ("aarch64", "big") => "elf64-bigaarch64",
        _ => panic!("Unsupported architecture: {}", arch),
    };
    let output_arch = arch;

    // 替换模板变量
    let script = template
        .replace("{{OUTPUT_FORMAT}}", output_format)
        .replace("{{OUTPUT_ARCH}}", output_arch)
        .replace("{{KERNEL_BASE}}", kernel_base)
        .replace("{{PAGE_SIZE}}", &page_size.to_string())
        .replace("{{STACK_SIZE}}", &format!("0x{:x}", stack_size))
        .replace("{{PHYS_BASE}}", phys_base);

    // 写入输出目录
    let out_dir = env::var("OUT_DIR").unwrap();
    let script_path = PathBuf::from(out_dir).join("linker.ld");
    fs::write(&script_path, script).unwrap();

    // 告诉 Cargo 使用这个链接脚本
    println!("cargo:rustc-link-arg=-T{}", script_path.display());
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
}

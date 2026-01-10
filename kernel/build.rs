// build.rs
// unuse warnings
#![allow(unused)]
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
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
    let out_dir_clone = out_dir.clone(); // 克隆用于后续使用
    let script_path = PathBuf::from(out_dir).join("linker.ld");
    fs::write(&script_path, script).unwrap();

    // 生成 config.rs 文件
    generate_config_rs(&out_dir_clone, arch, page_size as usize, stack_size as usize, kernel_base, phys_base);

    // 告诉 Cargo 使用这个链接脚本
    println!("cargo:rustc-link-arg=-T{}", script_path.display());
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
}

fn generate_config_rs(out_dir: &str, arch: &str, page_size: usize, stack_size: usize, kernel_base: &str, phys_base: &str) {
    // 计算 PAGE_SHIFT 和 PAGE_MASK
    let page_shift = page_size.trailing_zeros() as usize;
    let page_mask = page_size - 1;
    
    // 解析十六进制字符串为数字
    let kernel_base_num = parse_hex(kernel_base);
    let phys_base_num = parse_hex(phys_base);
    
    // 生成 config .rs内容
    let config_rs = format!(
        r#"// 自动生成的配置常量
// 此文件由 build.rs 生成，请勿手动修改

/// 当前目标架构
pub const ARCH: &str = "{arch}";

/// 是否 AArch64 架构
pub const ARCH_AARCH64: bool = {is_aarch64};

/// 页面大小（字节）
pub const PAGE_SIZE: usize = {page_size};

/// 页面移位（log2(PAGE_SIZE)）
pub const PAGE_SHIFT: usize = {page_shift};

/// 页面掩码（PAGE_SIZE - 1）
pub const PAGE_MASK: usize = {page_mask};

/// 内核虚拟基地址
pub const KERNEL_BASE: usize = {kernel_base_num};

/// 物理内存基地址
pub const PHYSICAL_BASE: usize = {phys_base_num};

/// 默认栈大小（字节）
pub const STACK_SIZE: usize = {stack_size};

/// 对齐到页面边界
pub const fn align_to_page(addr: usize) -> usize {{
    (addr + PAGE_MASK) & !PAGE_MASK
}}

/// 是否页面对齐
pub const fn is_page_aligned(addr: usize) -> bool {{
    addr & PAGE_MASK == 0
}}

/// 转换为页号
pub const fn addr_to_page(addr: usize) -> usize {{
    addr >> PAGE_SHIFT
}}

/// 从页号转换为地址
pub const fn page_to_addr(page: usize) -> usize {{
    page << PAGE_SHIFT
}}
"#,
        arch = arch,
        is_aarch64 = if arch == "aarch64" { "true" } else { "false" },
        page_size = page_size,
        page_shift = page_shift,
        page_mask = page_mask,
        kernel_base_num = kernel_base_num,
        phys_base_num = phys_base_num,
        stack_size = stack_size,
    );
    
    // 写入 config.rs 文件
    let config_path = PathBuf::from(out_dir).join("config.rs");
    fs::write(&config_path, config_rs).unwrap();
    
    println!("cargo:rustc-cfg=has_config_rs");
    println!("cargo:rerun-if-changed=../../configs/arch/{}.toml", arch);
}

fn parse_hex(hex_str: &str) -> usize {
    if hex_str.starts_with("0x") || hex_str.starts_with("0X") {
        usize::from_str_radix(&hex_str[2..], 16).unwrap_or(0)
    } else {
        hex_str.parse().unwrap_or(0)
    }
}

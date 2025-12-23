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

    // 3. 链接 libc 库
    link_libc();

    // 4. 设置重新编译条件
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/scripts/link.ld.S");
    println!("cargo:rerun-if-changed=src/configs/");
}

fn link_libc() {
    // 链接 libc 库
    let libc_path = "../libc/libhnxc.a";
    println!("cargo:rustc-link-search=native=../libc");
    println!("cargo:rustc-link-lib=static=hnxc");
    println!("cargo:rerun-if-changed={}", libc_path);
}

fn generate_linker_script_and_config_header(arch: &str, _target: &str) {
    // 读取模板（优先使用仓库模板）
    let template = fs::read_to_string("src/scripts/link.ld.S").unwrap();
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
    let config_path = format!("src/configs/{}.toml", arch);
    let cfg_str = fs::read_to_string(&config_path).unwrap_or_else(|_| String::new());
    let cfg: Value = toml::from_str(&cfg_str).unwrap_or(Value::Table(Default::default()));

    // 提取配置项，带默认值
    let endian = cfg
        .get("arch")
        .and_then(|a| a.get("endian"))
        .and_then(Value::as_str)
        .unwrap_or("little");
    let kernel_base = cfg
        .get("arch")
        .and_then(|a| a.get("kernel_base"))
        .and_then(Value::as_str)
        .unwrap_or("0xffff000000000000");
    let page_size = cfg
        .get("arch")
        .and_then(|a| a.get("page_size"))
        .and_then(Value::as_integer)
        .unwrap_or(4096);
    let stack_size = cfg
        .get("arch")
        .and_then(|a| a.get("stack_size"))
        .and_then(Value::as_integer)
        .unwrap_or(65536);
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
    let header_h = config
        .replace("{{OUTPUT_FORMAT}}", output_format)
        .replace("{{OUTPUT_ARCH}}", output_arch)
        .replace("{{KERNEL_BASE}}", kernel_base)
        .replace("{{PAGE_SIZE}}", &page_size.to_string())
        .replace("{{STACK_SIZE}}", &format!("0x{:x}", stack_size))
        .replace("{{PHYS_BASE}}", phys_base);

    // 读取运行期配置并导出环境变量
    let defaults_platform = cfg
        .get("defaults")
        .and_then(|d| d.get("platform"))
        .and_then(Value::as_str)
        .unwrap_or("virt");
    let platforms = cfg.get("platforms");
    let binding_plat = Value::Table(Default::default());
    let plat_tbl = platforms
        .and_then(|p| p.get(defaults_platform))
        .unwrap_or(&binding_plat);
    let uart_driver = plat_tbl
        .get("uart")
        .and_then(Value::as_str)
        .or_else(|| cfg.get("uart").and_then(|u| u.get("driver")).and_then(Value::as_str))
        .unwrap_or("pl011");
    let irq_ctl = plat_tbl
        .get("irq_controller")
        .and_then(Value::as_str)
        .unwrap_or("gicv2");
    let timer_tick_ms = cfg
        .get("timer")
        .and_then(|t| t.get("tick_ms"))
        .and_then(Value::as_integer)
        .unwrap_or(500);
    let mmio_tbl = plat_tbl.get("mmio");
    let binding_mmio = Value::Table(Default::default());
    let virt_mmio = mmio_tbl.unwrap_or(&binding_mmio);
    let pl011_base_hex = virt_mmio
        .get("pl011_base")
        .and_then(Value::as_str)
        .unwrap_or("0x09000000");
    let gicd_base_hex = virt_mmio
        .get("gicd_base")
        .and_then(Value::as_str)
        .unwrap_or("0x08000000");
    let gicc_base_hex = virt_mmio
        .get("gicc_base")
        .and_then(Value::as_str)
        .unwrap_or("0x08010000");
    fn hex_str_to_u64(s: &str) -> u64 {
        let t = s.trim().trim_start_matches("0x");
        u64::from_str_radix(t, 16).unwrap_or(0)
    }
    let pl011_base_dec = hex_str_to_u64(pl011_base_hex);
    let gicd_base_dec = hex_str_to_u64(gicd_base_hex);
    let gicc_base_dec = hex_str_to_u64(gicc_base_hex);
    println!("cargo:rustc-env=HNX_PLATFORM_DEFAULT={}", defaults_platform);
    println!("cargo:rustc-env=HNX_UART_DEFAULT={}", uart_driver);
    println!("cargo:rustc-env=HNX_IRQ_DEFAULT={}", irq_ctl);
    println!("cargo:rustc-env=HNX_TIMER_DEFAULT_MS={}", timer_tick_ms);
    println!("cargo:rustc-env=HNX_FALLBACK_PL011_BASE_DEC={}", pl011_base_dec);
    println!("cargo:rustc-env=HNX_FALLBACK_GICD_BASE_DEC={}", gicd_base_dec);
    println!("cargo:rustc-env=HNX_FALLBACK_GICC_BASE_DEC={}", gicc_base_dec);

    // 写入输出目录
    let out_dir = env::var("OUT_DIR").unwrap();
    let script_path = PathBuf::from(out_dir).join("link.ld");
    fs::write(&script_path, script).unwrap();
    let out_dir2 = env::var("OUT_DIR").unwrap();
    let header_path: PathBuf = PathBuf::from(out_dir2).join("config.h");
    fs::write(header_path, header_h).unwrap();

    // 告诉 Cargo 使用这个链接脚本
    println!("cargo:rustc-link-arg=-T{}", script_path.display());
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
    println!("cargo:rerun-if-changed={}", config_path);
    
}
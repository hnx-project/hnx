// build.rs
use std::env;
use std::fs;
use std::path::PathBuf;
use toml::Value;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target = env::var("TARGET").unwrap();
    
    println!("Building HNX kernel for {} {}", target_arch, target);
    
    // 生成链接脚本和配置
    generate_linker_script_and_config(&target_arch);
    
    // 设置重新编译条件
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../configs/scripts/link.ld.S");
}

fn generate_linker_script_and_config(arch: &str) {
    // 1. 读取新配置文件（workspace级别）
    let workspace_config = load_workspace_config(arch);
    
    // 2. 读取旧内核配置文件（如存在，用于兼容）
    let kernel_config = load_kernel_config_if_exists(arch);
    
    // 3. 合并配置，新配置优先，旧配置作后备
    let config = merge_configs(&workspace_config, &kernel_config);
    
    // 4. 生成链接脚本
    generate_linker_script(&config, arch);
    
    // 5. 生成内核配置文件
    generate_kernel_config(&config);
    
    // 6. 导出环境变量给内核使用
    export_config_to_env(&config);
}

fn load_workspace_config(arch: &str) -> Value {
    // 读取架构配置
    let arch_path = format!("../../configs/arch/{}.toml", arch);
    let arch_str = fs::read_to_string(&arch_path)
        .unwrap_or_else(|_| String::new());
    let mut config: Value = toml::from_str(&arch_str).unwrap();
    
    // 读取板级配置（如果指定了环境变量）
    if let Ok(board) = env::var("BOARD") {
        let board_path = format!("../../configs/board/{}.toml", board);
        if let Ok(board_str) = fs::read_to_string(&board_path) {
            let board_config: Value = toml::from_str(&board_str).unwrap();
            // 合并板级配置
            merge_value(&mut config, board_config);
        }
    }
    
    // 读取profile配置
    if let Ok(profile) = env::var("PROFILE") {
        let profile_path = format!("../../configs/profile/{}.toml", profile);
        if let Ok(profile_str) = fs::read_to_string(&profile_path) {
            let profile_config: Value = toml::from_str(&profile_str).unwrap();
            merge_value(&mut config, profile_config);
        }
    }
    
    config
}

fn load_kernel_config_if_exists(arch: &str) -> Option<Value> {
    let kernel_config_path = format!("src/configs/{}.toml", arch);
    fs::read_to_string(&kernel_config_path).ok().and_then(|s| {
        toml::from_str(&s).ok()
    })
}

fn merge_configs(workspace: &Value, kernel: &Option<Value>) -> Config {
    let mut config = Config::default();
    
    // 1. 从workspace配置获取核心字段
    if let Some(arch) = workspace.get("arch") {
        config.name = arch.get("name").and_then(Value::as_str).unwrap_or("unknown").to_string();
        config.page_size = arch.get("page_size").and_then(Value::as_integer).unwrap_or(4096) as u64;
        config.kernel_base = arch.get("kernel_base").and_then(Value::as_str)
            .unwrap_or("0xffff000000000000").to_string();
        config.endian = arch.get("endian").and_then(Value::as_str)
            .unwrap_or("little").to_string();
    }
    
    if let Some(memory) = workspace.get("memory") {
        config.phys_base = memory.get("physical_base").and_then(Value::as_str)
            .unwrap_or("0x40000000").to_string();
        config.stack_size = memory.get("stack_size").and_then(Value::as_integer)
            .unwrap_or(65536) as u64;
    }
    
    if let Some(kernel_cfg) = workspace.get("kernel") {
        config.max_processes = kernel_cfg.get("max_processes").and_then(Value::as_integer)
            .unwrap_or(256) as u64;
        config.max_threads = kernel_cfg.get("max_threads").and_then(Value::as_integer)
            .unwrap_or(1024) as u64;
    }
    
    // 2. 从旧配置获取兼容字段（仅当新配置不存在时）
    if let Some(ref kernel_config) = kernel {
        // 填充新配置缺少的字段
        if config.endian.is_empty() {
            config.endian = kernel_config.get("arch")
                .and_then(|a| a.get("endian"))
                .and_then(Value::as_str)
                .unwrap_or("little")
                .to_string();
        }
        
        // 获取平台默认值
        if let Some(defaults) = kernel_config.get("defaults") {
            config.default_platform = defaults.get("platform")
                .and_then(Value::as_str)
                .unwrap_or("virt")
                .to_string();
        }
        
        // 获取平台特定配置
        if let Some(platforms) = kernel_config.get("platforms") {
            if let Some(plat) = platforms.get(&config.default_platform) {
                config.uart_driver = plat.get("uart")
                    .and_then(Value::as_str)
                    .unwrap_or("pl011")
                    .to_string();
                
                if let Some(mmio) = plat.get("mmio") {
                    config.pl011_base = mmio.get("pl011_base")
                        .and_then(Value::as_str)
                        .unwrap_or("0x09000000")
                        .to_string();
                }
            }
        }
    }
    
    config
}

fn generate_linker_script(config: &Config, arch: &str) {
    // 读取模板
    let template = fs::read_to_string("../../configs/scripts/link.ld.S")
        .expect("Failed to read linker template");
    
    // 设置输出格式
    let output_format = match (arch, config.endian.as_str()) {
        ("aarch64", "little") => "elf64-littleaarch64",
        ("aarch64", "big") => "elf64-bigaarch64",
        _ => panic!("Unsupported architecture: {} with endian: {}", arch, config.endian.as_str()),
    };
    
    // 替换变量
    let script = template
        .replace("{{OUTPUT_FORMAT}}", output_format)
        .replace("{{OUTPUT_ARCH}}", arch)
        .replace("{{KERNEL_BASE}}", &config.kernel_base)
        .replace("{{PAGE_SIZE}}", &config.page_size.to_string())
        .replace("{{STACK_SIZE}}", &format!("0x{:x}", config.stack_size))
        .replace("{{PHYS_BASE}}", &config.phys_base);
    
    // 写入输出目录
    let out_dir = env::var("OUT_DIR").unwrap();
    let script_path = PathBuf::from(out_dir).join("link.ld");
    fs::write(&script_path, script).unwrap();
    
    // 告诉Cargo使用这个链接脚本
    println!("cargo:rustc-link-arg=-T{}", script_path.display());
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");
}

fn generate_kernel_config(config: &Config) {
    // 生成Rust配置文件
    let out_dir = env::var("OUT_DIR").unwrap();
    let config_rs = format!(r#"
        pub const PAGE_SIZE: usize = {};
        pub const KERNEL_BASE: usize = 0x{:x};
        pub const PHYS_BASE: usize = 0x{:x};
        pub const STACK_SIZE: usize = {};
        pub const MAX_PROCESSES: usize = {};
        pub const MAX_THREADS: usize = {};
    "#,
        config.page_size,
        parse_hex(&config.kernel_base),
        parse_hex(&config.phys_base),
        config.stack_size,
        config.max_processes,
        config.max_threads
    );
    
    let config_path = PathBuf::from(out_dir).join("config.rs");
    fs::write(config_path, config_rs).unwrap();
}

fn export_config_to_env(config: &Config) {
    // 导出关键配置为环境变量
    println!("cargo:rustc-env=HNX_PAGE_SIZE={}", config.page_size);
    println!("cargo:rustc-env=HNX_KERNEL_BASE={}", config.kernel_base);
    println!("cargo:rustc-env=HNX_PHYS_BASE={}", config.phys_base);
    
    if !config.uart_driver.is_empty() {
        println!("cargo:rustc-env=HNX_UART_DRIVER={}", config.uart_driver);
    }
    if !config.pl011_base.is_empty() {
        println!("cargo:rustc-env=HNX_PL011_BASE={}", config.pl011_base);
    }
}

// 辅助函数
fn merge_value(target: &mut Value, source: Value) {
    if let (Value::Table(t), Value::Table(s)) = (target, source) {
        for (k, v) in s {
            if let Some(existing) = t.get_mut(&k) {
                merge_value(existing, v);
            } else {
                t.insert(k, v);
            }
        }
    }
}

fn parse_hex(hex_str: &str) -> u64 {
    let trimmed = hex_str.trim().trim_start_matches("0x");
    u64::from_str_radix(trimmed, 16).unwrap_or(0)
}

#[derive(Default)]
struct Config {
    name: String,
    endian: String,
    page_size: u64,
    kernel_base: String,
    phys_base: String,
    stack_size: u64,
    max_processes: u64,
    max_threads: u64,
    default_platform: String,
    uart_driver: String,
    pl011_base: String,
}
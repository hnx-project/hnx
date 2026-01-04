// abi-bindings/build.rs
extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../include/hnx/abi/syscalls.h");
    println!("cargo:rerun-if-changed=../include/hnx/abi/types.h");
    println!("cargo:rerun-if-changed=../include/hnx/abi/errors.h");
    
    // 设置 clang 参数
    let target = env::var("TARGET").unwrap_or_default();
    let mut clang_args = Vec::new();
    
    // 添加系统包含路径
    if let Ok(sysroot) = env::var("SYSROOT") {
        clang_args.push(format!("--sysroot={}", sysroot));
    }
    
    // 根据目标架构添加参数
    if target.contains("aarch64") {
        clang_args.push("-target".to_string());
        clang_args.push("aarch64-none-elf".to_string());
    }
    
    // 生成绑定
    let bindings = bindgen::Builder::default()
        .header("../include/hnx/abi/syscalls.h")
        .clang_args(&clang_args)
        .use_core()
        .ctypes_prefix("::core::ffi")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    
    // 写入输出目录
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    
    // 也复制到源码目录供 IDE 使用
    let src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("src");
    
    let generated_path = src_dir.join("generated.rs");
    bindings
        .write_to_file(&generated_path)
        .expect("Couldn't write to src/generated.rs");
    
    // 生成一个简化的版本用于内核
    generate_kernel_constants();
}

fn generate_kernel_constants() {
    // 从头文件提取常量，生成简单的 Rust 文件
    let header_content = std::fs::read_to_string("../include/hnx/abi/syscalls.h")
        .expect("Failed to read syscalls.h");
    
    let mut rust_constants = String::new();
    rust_constants.push_str("//! 自动生成的系统调用常量\n");
    rust_constants.push_str("//! 请勿手动修改\n\n");
    rust_constants.push_str("#![allow(non_upper_case_globals, dead_code)]\n\n");
    
    // 提取 #define 常量
    for line in header_content.lines() {
        let line = line.trim();
        if line.starts_with("#define") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[1];
                let value = parts[2];
                
                // 跳过函数原型
                if name.contains('(') {
                    continue;
                }
                
                // Convert SYS_ prefix to HNX_SYS_ prefix for consistency
                let new_name = if name.starts_with("SYS_") {
                    name.replacen("SYS_", "HNX_SYS_", 1)
                } else {
                    name.to_string()
                };
                rust_constants.push_str(&format!("pub const {}: u32 = {};\n", new_name, value));
            }
        }
    }
    
    // 写入内核目录
    let kernel_abi_path = PathBuf::from("../kernel/src/abi/constants.rs");
    std::fs::create_dir_all(kernel_abi_path.parent().unwrap())
        .expect("Failed to create directory");
    
    std::fs::write(&kernel_abi_path, rust_constants)
        .expect("Failed to write kernel constants");
    
    println!("cargo:rerun-if-changed={}", kernel_abi_path.display());
}
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::{println, ipc::{IpcError, ServiceFramework}, syscall::spawn_service};

mod elf;

/// Loader Service 主函数
///
/// 职责：加载所有用户空间程序和服务，提供统一的加载接口
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("========================================");
    println!("       HNX Loader Service (PID {})", get_pid());
    println!("========================================");

    println!("[1/3] 初始化 ELF 加载器...");
    println!("[2/3] 等待 IPC Router 服务就绪...");

    // 等待 IPC Router 服务启动
    for i in 0..10 {
        println!("  等待 IPC Router (尝试 {}/10)...", i + 1);
        hnxlib::syscall::yield_cpu();
    }

    println!("[3/3] 创建服务框架...");

    // 创建服务框架
    let framework = match ServiceFramework::new("loader") {
        Ok(f) => {
            println!("  服务框架创建成功");
            Some(f)
        },
        Err(e) => {
            println!("  警告：服务框架创建失败: {:?}", e);
            None
        }
    };

    // 主动加载核心服务
    println!("[loader] 主动加载核心服务...");
    load_core_services();

    if let Some(framework) = framework {
        // 运行服务框架处理后续请求
        println!("[INFO] Loader Service 已启动，等待请求...");
        framework.run(handle_message);
    } else {
        // 进入简单模式
        println!("[loader] 进入简单模式...");
        simple_loader_loop();
    }
}

/// 简单的加载器循环（备用）
fn simple_loader_loop() -> ! {
    loop {
        println!("[loader] 备用模式运行中...");
        for _ in 0..20 {
            hnxlib::syscall::yield_cpu();
        }
    }
}

/// 处理 IPC 消息
fn handle_message(op: u16, request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    match op {
        // 操作码 100: 加载可执行文件
        100 => handle_load_executable(request, response),
        // 操作码 101: 查询加载器信息
        101 => handle_get_info(response),
        // 操作码 102: 测试功能
        102 => handle_test(request, response),
        _ => Err(IpcError::InvalidArgs),
    }
}

/// 处理加载可执行文件请求
fn handle_load_executable(request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 请求格式：[路径长度(8字节)][路径...]
    if request.len() < 8 {
        return Err(IpcError::InvalidArgs);
    }

    // 解析路径长度
    let path_len_bytes = &request[0..8];
    let path_len = usize::from_ne_bytes([
        path_len_bytes[0], path_len_bytes[1], path_len_bytes[2], path_len_bytes[3],
        path_len_bytes[4], path_len_bytes[5], path_len_bytes[6], path_len_bytes[7],
    ]);

    if request.len() < 8 + path_len {
        return Err(IpcError::InvalidArgs);
    }

    // 解析路径
    let path_data = &request[8..8 + path_len];
    let path = match core::str::from_utf8(path_data) {
        Ok(s) => s,
        Err(_) => return Err(IpcError::InvalidArgs),
    };

    println!("[loader] 收到加载请求: {}", path);

    // 调用内核系统调用来加载服务
    let result = spawn_service(path);
    println!("[loader] spawn_service 返回: {}", result);

    // 响应格式：[结果代码(8字节)][PID(8字节)可选]
    if response.len() < 8 {
        return Ok(0);
    }

    if result >= 0 {
        // 成功：返回PID
        let pid = result as usize;
        let pid_bytes = pid.to_ne_bytes();
        response[0..8].copy_from_slice(&pid_bytes);
        println!("[loader] 服务加载成功，PID: {}", pid);
        Ok(8)
    } else {
        // 失败：返回错误代码
        let error_code = result as i64;
        let error_bytes = error_code.to_ne_bytes();
        response[0..8].copy_from_slice(&error_bytes);
        println!("[loader] 服务加载失败，错误代码: {}", error_code);
        Ok(8)
    }
}

/// 处理获取加载器信息请求
fn handle_get_info(response: &mut [u8]) -> Result<usize, IpcError> {
    // 使用固定信息避免format
    const INFO: &[u8] = b"HNX Loader Service v0.1\n- ELF parser: available\n- Status: operational\n- PID: 2";

    let len = INFO.len().min(response.len());
    response[..len].copy_from_slice(&INFO[..len]);
    Ok(len)
}

/// 处理测试请求
fn handle_test(request: &[u8], response: &mut [u8]) -> Result<usize, IpcError> {
    // 测试 ELF 解析器
    if request.len() > 0 {
        println!("[loader] 测试 ELF 解析器...");

        // 创建一个简单的测试 ELF 头部
        let test_elf: &[u8] = &[
            0x7F, b'E', b'L', b'F', // ELF magic
            2, 1, 1, 0,            // 64-bit, little endian
            0, 0, 0, 0, 0, 0, 0, 0, // padding
            2, 0,                  // ET_EXEC
            0xB7, 0,               // EM_AARCH64
            1, 0, 0, 0,            // version
            0x80, 0, 0, 0, 0, 0, 0, 0, // entry point
            0x40, 0, 0, 0, 0, 0, 0, 0, // phoff
            0, 0, 0, 0, 0, 0, 0, 0, // shoff
            0, 0, 0, 0,            // flags
            0x40, 0,               // ehsize
            0x38, 0,               // phentsize
            1, 0,                  // phnum
            0x40, 0,               // shentsize
            0, 0,                  // shnum
            0, 0,                  // shstrndx
        ];

        match elf::ElfLoader::new(test_elf) {
            Ok(loader) => {
                println!("[loader] ELF 解析器测试通过: entry=0x{:X}", loader.entry());
                const RESPONSE_OK: &[u8] = b"ELF parser test: OK";
                let len = RESPONSE_OK.len().min(response.len());
                response[..len].copy_from_slice(&RESPONSE_OK[..len]);
                return Ok(len);
            }
            Err(e) => {
                println!("[loader] ELF 解析器测试失败: {}", e);
                const RESPONSE_ERR: &[u8] = b"ELF parser test: ERROR";
                let len = RESPONSE_ERR.len().min(response.len());
                response[..len].copy_from_slice(&RESPONSE_ERR[..len]);
                return Ok(len);
            }
        }
    }

    const RESPONSE_AVAILABLE: &[u8] = b"Loader test: available";
    let len = RESPONSE_AVAILABLE.len().min(response.len());
    response[..len].copy_from_slice(&RESPONSE_AVAILABLE[..len]);
    Ok(len)
}

/// 加载核心服务
fn load_core_services() {
    println!("[loader] 开始加载核心服务...");

    // 首先测试加载一个已知存在的服务
    println!("[loader] 测试加载自身: /bin/loader-service");
    let test_result = spawn_service("/bin/loader-service");
    println!("[loader] 测试结果: {}", test_result);

    // 核心服务列表（完整路径）
    let services = ["/bin/vfs-service", "/bin/procmgr-service", "/bin/echo-service"];

    for path in services.iter() {
        println!("[loader] 加载服务: {}", path);

        let result = spawn_service(path);
        if result >= 0 {
            println!("[loader]  成功加载 {}，PID: {}", path, result);
        } else {
            println!("[loader]  加载 {} 失败，错误代码: {}", path, result);
        }

        // 短暂让出CPU，让新服务有机会运行
        for _ in 0..5 {
            hnxlib::syscall::yield_cpu();
        }
    }

    println!("[loader] 核心服务加载完成");
}

/// 获取进程ID（临时实现）
fn get_pid() -> i32 {
    // TODO: 实现 getpid 系统调用
    2 // loader-service 通常是 PID 2
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[loader] PANIC: {}", info);

    if let Some(location) = info.location() {
        println!("[loader] at {}:{}:{}",
                location.file(),
                location.line(),
                location.column());
    }

    println!("[loader] Loader halted");
    loop {
        hnxlib::syscall::yield_cpu();
    }
}
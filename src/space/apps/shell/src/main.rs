#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::{println, print, syscall};
use hnx_abi::*;
use hnxlib::ipc::{IpcError, IpcRouterClient, Endpoint};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();
    // Never return
    loop {
        syscall::yield_cpu();
    }
}

/// 环境变量管理器
struct Environment {
    path: &'static str,
    home: &'static str,
    shell: &'static str,
}

impl Environment {
    fn new() -> Self {
        Self {
            path: "/bin",
            home: "/",
            shell: "/bin/hnx-shell",
        }
    }

    /// 获取PATH环境变量
    fn get_path(&self) -> &str {
        self.path
    }

    /// 查找可执行文件路径
    fn find_executable(&self, name: &str) -> Option<&'static str> {
        let path_dirs = self.path.split(':');
        for dir in path_dirs {
            // 检查是否是/bin目录（我们知道的目录）
            if dir == "/bin" {
                // 检查是否有对应的服务文件
                // 注意：这里我们硬编码几个已知的服务
                match name {
                    "loader-service" => return Some("/bin/loader-service"),
                    "ipcrouter-service" => return Some("/bin/ipcrouter-service"),
                    "procmgr-service" => return Some("/bin/procmgr-service"),
                    "vfs-service" => return Some("/bin/vfs-service"),
                    "echo-service" => return Some("/bin/echo-service"),
                    _ => continue,
                }
            }
        }
        None
    }
}

fn main() {
    // Simple debug info to confirm shell starts
    println!("\n[SHELL] HNX Shell starting...");

    println!("\n========================================");
    println!("        HNX Shell v0.1.0");
    println!("========================================");
    println!("Type 'help' for available commands");
    println!("Press Enter to execute command\n");

    // TEST: Simple output to verify shell is running
    println!("[SHELL] TEST: Shell is alive!");

    // Initialize environment
    let env = Environment::new();

    // TEST: Call shell_loop but also add periodic debug output
    test_shell_loop(env);
}

/// 测试版本的shell循环，添加定期调试输出
fn test_shell_loop(env: Environment) {
    let mut loop_count = 0;

    loop {
        loop_count += 1;

        if loop_count % 10 == 0 {
            println!("[SHELL] Loop count: {}", loop_count);
        }

        // Call the original shell_loop but with a limit
        if loop_count < 5 {
            // Try to read a command
            let mut line_buffer = [0u8; 256];
            let mut line_pos = 0;

            print_prompt();

            // Try to read with a simple non-blocking approach
            // Just attempt once and continue
            if let Some(_cmd) = read_line_once(&mut line_buffer, &mut line_pos) {
                // Got input, execute command
                // For now just echo
                println!("[SHELL] Got command, would execute");
            } else {
                // No input, just continue
                println!(""); // New line
            }
        } else {
            // After a few attempts, just yield
            hnxlib::syscall::yield_cpu();
        }
    }
}

/// 尝试读取一行，非阻塞方式
fn read_line_once<'a>(buffer: &'a mut [u8], pos: &mut usize) -> Option<&'a str> {
    *pos = 0;

    // Try to read one character
    let mut ch_buf = [0u8; 1];
    let result = hnxlib::syscall::read(0, &mut ch_buf);

    if result > 0 {
        // Got a character
        buffer[0] = ch_buf[0];
        *pos = 1;

        // If it's newline, return empty string
        if ch_buf[0] == b'\n' {
            Some("")
        } else {
            // For now just return single character as string
            Some(core::str::from_utf8(&buffer[0..1]).unwrap_or("?"))
        }
    } else {
        None
    }
}

fn shell_loop(env: Environment) {
    let mut line_buffer = [0u8; 256];
    let mut line_pos = 0;

    loop {
        // Display prompt
        print_prompt();

        // Read a line of input
        if let Some(cmd) = read_line(&mut line_buffer, &mut line_pos) {
            execute_command(cmd, &env);
        } else {
            // No input or error, just continue
            println!(""); // New line after prompt
        }
    }
}

fn print_prompt() {
    print!("hnx> ");
}

fn read_line<'a>(buffer: &'a mut [u8], pos: &mut usize) -> Option<&'a str> {
    // Reset buffer position
    *pos = 0;

    loop {
        // Try to read a single character
        let mut ch = [0u8; 1];
        let result = syscall::read(0, &mut ch); // stdin is fd 0

        if result == 1 {
            // Character read successfully
            let c = ch[0];

            // Handle special characters
            match c {
                b'\n' | b'\r' => {
                    // End of line
                    if *pos > 0 {
                        // Return the line
                        if let Ok(line) = core::str::from_utf8(&buffer[..*pos]) {
                            return Some(line);
                        }
                    }
                    return None;
                }
                b'\x7f' | b'\x08' => { // Backspace or delete
                    if *pos > 0 {
                        *pos -= 1;
                        // Echo backspace (move cursor back, space, move cursor back)
                        print!("\x08 \x08");
                    }
                }
                _ => {
                    // Regular character
                    if *pos < buffer.len() - 1 && c >= 32 && c <= 126 {
                        // Printable ASCII character
                        buffer[*pos] = c;
                        *pos += 1;
                        // Echo character
                        print!("{}", c as char);
                    }
                }
            }
        } else if result == 0 {
            // No input available, yield and try again
            syscall::yield_cpu();
        } else {
            // Error reading, just yield
            syscall::yield_cpu();
        }
    }
}

/// 通过IPC调用echo-service
fn call_echo_service(text: &str) -> Result<&'static str, IpcError> {
    // 连接到IPC路由器
    let router = IpcRouterClient::connect()?;

    // 查找echo-service
    let echo_endpoint = router.find_service("echo")?;

    // 准备请求数据
    let request_data = text.as_bytes();
    let mut response_buf = [0u8; 256];

    // 发送echo请求（操作码100）
    match echo_endpoint.call(100, request_data, &mut response_buf) {
        Ok((_, len)) => {
            // 成功获取响应
            // 注意：我们不能从字节创建新的字符串，所以使用静态字符串
            // 这是一个临时解决方案，实际应该使用缓冲区
            static mut RESPONSE: [u8; 256] = [0; 256];
            unsafe {
                RESPONSE[..len].copy_from_slice(&response_buf[..len]);
                match core::str::from_utf8(&RESPONSE[..len]) {
                    Ok(s) => Ok(s),
                    Err(_) => Ok("<invalid-utf8>"),
                }
            }
        }
        Err(e) => Err(e),
    }
}

/// 尝试通过环境变量运行服务
fn run_service_from_env(name: &str, args: &str, env: &Environment) -> Result<(), ()> {
    // 查找可执行文件
    if let Some(executable_path) = env.find_executable(name) {
        // 尝试通过spawn_service运行
        // 注意：spawn_service不支持参数，我们只传递可执行路径
        let pid = syscall::spawn_service(executable_path);
        if pid > 0 {
            println!("[Started {} with PID {}]", name, pid);
            Ok(())
        } else {
            println!("Failed to start {}: error {}", name, pid);
            Err(())
        }
    } else {
        Err(())
    }
}

fn execute_command(cmd: &str, env: &Environment) {
    let cmd = cmd.trim();

    // 分割命令和参数（不使用Vec）
    let mut parts = [""; 16]; // 最多16个参数
    let mut part_count = 0;
    let mut in_whitespace = true;
    let mut start_idx = 0;

    for (i, c) in cmd.char_indices() {
        if c.is_whitespace() {
            if !in_whitespace {
                // 单词结束
                if part_count < parts.len() {
                    parts[part_count] = &cmd[start_idx..i];
                    part_count += 1;
                }
                in_whitespace = true;
            }
        } else {
            if in_whitespace {
                // 单词开始
                start_idx = i;
                in_whitespace = false;
            }
        }
    }

    // 处理最后一个单词
    if !in_whitespace && part_count < parts.len() {
        parts[part_count] = &cmd[start_idx..];
        part_count += 1;
    }

    if part_count == 0 {
        println!("");
        return;
    }

    let command_name = parts[0];
    let args = if part_count > 1 {
        // 重建参数字符串（从第二个词开始）
        let mut args_str = "";
        for i in 1..part_count {
            if i == 1 {
                args_str = parts[i];
            } else {
                // 注意：我们无法动态拼接字符串，所以只取第一个参数
                // 这是一个简化版本
                args_str = parts[i]; // 只取最后一个参数
            }
        }
        args_str
    } else {
        ""
    };

    match command_name {
        "help" => {
            println!("\nAvailable commands:");
            println!("  help     - Display this help message");
            println!("  echo     - Echo arguments back (via echo-service)");
            println!("  version  - Display shell version");
            println!("  exit     - Exit shell (not implemented)");
            println!("  env      - Show environment variables");
            println!("");
        }
        "version" => {
            println!("\nHNX Shell v0.1.0");
            println!("Built for HNX Microkernel v0.3.0-alpha.1\n");
        }
        "env" => {
            println!("\nEnvironment variables:");
            println!("  PATH={}", env.get_path());
            println!("  HOME={}", env.home);
            println!("  SHELL={}", env.shell);
            println!("");
        }
        "echo" => {
            if args.is_empty() {
                println!("\necho: missing argument");
                println!("Usage: echo <text>\n");
            } else {
                // 尝试通过echo-service执行
                match call_echo_service(args) {
                    Ok(response) => {
                        println!("\n{}\n", response);
                    }
                    Err(_e) => {
                        // IPC失败，使用内置echo作为后备
                        println!("\n[Warning: echo-service not available, using built-in echo]");
                        println!("{}\n", args);
                    }
                }
            }
        }
        "exit" => {
            println!("\nExit command not yet implemented\n");
        }
        _ => {
            // 尝试从环境变量运行服务
            match run_service_from_env(command_name, args, env) {
                Ok(()) => {
                    // 服务启动成功
                }
                Err(()) => {
                    println!("\nUnknown command: '{}'", command_name);
                    println!("Type 'help' for available commands\n");
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n[shell] PANIC: {}", info);

    if let Some(location) = info.location() {
        println!("[shell] at {}:{}:{}",
                location.file(),
                location.line(),
                location.column());
    }

    println!("[shell] Shell halted");
    loop {
        syscall::yield_cpu();
    }
}
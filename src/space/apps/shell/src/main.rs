#![no_std]
#![no_main]

use core::panic::PanicInfo;
use hnxlib::{println, print, syscall};
use hnx_abi::*;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();
    // Never return
    loop {
        syscall::yield_cpu();
    }
}

fn main() {
    println!("\n========================================");
    println!("        HNX Shell v0.1.0");
    println!("========================================");
    println!("Type 'help' for available commands");
    println!("Press Enter to execute command\n");

    shell_loop();
}

fn shell_loop() {
    let mut line_buffer = [0u8; 256];
    let mut line_pos = 0;

    loop {
        // Display prompt
        print_prompt();

        // Read a line of input
        if let Some(cmd) = read_line(&mut line_buffer, &mut line_pos) {
            execute_command(cmd);
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

fn execute_command(cmd: &str) {
    let cmd = cmd.trim();

    match cmd {
        "help" => {
            println!("\nAvailable commands:");
            println!("  help     - Display this help message");
            println!("  echo     - Echo arguments back");
            println!("  version  - Display shell version");
            println!("  exit     - Exit shell (not implemented)");
            println!("");
        }
        "version" => {
            println!("\nHNX Shell v0.1.0");
            println!("Built for HNX Microkernel v0.3.0-alpha.1\n");
        }
        "" => {
            // Empty command, do nothing
            println!("");
        }
        cmd if cmd.starts_with("echo ") => {
            let args = &cmd[5..];
            println!("\n{}\n", args);
        }
        "echo" => {
            println!("\necho: missing argument");
            println!("Usage: echo <text>\n");
        }
        "exit" => {
            println!("\nExit command not yet implemented\n");
        }
        _ => {
            println!("\nUnknown command: '{}'", cmd);
            println!("Type 'help' for available commands\n");
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
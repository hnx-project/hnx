//! 调试命令解析器

#![no_std]

use super::{DebugResult, DebugError};
use heapless::String;

/// 支持的调试命令枚举
#[derive(Debug, PartialEq)]
pub enum Command {
    /// 显示内存状态
    MemoryStatus,
    /// 显示进程列表
    ProcessList,
    /// 显示特定进程信息
    ProcessInfo(u32),
    /// 显示帮助信息
    Help,
}

/// 解析调试命令字符串
pub fn parse_command(input: &str) -> DebugResult<Command> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        let mut err_msg = String::<128>::new();
        err_msg.push_str("Empty command").unwrap_or(());
        return Err(DebugError::ParseError(err_msg));
    }
    
    // 手动分割字符串，因为no_std环境中没有split_whitespace
    let mut parts: [&str; 3] = [""; 3];
    let mut part_count = 0;
    let mut start = 0;
    let mut in_word = false;
    
    for (i, ch) in trimmed.chars().enumerate() {
        if ch != ' ' && !in_word {
            start = i;
            in_word = true;
        } else if ch == ' ' && in_word {
            if part_count < 3 {
                parts[part_count] = &trimmed[start..i];
                part_count += 1;
            }
            in_word = false;
        }
    }
    
    // 处理最后一个词
    if in_word && part_count < 3 {
        parts[part_count] = &trimmed[start..];
        part_count += 1;
    }
    
    if part_count == 0 {
        let mut err_msg = String::<128>::new();
        err_msg.push_str("Empty command").unwrap_or(());
        return Err(DebugError::ParseError(err_msg));
    }
    
    match parts[0].to_lowercase().as_str() {
        "memstat" => Ok(Command::MemoryStatus),
        "ps" => {
            if part_count == 1 {
                Ok(Command::ProcessList)
            } else if part_count == 2 {
                // 手动解析数字，因为no_std环境中没有parse
                let mut num = 0u32;
                let mut valid = true;
                for ch in parts[1].chars() {
                    if ch >= '0' && ch <= '9' {
                        num = num * 10 + (ch as u32 - '0' as u32);
                    } else {
                        valid = false;
                        break;
                    }
                }
                
                if valid {
                    Ok(Command::ProcessInfo(num))
                } else {
                    let mut err_msg = String::<128>::new();
                    err_msg.push_str("Invalid PID: ").unwrap_or(());
                    err_msg.push_str(parts[1]).unwrap_or(());
                    Err(DebugError::ParseError(err_msg))
                }
            } else {
                let mut err_msg = String::<128>::new();
                err_msg.push_str("Too many arguments for ps command").unwrap_or(());
                Err(DebugError::ParseError(err_msg))
            }
        },
        "help" | "?" => Ok(Command::Help),
        _ => {
            let mut err_msg = String::<128>::new();
            err_msg.push_str("Unknown command: ").unwrap_or(());
            err_msg.push_str(parts[0]).unwrap_or(());
            Err(DebugError::ParseError(err_msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memstat() {
        assert_eq!(parse_command("memstat").unwrap(), Command::MemoryStatus);
        assert_eq!(parse_command("  memstat  ").unwrap(), Command::MemoryStatus);
    }

    #[test]
    fn test_parse_ps() {
        assert_eq!(parse_command("ps").unwrap(), Command::ProcessList);
        assert_eq!(parse_command("ps 123").unwrap(), Command::ProcessInfo(123));
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_command("help").unwrap(), Command::Help);
        assert_eq!(parse_command("?").unwrap(), Command::Help);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_command("").is_err());
        assert!(parse_command("invalid").is_err());
        assert!(parse_command("ps abc").is_err());
        assert!(parse_command("ps 1 2 3").is_err());
    }
}
//! 调试模块测试套件

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::{command_parser, memory_monitor, process_monitor};

    #[test]
    fn test_command_parsing() {
        // 测试内存状态命令解析
        assert_eq!(
            command_parser::parse_command("memstat").unwrap(),
            command_parser::Command::MemoryStatus
        );

        // 测试进程列表命令解析
        assert_eq!(
            command_parser::parse_command("ps").unwrap(),
            command_parser::Command::ProcessList
        );

        // 测试特定进程信息命令解析
        assert_eq!(
            command_parser::parse_command("ps 123").unwrap(),
            command_parser::Command::ProcessInfo(123)
        );

        // 测试帮助命令解析
        assert_eq!(
            command_parser::parse_command("help").unwrap(),
            command_parser::Command::Help
        );
        assert_eq!(
            command_parser::parse_command("?").unwrap(),
            command_parser::Command::Help
        );
    }

    #[test]
    fn test_invalid_commands() {
        // 测试无效命令
        assert!(command_parser::parse_command("").is_err());
        assert!(command_parser::parse_command("invalid").is_err());
        assert!(command_parser::parse_command("ps abc").is_err());
        assert!(command_parser::parse_command("ps 1 2 3").is_err());
    }

    #[test]
    fn test_memory_status() {
        // 测试内存状态获取
        let result = memory_monitor::get_memory_status();
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.contains("Memory Status"));
    }

    #[test]
    fn test_process_list() {
        // 测试进程列表获取
        let result = process_monitor::get_process_list();
        assert!(result.is_ok());
        let list = result.unwrap();
        assert!(list.contains("PID"));
        assert!(list.contains("NAME"));
    }

    #[test]
    fn test_process_info() {
        // 测试特定进程信息获取
        let result = process_monitor::get_process_info(1);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.contains("Process Information"));
    }
}
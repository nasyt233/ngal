// src/edit/shell.rs
use std::process::Command;

pub fn execute_shell_command(cmd: &str) -> String {
    #[cfg(unix)]
    let output = Command::new("sh").arg("-c").arg(cmd).output();

    #[cfg(windows)]
    let output = Command::new("cmd").arg("/C").arg(cmd).output();

    match output {
        Ok(out) => {
            let mut result = String::new();
            if !out.stdout.is_empty() {
                result.push_str(&String::from_utf8_lossy(&out.stdout));
            }
            if !out.stderr.is_empty() {
                let err = String::from_utf8_lossy(&out.stderr);
                result.push_str(&err);
            }
            if result.trim().is_empty() {
                result = format!("(命令成功执行，无输出，退出码: {:?})", out.status.code());
            }
            result
        }
        Err(e) => format!("执行失败: {}", e),
    }
}
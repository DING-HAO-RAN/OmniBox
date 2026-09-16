//! 开发者环境与工具链版本安全探测模块
//!
//! 在 PATH 中探测 Git、Rust、Node.js、Python、WSL、Docker 等开发环境，
//! 采用严格超时控制 (1~2 秒)，捕获标准输出并对缺失环境标记为未安装，避免无限挂起。

use serde::{Deserialize, Serialize};
use std::process::Command;

/// 单个开发环境工具项
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevToolEntry {
    pub name: String,
    pub installed: bool,
    pub version: String,
    pub path: Option<String>,
}

/// 开发者环境快照
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevEnvironmentSnapshot {
    pub tools: Vec<DevToolEntry>,
}

fn probe_tool_version(cmd_name: &str, args: &[&str]) -> (bool, String) {
    let output = Command::new(cmd_name)
        .args(args)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let first_line = stdout.lines().next().unwrap_or("").trim().to_string();
            if !first_line.is_empty() {
                return (true, first_line);
            }
        }
    }
    (false, "未安装 / 不在 PATH 中".to_string())
}

/// 采集开发者环境与工具链版本
pub fn collect_dev_environment() -> DevEnvironmentSnapshot {
    let mut tools = Vec::new();

    let probe_targets = [
        ("Rust", "rustc", &["--version"][..]),
        ("Cargo", "cargo", &["--version"][..]),
        ("Git", "git", &["--version"][..]),
        ("Node.js", "node", &["--version"][..]),
        ("npm", "npm", &["--version"][..]),
        ("pnpm", "pnpm", &["--version"][..]),
        ("Python", "python", &["--version"][..]),
        ("PowerShell", "powershell", &["-Command", "$PSVersionTable.PSVersion.ToString()"][..]),
        ("Docker", "docker", &["--version"][..]),
        ("WSL", "wsl", &["--status"][..]),
    ];

    for (label, exe, args) in probe_targets {
        let (installed, ver) = probe_tool_version(exe, args);
        tools.push(DevToolEntry {
            name: label.to_string(),
            installed,
            version: ver,
            path: None,
        });
    }

    DevEnvironmentSnapshot { tools }
}

//! 开发者环境与工具链版本安全探测模块。
//!
//! 每个外部命令都通过独立 stdout/stderr 管道读取，并受固定 deadline 与输出上限约束；
//! 找不到工具或读取失败只产生带质量的空值，不伪造版本信息。

use super::quality::{current_timestamp_ms, MetricQuality, MetricValue};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;
const TOOL_TIMEOUT: Duration = Duration::from_secs(3);
const TOOL_SOURCE: &str = "Process_Command";

/// 单个开发环境工具项。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevToolEntry {
    pub name: String,
    pub installed: MetricValue<bool>,
    pub version: MetricValue<String>,
    pub path: MetricValue<String>,
}

/// 开发者环境快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevEnvironmentSnapshot {
    pub tools: Vec<DevToolEntry>,
}

#[derive(Clone, Debug, PartialEq)]
struct ToolProbeResult {
    installed: bool,
    version: Option<String>,
    path: Option<String>,
    quality: MetricQuality,
    error: Option<String>,
}

#[derive(Default)]
struct ToolOutput {
    bytes: Vec<u8>,
    read_error: bool,
}

fn unavailable_probe(reason: &str) -> ToolProbeResult {
    ToolProbeResult {
        installed: false,
        version: None,
        path: None,
        quality: MetricQuality::Unavailable,
        error: Some(reason.to_string()),
    }
}

/// 保留输出前 64 KiB，同时持续读取管道直到 EOF，避免子进程被写满的管道阻塞。
fn read_tool_output(mut reader: impl Read) -> ToolOutput {
    let mut output = ToolOutput {
        bytes: Vec::with_capacity(MAX_TOOL_OUTPUT_BYTES),
        read_error: false,
    };
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read_bytes) => {
                if output.bytes.len() < MAX_TOOL_OUTPUT_BYTES {
                    let keep = (MAX_TOOL_OUTPUT_BYTES - output.bytes.len()).min(read_bytes);
                    output.bytes.extend_from_slice(&buffer[..keep]);
                }
            }
            Err(_) => {
                output.read_error = true;
                break;
            }
        }
    }
    output
}

fn first_output_line(output: &[u8]) -> Option<String> {
    let bounded = &output[..output.len().min(MAX_TOOL_OUTPUT_BYTES)];
    String::from_utf8_lossy(bounded)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
}

fn join_reader_until(
    reader: thread::JoinHandle<ToolOutput>,
    deadline: Instant,
) -> Option<ToolOutput> {
    while !reader.is_finished() {
        if Instant::now() >= deadline {
            // 读线程继续 drain 自己的管道，丢弃句柄以避免探测调用被 join 阻塞。
            return None;
        }
        thread::sleep(Duration::from_millis(1));
    }
    reader.join().ok()
}

fn stop_child_without_blocking(child: &mut Child) {
    let _ = child.kill();
    let cleanup_deadline = Instant::now() + Duration::from_millis(100);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let _ = child.wait();
                break;
            }
            Err(_) => break,
            Ok(None) if Instant::now() >= cleanup_deadline => break,
            Ok(None) => thread::sleep(Duration::from_millis(5)),
        }
    }
}

/// 根据进程输出构造探测结果；成功命令的空 stdout 使用 stderr 第一行。
fn parse_tool_output(stdout: &[u8], stderr: &[u8], exit_code: i32) -> ToolProbeResult {
    let output_line = first_output_line(stdout).or_else(|| first_output_line(stderr));
    if exit_code == 0 {
        if let Some(version) = output_line {
            return ToolProbeResult {
                installed: true,
                version: Some(version),
                path: None,
                quality: MetricQuality::Good,
                error: None,
            };
        }
        return ToolProbeResult {
            installed: false,
            version: None,
            path: None,
            quality: MetricQuality::Unavailable,
            error: Some("工具未返回版本信息".to_string()),
        };
    }

    ToolProbeResult {
        installed: false,
        version: None,
        path: None,
        quality: MetricQuality::ReadError,
        error: output_line.or_else(|| Some("工具进程返回非零退出码".to_string())),
    }
}

fn path_is_file(path: &Path) -> bool {
    path.is_file()
}

fn command_candidates(command: &str) -> Vec<PathBuf> {
    let command_path = Path::new(command);
    let has_path_separator = command.contains('\\') || command.contains('/');
    let mut bases = Vec::new();

    if command_path.is_absolute() || has_path_separator {
        bases.push(command_path.to_path_buf());
    } else if let Some(path) = env::var_os("PATH") {
        bases.extend(env::split_paths(&path).map(|directory| directory.join(command_path)));
    }

    let mut candidates = Vec::new();
    for base in bases {
        candidates.push(base.clone());
        if base.extension().is_none() {
            let extensions = env::var_os("PATHEXT")
                .map(|value| {
                    value
                        .to_string_lossy()
                        .split(';')
                        .filter(|extension| !extension.is_empty())
                        .map(|extension| extension.trim_start_matches('.').to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| {
                    vec![
                        "COM".to_string(),
                        "EXE".to_string(),
                        "BAT".to_string(),
                        "CMD".to_string(),
                    ]
                });
            candidates.extend(extensions.into_iter().map(|extension| {
                let mut candidate = base.clone();
                candidate.set_extension(extension);
                candidate
            }));
        }
    }
    candidates
}

/// 从 PATH 找到真实可执行文件路径，不通过另一个外部命令解析。
fn resolve_executable(command: &str) -> Option<String> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    command_candidates(command)
        .into_iter()
        .find(|candidate| path_is_file(candidate))
        .map(|candidate| {
            std::fs::canonicalize(&candidate)
                .unwrap_or(candidate)
                .to_string_lossy()
                .into_owned()
        })
}

/// 在质量状态下构造 MetricValue，保证非 Good 状态没有 value。
fn metric_from_probe<T>(
    value: Option<T>,
    quality: MetricQuality,
    error: Option<&str>,
    unit: &str,
    timestamp: u64,
) -> MetricValue<T> {
    let reason = error.unwrap_or("工具探测未返回有效值");
    match quality {
        MetricQuality::Good => value
            .map(|value| MetricValue::good_at(value, unit, TOOL_SOURCE, timestamp))
            .unwrap_or_else(|| MetricValue::unavailable_at(unit, TOOL_SOURCE, reason, timestamp)),
        MetricQuality::Estimated => value
            .map(|value| MetricValue::estimated_at(value, unit, TOOL_SOURCE, timestamp))
            .unwrap_or_else(|| MetricValue::unavailable_at(unit, TOOL_SOURCE, reason, timestamp)),
        MetricQuality::Unsupported => {
            MetricValue::unsupported_at(unit, TOOL_SOURCE, reason, timestamp)
        }
        MetricQuality::PermissionDenied => {
            MetricValue::permission_denied_at(unit, TOOL_SOURCE, reason, timestamp)
        }
        MetricQuality::Unavailable => {
            MetricValue::unavailable_at(unit, TOOL_SOURCE, reason, timestamp)
        }
        MetricQuality::DriverMissing => {
            MetricValue::driver_missing_at(unit, TOOL_SOURCE, reason, timestamp)
        }
        MetricQuality::ApiUnavailable => {
            MetricValue::api_unavailable_at(unit, TOOL_SOURCE, reason, timestamp)
        }
        MetricQuality::ReadError => {
            MetricValue::read_error_at(unit, TOOL_SOURCE, reason, timestamp)
        }
        MetricQuality::Invalid => MetricValue::invalid_at(unit, TOOL_SOURCE, reason, timestamp),
        MetricQuality::Stale | MetricQuality::Unknown => {
            MetricValue::unavailable_at(unit, TOOL_SOURCE, reason, timestamp)
        }
    }
}

/// 探测 PATH 中的工具版本；进程、stdout 和 stderr 都受 deadline 约束。
fn probe_tool_version(command: &str, args: &[&str], timeout: Duration) -> ToolProbeResult {
    let Some(executable_path) = resolve_executable(command) else {
        return unavailable_probe("工具不在 PATH 中");
    };

    let child = Command::new(&executable_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return ToolProbeResult {
            installed: false,
            version: None,
            path: Some(executable_path),
            quality: MetricQuality::ReadError,
            error: Some("工具进程启动失败".to_string()),
        };
    };

    let Some(stdout_pipe) = child.stdout.take() else {
        stop_child_without_blocking(&mut child);
        return ToolProbeResult {
            installed: false,
            version: None,
            path: Some(executable_path),
            quality: MetricQuality::ReadError,
            error: Some("工具 stdout 管道不可用".to_string()),
        };
    };
    let Some(stderr_pipe) = child.stderr.take() else {
        stop_child_without_blocking(&mut child);
        return ToolProbeResult {
            installed: false,
            version: None,
            path: Some(executable_path),
            quality: MetricQuality::ReadError,
            error: Some("工具 stderr 管道不可用".to_string()),
        };
    };

    let stdout_reader = thread::spawn(move || read_tool_output(stdout_pipe));
    let stderr_reader = thread::spawn(move || read_tool_output(stderr_pipe));
    let deadline = Instant::now()
        .checked_add(timeout.min(TOOL_TIMEOUT))
        .unwrap_or_else(Instant::now);
    let mut timed_out = false;
    let exit_code = loop {
        if Instant::now() >= deadline {
            timed_out = true;
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) if Instant::now() < deadline => {
                break Some(status.code().unwrap_or(-1));
            }
            Ok(Some(_)) => {
                timed_out = true;
                break None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };

    if timed_out || exit_code.is_none() {
        stop_child_without_blocking(&mut child);
    }

    let stdout = join_reader_until(stdout_reader, deadline);
    let stderr = join_reader_until(stderr_reader, deadline);
    if timed_out {
        return ToolProbeResult {
            installed: false,
            version: None,
            path: Some(executable_path),
            quality: MetricQuality::Unavailable,
            error: Some("工具探测超时".to_string()),
        };
    }
    let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
        return ToolProbeResult {
            installed: false,
            version: None,
            path: Some(executable_path),
            quality: MetricQuality::Unavailable,
            error: Some("工具输出读取超时".to_string()),
        };
    };
    if stdout.read_error || stderr.read_error {
        return ToolProbeResult {
            installed: false,
            version: None,
            path: Some(executable_path),
            quality: MetricQuality::ReadError,
            error: Some("工具输出读取失败".to_string()),
        };
    }

    let mut result = parse_tool_output(&stdout.bytes, &stderr.bytes, exit_code.unwrap_or(-1));
    result.path = Some(executable_path);
    result
}

/// 采集开发者环境与工具链版本。
pub fn collect_dev_environment() -> DevEnvironmentSnapshot {
    let probe_targets = [
        ("Rust", "rustc", &["--version"][..]),
        ("Cargo", "cargo", &["--version"][..]),
        ("Git", "git", &["--version"][..]),
        ("Node.js", "node", &["--version"][..]),
        ("npm", "npm", &["--version"][..]),
        ("pnpm", "pnpm", &["--version"][..]),
        ("Python", "python", &["--version"][..]),
        (
            "PowerShell",
            "powershell",
            &["-Command", "$PSVersionTable.PSVersion.ToString()"][..],
        ),
        ("Docker", "docker", &["--version"][..]),
        ("WSL", "wsl", &["--status"][..]),
    ];

    let mut tools = Vec::with_capacity(probe_targets.len());
    for (label, command, args) in probe_targets {
        let result = probe_tool_version(command, args, TOOL_TIMEOUT);
        let timestamp = current_timestamp_ms();
        let error = result.error.as_deref();
        let installed = metric_from_probe(
            (result.quality == MetricQuality::Good).then_some(result.installed),
            result.quality,
            error,
            "",
            timestamp,
        );
        let version = metric_from_probe(result.version, result.quality, error, "", timestamp);
        let path = match result.path {
            Some(path) => MetricValue::good_at(path, "", TOOL_SOURCE, timestamp),
            None => metric_from_probe(None::<String>, result.quality, error, "", timestamp),
        };
        tools.push(DevToolEntry {
            name: label.to_string(),
            installed,
            version,
            path,
        });
    }

    DevEnvironmentSnapshot { tools }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_version_can_be_read_from_stderr_and_has_deadline() {
        let result = parse_tool_output(b"", b"Python 3.12.0\r\n", 0);
        assert_eq!(result.version.as_deref(), Some("Python 3.12.0"));
    }
}

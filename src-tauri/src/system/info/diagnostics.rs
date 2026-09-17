//! 系统健康诊断、WHEA 硬件事件与蓝屏崩溃 Dump 分析模块。
//!
//! 仅扫描 SystemRoot 下的 Minidump 文件 metadata；没有 Event Log Provider 时，事件集合
//! 保持为空并以 Unsupported 指标表达能力边界，不读取 dump 内容或任何密钥。

use super::quality::{current_timestamp_ms, CollectionStatus, MetricQuality, MetricValue};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const MAX_DUMP_ENTRIES: usize = 100;
const MINIDUMP_SOURCE: &str = "Windows_Minidump_Metadata";
const EVENT_LOG_SOURCE: &str = "Windows_EventLog";

/// 蓝屏 Dump 文件概要。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CrashDumpEntry {
    pub file_name: String,
    pub file_size_bytes: u64,
    pub created_at: String,
}

/// WHEA / 硬件系统事件摘要。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HardwareEventSummary {
    pub provider: String,
    pub event_id: u32,
    pub severity: String,
    pub description: String,
}

/// 系统健康与故障诊断快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DiagnosticsSnapshot {
    pub minidump_count: MetricValue<u32>,
    pub recent_crash_dumps: Vec<CrashDumpEntry>,
    pub minidump_truncated: bool,
    pub unexpected_shutdowns_count: MetricValue<u32>,
    pub whea_hardware_events: Vec<HardwareEventSummary>,
    pub whea_hardware_events_status: CollectionStatus,
    pub overall_health_assessment: MetricValue<String>,
}

/// 从真实 metadata 创建 dump 条目，时间以十进制 Unix 毫秒保留。
fn dump_entry_from_metadata(file_name: &str, size: u64, modified_at_ms: u64) -> CrashDumpEntry {
    CrashDumpEntry {
        file_name: file_name.to_string(),
        file_size_bytes: size,
        created_at: modified_at_ms.to_string(),
    }
}

fn minidump_path() -> Option<PathBuf> {
    let system_root = env::var_os("SystemRoot")?;
    if system_root.to_string_lossy().trim().is_empty() {
        return None;
    }
    Some(PathBuf::from(system_root).join("Minidump"))
}

fn minidump_error_metric<T>(kind: std::io::ErrorKind, timestamp: u64) -> MetricValue<T> {
    match kind {
        std::io::ErrorKind::NotFound => MetricValue::unavailable_at(
            "",
            MINIDUMP_SOURCE,
            "SystemRoot\u{005c}Minidump 不存在",
            timestamp,
        ),
        std::io::ErrorKind::PermissionDenied => MetricValue::permission_denied_at(
            "",
            MINIDUMP_SOURCE,
            "读取 SystemRoot Minidump 目录权限不足",
            timestamp,
        ),
        _ => MetricValue::read_error_at(
            "",
            MINIDUMP_SOURCE,
            "读取 SystemRoot Minidump 目录失败",
            timestamp,
        ),
    }
}

/// 扫描 Minidump metadata，返回所有可读文件及是否发生列表截断。
fn scan_minidumps(path: &Path) -> Result<(Vec<(u64, CrashDumpEntry)>, bool), MetricValue<u32>> {
    let mut entries = Vec::new();
    let directory = fs::read_dir(path)
        .map_err(|error| minidump_error_metric(error.kind(), current_timestamp_ms()))?;
    let mut had_metadata_error = false;

    for entry in directory {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                had_metadata_error = true;
                continue;
            }
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                had_metadata_error = true;
                continue;
            }
        };
        if !metadata.is_file() {
            continue;
        }
        let modified_at_ms = match metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        {
            Some(value) => value,
            None => {
                had_metadata_error = true;
                continue;
            }
        };
        let file_name = entry.file_name().to_string_lossy().to_string();
        entries.push((
            modified_at_ms,
            dump_entry_from_metadata(&file_name, metadata.len(), modified_at_ms),
        ));
    }

    if had_metadata_error {
        return Err(MetricValue::read_error_at(
            "",
            MINIDUMP_SOURCE,
            "读取一个或多个 Minidump metadata 失败",
            current_timestamp_ms(),
        ));
    }

    entries.sort_by(|(left_time, left_entry), (right_time, right_entry)| {
        right_time
            .cmp(left_time)
            .then_with(|| left_entry.file_name.cmp(&right_entry.file_name))
    });
    let truncated = entries.len() > MAX_DUMP_ENTRIES;
    Ok((entries, truncated))
}

fn unsupported_event_count(timestamp: u64) -> MetricValue<u32> {
    MetricValue::unsupported_at(
        "",
        EVENT_LOG_SOURCE,
        "未接入 Event Log Provider，无法统计异常关机",
        timestamp,
    )
}

fn unsupported_event_status(timestamp: u64) -> CollectionStatus {
    CollectionStatus {
        quality: MetricQuality::Unsupported,
        source: EVENT_LOG_SOURCE.to_string(),
        timestamp,
        item_count: Some(0),
        truncated: false,
        error: Some("未接入 Event Log Provider，WHEA 事件集合不可用".to_string()),
    }
}

fn unsupported_health(timestamp: u64) -> MetricValue<String> {
    MetricValue::unsupported_at(
        "",
        EVENT_LOG_SOURCE,
        "未接入 Event Log/WHEA Provider，无法评估整体系统健康",
        timestamp,
    )
}

/// 扫描 Minidump 目录与硬件错误事件。
pub fn collect_diagnostics_snapshot() -> DiagnosticsSnapshot {
    let timestamp = current_timestamp_ms();
    let mut recent_crash_dumps = Vec::new();
    let mut minidump_truncated = false;
    let minidump_count = match minidump_path() {
        None => MetricValue::unavailable_at(
            "",
            MINIDUMP_SOURCE,
            "SystemRoot 环境变量不可用，无法定位 Minidump",
            timestamp,
        ),
        Some(path) => match scan_minidumps(&path) {
            Ok((entries, truncated)) => {
                let count = u32::try_from(entries.len()).unwrap_or(u32::MAX);
                minidump_truncated = truncated;
                recent_crash_dumps = entries
                    .into_iter()
                    .take(MAX_DUMP_ENTRIES)
                    .map(|(_, entry)| entry)
                    .collect();
                MetricValue::good_at(count, "", MINIDUMP_SOURCE, timestamp)
            }
            Err(error) => error,
        },
    };

    DiagnosticsSnapshot {
        minidump_count,
        recent_crash_dumps,
        minidump_truncated,
        // 第一阶段不接入 Event Log；空集合不代表没有事件。
        unexpected_shutdowns_count: unsupported_event_count(timestamp),
        whea_hardware_events: Vec::new(),
        whea_hardware_events_status: unsupported_event_status(timestamp),
        overall_health_assessment: unsupported_health(timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_metadata_uses_file_time_not_recent_placeholder() {
        let entry = dump_entry_from_metadata("x.dmp", 42, 1_700_000_000_000);
        assert_eq!(entry.file_size_bytes, 42);
        assert_eq!(entry.created_at, "1700000000000");
    }
}

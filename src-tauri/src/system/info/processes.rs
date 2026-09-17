//! 系统活跃进程与资源占用分析模块
//!
//! Toolhelp32 只负责枚举进程关系与线程数；工作集必须逐进程通过真实
//! `OpenProcess`/`GetProcessMemoryInfo` 读取，任何权限或退出竞态都保留为空值。

use super::collection_status_for;
use super::quality::{
    classify_win32_error, CollectionStatus, CollectorResult, MetricQuality, MetricValue,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

/// 进程摘要信息。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProcessSummaryItem {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub threads: u32,
    pub memory_working_set_bytes: MetricValue<u64>,
}

/// 进程全景快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProcessSnapshot {
    pub total_processes: u32,
    pub total_threads: u32,
    pub top_memory_processes: Vec<ProcessSummaryItem>,
}

const PROCESS_MEMORY_SOURCE: &str = "Win32_GetProcessMemoryInfo";

/// 将进程内存读取失败映射为无值指标，不把线程数转换成内存估算。
fn process_memory_error(error_code: u32) -> MetricValue<u64> {
    let reason = format!("读取进程工作集失败，Win32 错误码 {error_code}");
    match classify_win32_error(error_code) {
        MetricQuality::PermissionDenied => {
            MetricValue::permission_denied("Bytes", PROCESS_MEMORY_SOURCE, &reason)
        }
        MetricQuality::Unavailable => {
            MetricValue::unavailable("Bytes", PROCESS_MEMORY_SOURCE, &reason)
        }
        // 进程退出竞态或未分类的 API 错误均不能生成数值。
        _ => MetricValue::read_error_at(
            "Bytes",
            PROCESS_MEMORY_SOURCE,
            &reason,
            super::quality::current_timestamp_ms(),
        ),
    }
}

/// 在没有可用进程句柄时构造权限降级条目。
#[cfg(test)]
fn process_item_without_handle(pid: u32, threads: u32, name: &str) -> ProcessSummaryItem {
    ProcessSummaryItem {
        pid,
        ppid: 0,
        name: name.to_string(),
        threads,
        memory_working_set_bytes: MetricValue::permission_denied(
            "Bytes",
            PROCESS_MEMORY_SOURCE,
            "OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) 被拒绝访问",
        ),
    }
}

/// 读取单个进程的真实 Working Set。
fn read_process_working_set(pid: u32) -> MetricValue<u64> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return process_memory_error(GetLastError());
        }

        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = GetProcessMemoryInfo(process, &mut counters, counters.cb) != 0;
        if ok {
            let working_set = counters.WorkingSetSize as u64;
            // 无论读取是否成功，已打开的句柄都必须关闭。
            CloseHandle(process);
            MetricValue::good(working_set, "Bytes", PROCESS_MEMORY_SOURCE)
        } else {
            let error_code = GetLastError();
            CloseHandle(process);
            process_memory_error(error_code)
        }
    }
}

const PROCESS_DISPLAY_LIMIT: usize = 25;
const PROCESS_SOURCE: &str = "Win32_Toolhelp32";

fn process_status_for(
    total_processes: u32,
    returned_items: usize,
    quality: MetricQuality,
    error: Option<&str>,
) -> CollectionStatus {
    let mut status = collection_status_for(
        PROCESS_SOURCE,
        quality,
        total_processes as usize,
        total_processes as usize > returned_items,
        error,
    );
    // 进程状态的 item_count 始终表示已枚举总数，即使后续 API 失败。
    status.item_count = Some(total_processes);
    status
}

/// 采集系统当前运行进程快照并保留枚举 API 状态。
pub(crate) fn collect_processes_snapshot_with_status() -> CollectorResult<ProcessSnapshot> {
    let mut list = Vec::new();
    let mut total_procs = 0u32;
    let mut total_threads = 0u32;
    let mut quality = MetricQuality::Good;
    let mut error = None;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            let error_code = GetLastError();
            let quality = if error_code == 0 {
                MetricQuality::ApiUnavailable
            } else {
                classify_win32_error(error_code)
            };
            let status = process_status_for(
                0,
                0,
                quality,
                Some(&format!(
                    "Toolhelp32 process snapshot failed (Win32 error {error_code})"
                )),
            );
            return CollectorResult {
                value: ProcessSnapshot {
                    total_processes: 0,
                    total_threads: 0,
                    top_memory_processes: list,
                },
                status,
            };
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                total_procs = total_procs.checked_add(1).unwrap_or(u32::MAX);
                total_threads = total_threads
                    .checked_add(entry.cntThreads)
                    .unwrap_or(u32::MAX);

                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..name_len])
                    .trim()
                    .to_string();
                let memory_working_set_bytes = read_process_working_set(entry.th32ProcessID);

                list.push(ProcessSummaryItem {
                    pid: entry.th32ProcessID,
                    ppid: entry.th32ParentProcessID,
                    name,
                    threads: entry.cntThreads,
                    memory_working_set_bytes,
                });

                if Process32NextW(snapshot, &mut entry) == 0 {
                    let error_code = GetLastError();
                    if error_code != windows_sys::Win32::Foundation::ERROR_NO_MORE_FILES {
                        quality = if error_code == 0 {
                            MetricQuality::ApiUnavailable
                        } else {
                            classify_win32_error(error_code)
                        };
                        error = Some(format!(
                            "Toolhelp32 process continuation failed (Win32 error {error_code})"
                        ));
                    }
                    break;
                }
            }
        } else {
            let error_code = GetLastError();
            if error_code != windows_sys::Win32::Foundation::ERROR_NO_MORE_FILES {
                quality = classify_win32_error(error_code);
                error = Some(format!(
                    "Toolhelp32 process enumeration failed (Win32 error {error_code})"
                ));
            }
        }
        CloseHandle(snapshot);
    }

    list.sort_by(|a, b| {
        match (
            a.memory_working_set_bytes.value,
            b.memory_working_set_bytes.value,
        ) {
            (Some(a_memory), Some(b_memory)) => {
                b_memory.cmp(&a_memory).then_with(|| a.pid.cmp(&b.pid))
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.pid.cmp(&b.pid),
        }
    });

    let returned_items = list.len().min(PROCESS_DISPLAY_LIMIT);
    let status = process_status_for(total_procs, returned_items, quality, error.as_deref());
    CollectorResult {
        value: ProcessSnapshot {
            total_processes: total_procs,
            total_threads,
            // 列表截断只影响展示列表，状态计数仍保持完整真实值。
            top_memory_processes: list.into_iter().take(PROCESS_DISPLAY_LIMIT).collect(),
        },
        status,
    }
}

/// 保持历史公开签名；聚合层使用带状态的内部结果。
pub fn collect_processes_snapshot() -> ProcessSnapshot {
    collect_processes_snapshot_with_status().value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::info::quality::MetricQuality;

    #[test]
    fn process_memory_is_not_derived_from_thread_count() {
        let item = process_item_without_handle(7, 2, "protected.exe");
        assert_eq!(item.memory_working_set_bytes.value, None);
        assert_eq!(
            item.memory_working_set_bytes.quality,
            MetricQuality::PermissionDenied
        );
    }

    #[test]
    fn process_status_reports_total_count_and_display_truncation() {
        let status = process_status_for(30, 25, MetricQuality::Good, None);
        assert_eq!(status.quality, MetricQuality::Good);
        assert_eq!(status.item_count, Some(30));
        assert!(status.truncated);
        assert!(status.error.is_none());
    }

    #[test]
    fn process_status_keeps_api_failure_non_good_for_empty_collection() {
        let status = process_status_for(
            0,
            0,
            MetricQuality::ReadError,
            Some("Toolhelp32 enumeration failed"),
        );
        assert_ne!(status.quality, MetricQuality::Good);
        assert_eq!(status.item_count, Some(0));
        assert!(status.error.is_some());
    }
}

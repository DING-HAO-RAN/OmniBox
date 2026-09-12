//! Windows 内存监控与工作集优化模块
//!
//! 提供物理内存信息读取 (`GlobalMemoryStatusEx`) 以及基于工作集修剪 (`EmptyWorkingSet`) 的内存优化能力。

use crate::system::types::{CleanResult, MemoryStatus};
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

/// 获取当前系统物理内存的使用状态
///
/// 通过 Windows API `GlobalMemoryStatusEx` 获取全局内存状态，计算已用内存及使用百分比。
pub fn get_memory_info() -> Result<MemoryStatus, String> {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
            ullAvailExtendedVirtual: 0,
        };

        if GlobalMemoryStatusEx(&mut mem_status) == 0 {
            return Err("调用 GlobalMemoryStatusEx 获取物理内存状态失败".to_string());
        }

        let total_ram = mem_status.ullTotalPhys;
        let available_ram = mem_status.ullAvailPhys;
        let used_ram = total_ram.saturating_sub(available_ram);
        let usage_percent = if total_ram > 0 {
            ((used_ram as f64 / total_ram as f64) * 10000.0).round() / 100.0
        } else {
            0.0
        };

        Ok(MemoryStatus {
            total_ram,
            available_ram,
            used_ram,
            usage_percent,
        })
    }
}

/// 扫描并修剪所有可访问进程的工作集，释放物理内存
///
/// 1. 记录清理前的系统内存状态；
/// 2. 使用快照 `CreateToolhelp32Snapshot` 遍历所有进程；
/// 3. 对具备权限的进程调用 `K32EmptyWorkingSet` 修剪无用工作集；
/// 4. 自动捕获并跳过系统保护进程或无权限进程，保障稳定性；
/// 5. 计算优化前后的内存变化并返回 `CleanResult`。
pub fn clean_process_working_sets() -> Result<CleanResult, String> {
    let before_status = get_memory_info()?;

    let mut processes_trimmed: u32 = 0;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err("创建进程快照 (CreateToolhelp32Snapshot) 失败".to_string());
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let pid = entry.th32ProcessID;

                // 跳过 PID 为 0 的 System Idle Process
                if pid != 0 {
                    // 请求查询信息与配额设置权限
                    let process_handle =
                        OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, 0, pid);
                    if !process_handle.is_null() {
                        // 尝试修剪工作集
                        if K32EmptyWorkingSet(process_handle) != 0 {
                            processes_trimmed += 1;
                        }
                        CloseHandle(process_handle);
                    }
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    let after_status = get_memory_info()?;

    let freed_bytes = if after_status.used_ram < before_status.used_ram {
        before_status.used_ram - after_status.used_ram
    } else {
        0
    };

    let freed_mb = ((freed_bytes as f64 / (1024.0 * 1024.0)) * 100.0).round() / 100.0;

    Ok(CleanResult {
        freed_bytes,
        freed_mb,
        processes_trimmed,
        before_usage_percent: before_status.usage_percent,
        after_usage_percent: after_status.usage_percent,
    })
}

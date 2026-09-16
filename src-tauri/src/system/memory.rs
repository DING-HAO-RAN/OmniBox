//! Windows 内存监控与工作集优化模块
//!
//! 提供物理内存信息读取 (`GlobalMemoryStatusEx`) 以及基于工作集修剪 (`EmptyWorkingSet`)
//! 与 Windows NT 原生内核待机列表清空 (`NtSetSystemInformation` Standby List Purge，参考 PCL2 启动器技术路线) 的深度内存优化能力。

use crate::system::types::{CleanResult, MemoryStatus};
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, GetTokenInformation, LookupPrivilegeValueW, TokenElevation,
    LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows_sys::Win32::System::Memory::SetSystemFileCacheSize;
use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

/// 检查当前进程是否以管理员权限 (Elevated) 运行
pub fn is_running_as_admin() -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut ret_len = 0;
        let res = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        res != 0 && elevation.TokenIsElevated != 0
    }
}

/// 启用指定的安全特权（如 SeProfileSingleProcessPrivilege / SeIncreaseQuotaPrivilege）
fn enable_privilege(privilege_name: &str) -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        ) == 0
        {
            return false;
        }

        let name_wide: Vec<u16> = privilege_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut luid: windows_sys::Win32::Foundation::LUID = std::mem::zeroed();
        if LookupPrivilegeValueW(std::ptr::null(), name_wide.as_ptr(), &mut luid) == 0 {
            CloseHandle(token);
            return false;
        }

        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        let result = AdjustTokenPrivileges(
            token,
            0,
            &tp,
            size_of::<TOKEN_PRIVILEGES>() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );

        CloseHandle(token);
        result != 0
    }
}

/// Windows NT 待机列表清理命令类型 (SystemMemoryListInformation)
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
enum SystemMemoryListCommand {
    MemoryCaptureLargePageSizes = 0,
    MemoryFlushModifiedList = 1,
    MemoryPurgeStandbyList = 2,
    MemoryPurgeLowPriorityStandbyList = 3,
    MemoryMakeLowPriorityStandbyList = 4,
    MemoryFlushAndPurgeModifiedList = 5,
    MemoryEmptyWorkingSets = 6,
}

type NtSetSystemInformationFn = unsafe extern "system" fn(
    system_information_class: u32,
    system_information: *const std::ffi::c_void,
    system_information_length: u32,
) -> i32;

/// 执行 PCL2 风格的内核级系统待机列表 (Standby List) 清理
///
/// 机制原理：
/// 1. 提权 `SeProfileSingleProcessPrivilege` 与 `SeIncreaseQuotaPrivilege`；
/// 2. 动态加载 `ntdll.dll` 导出函数 `NtSetSystemInformation`；
/// 3. 执行 `MemoryPurgeStandbyList` (2)，将 Windows 占用的海量待机缓存刷空为全新空闲内存；
/// 4. 执行 `MemoryFlushModifiedList` (1) 清空修改列表；
/// 5. 执行 `SetSystemFileCacheSize` 清理文件缓存工作集。
fn purge_standby_list() -> bool {
    // 首先启用必要的底层特权
    enable_privilege("SeProfileSingleProcessPrivilege");
    enable_privilege("SeIncreaseQuotaPrivilege");

    unsafe {
        let ntdll = GetModuleHandleA(b"ntdll.dll\0".as_ptr());
        if ntdll.is_null() {
            return false;
        }

        let proc = GetProcAddress(ntdll, b"NtSetSystemInformation\0".as_ptr());
        if let Some(nt_set_system_information) = proc {
            let func: NtSetSystemInformationFn = std::mem::transmute(nt_set_system_information);

            // 清理系统修改列表
            let cmd_flush = SystemMemoryListCommand::MemoryFlushModifiedList as u32;
            let _ = func(80, &cmd_flush as *const _ as *const _, 4);

            // 清空待机列表 (Purge Standby List)
            let cmd_purge = SystemMemoryListCommand::MemoryPurgeStandbyList as u32;
            let status = func(80, &cmd_purge as *const _ as *const _, 4);

            // 清理系统文件缓存工作集
            let _ = SetSystemFileCacheSize(usize::MAX, usize::MAX, 0);

            return status == 0;
        }
    }
    false
}

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

/// 扫描并修剪所有可访问进程的工作集，并结合内核待机列表清空 (参考 PCL2 深度优化路线)
///
/// 1. 记录清理前的系统内存状态；
/// 2. 检查并尝试执行内核级待机列表清空 (Standby List Purge)；
/// 3. 使用快照 `CreateToolhelp32Snapshot` 遍历所有进程；
/// 4. 对具备权限的进程调用 `K32EmptyWorkingSet` 修剪无用工作集；
/// 5. 自动捕获并跳过系统保护进程或无权限进程，保障稳定性；
/// 6. 计算优化前后的内存变化并返回详尽的 `CleanResult`。
pub fn clean_process_working_sets() -> Result<CleanResult, String> {
    let before_status = get_memory_info()?;
    let is_admin = is_running_as_admin();

    // 尝试执行 PCL2 级待机列表清空
    let standby_purged = purge_standby_list();

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
                    let process_handle =
                        OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, 0, pid);
                    if !process_handle.is_null() {
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

    let clean_mode = if standby_purged || is_admin {
        "PCL2 深度内核优化 (待机缓存+全工作集)".to_string()
    } else {
        "标准工作集修剪 (以管理员运行可激活内核待机优化)".to_string()
    };

    Ok(CleanResult {
        freed_bytes,
        freed_mb,
        processes_trimmed,
        before_usage_percent: before_status.usage_percent,
        after_usage_percent: after_status.usage_percent,
        standby_freed_bytes: if standby_purged { freed_bytes } else { 0 },
        clean_mode,
        is_admin,
    })
}

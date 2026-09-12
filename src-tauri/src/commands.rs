//! Tauri 前端交互命令层
//!
//! 暴露供前端 Vue 界面调用的系统管理与启动项调度 Commands。

use crate::storage;
use crate::system::types::{CleanResult, LaunchItem, MemoryStatus};
use serde::{Deserialize, Serialize};

/// 批量执行启动项的单项运行结果
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BatchLaunchResult {
    /// 启动项唯一标识 ID
    pub id: String,
    /// 启动项显示名称
    pub name: String,
    /// 是否启动成功
    pub success: bool,
    /// 成功启动后的进程 ID (PID)
    pub pid: Option<u32>,
    /// 失败时的错误信息描述
    pub error: Option<String>,
}

/// 获取当前系统物理内存运行状态
///
/// 包括物理总内存、可用内存、已用内存及使用百分比。
#[tauri::command]
pub fn get_memory_status() -> Result<MemoryStatus, String> {
    crate::system::get_memory_info()
}

/// 执行系统级进程工作集修剪清理
///
/// 遍历并优化非关键用户态进程工作集，返回释放的内存大小及修剪前后的内存占用。
#[tauri::command]
pub fn clean_system_memory() -> Result<CleanResult, String> {
    crate::system::clean_process_working_sets()
}

/// 执行单个指定的启动项程序
///
/// 支持普通与静默（无窗口）模式启动，成功时返回目标进程的 PID。
#[tauri::command]
pub fn execute_launch_item(item: LaunchItem) -> Result<u32, String> {
    crate::system::launch_process(&item)
}

/// 批量执行启动项列表
///
/// 依次遍历启动项并执行；若启动项未启用则自动跳过，
/// 单个项的启动失败不会中断后续项目的执行，最终汇总返回所有项的启动状态列表。
#[tauri::command]
pub fn execute_all_launch_items(items: Vec<LaunchItem>) -> Result<Vec<BatchLaunchResult>, String> {
    let mut results = Vec::with_capacity(items.len());

    for item in items {
        if !item.enabled {
            results.push(BatchLaunchResult {
                id: item.id,
                name: item.name,
                success: false,
                pid: None,
                error: Some("启动项未启用，已跳过".to_string()),
            });
            continue;
        }

        match crate::system::launch_process(&item) {
            Ok(pid) => {
                results.push(BatchLaunchResult {
                    id: item.id,
                    name: item.name,
                    success: true,
                    pid: Some(pid),
                    error: None,
                });
            }
            Err(err) => {
                results.push(BatchLaunchResult {
                    id: item.id,
                    name: item.name,
                    success: false,
                    pid: None,
                    error: Some(err),
                });
            }
        }
    }

    Ok(results)
}

/// 从本地持久化存储加载启动项配置
///
/// 首次运行时若配置文件不存在，将自动初始化默认启动项（记事本与计算器）。
#[tauri::command]
pub fn load_launcher_config(app: tauri::AppHandle) -> Result<Vec<LaunchItem>, String> {
    storage::load_launcher_items(&app)
}

/// 将启动项配置持久化保存到本地存储
#[tauri::command]
pub fn save_launcher_config(app: tauri::AppHandle, items: Vec<LaunchItem>) -> Result<(), String> {
    storage::save_launcher_items(&app, &items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    /// 辅助测试清理进程，避免残留
    fn kill_process_by_pid(pid: u32) {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
    }

    #[test]
    fn test_get_memory_status_command() {
        let res = get_memory_status();
        assert!(res.is_ok(), "获取内存状态命令应成功执行");
        let status = res.unwrap();
        assert!(status.total_ram > 0, "总内存应大于 0");
        assert!(status.usage_percent >= 0.0 && status.usage_percent <= 100.0);
    }

    #[test]
    fn test_execute_launch_item_command() {
        let item = LaunchItem {
            id: "test-cmd".to_string(),
            name: "测试命令".to_string(),
            path: "cmd.exe".to_string(),
            args: "/c exit 0".to_string(),
            work_dir: None,
            silent: true,
            enabled: true,
        };

        let result = execute_launch_item(item);
        assert!(result.is_ok(), "执行合法启动项应返回 PID");
        let pid = result.unwrap();
        assert!(pid > 0);
        kill_process_by_pid(pid);
    }

    #[test]
    fn test_execute_all_launch_items_mixed() {
        let items = vec![
            // 1. 启用的合法项
            LaunchItem {
                id: "item-valid".to_string(),
                name: "合法程序".to_string(),
                path: "cmd.exe".to_string(),
                args: "/c exit 0".to_string(),
                work_dir: None,
                silent: true,
                enabled: true,
            },
            // 2. 未启用的项 (应被跳过)
            LaunchItem {
                id: "item-disabled".to_string(),
                name: "禁用程序".to_string(),
                path: "notepad.exe".to_string(),
                args: "".to_string(),
                work_dir: None,
                silent: false,
                enabled: false,
            },
            // 3. 启用的非法项 (应捕获错误而不崩溃)
            LaunchItem {
                id: "item-invalid".to_string(),
                name: "不存在程序".to_string(),
                path: "this_non_existent_executable_12345.exe".to_string(),
                args: "".to_string(),
                work_dir: None,
                silent: false,
                enabled: true,
            },
        ];

        let results = execute_all_launch_items(items).expect("批量执行不应整体抛错");
        assert_eq!(results.len(), 3, "应返回所有 3 个项的结果");

        // 验证项 1
        assert_eq!(results[0].id, "item-valid");
        assert!(results[0].success);
        assert!(results[0].pid.is_some());
        assert!(results[0].error.is_none());
        if let Some(pid) = results[0].pid {
            kill_process_by_pid(pid);
        }

        // 验证项 2 (未启用)
        assert_eq!(results[1].id, "item-disabled");
        assert!(!results[1].success);
        assert!(results[1].pid.is_none());
        assert_eq!(results[1].error.as_deref(), Some("启动项未启用，已跳过"));

        // 验证项 3 (启动失败)
        assert_eq!(results[2].id, "item-invalid");
        assert!(!results[2].success);
        assert!(results[2].pid.is_none());
        assert!(results[2].error.is_some());
    }

    #[test]
    fn test_batch_launch_result_serialization() {
        let sample = BatchLaunchResult {
            id: "batch-1".to_string(),
            name: "批量测试项".to_string(),
            success: true,
            pid: Some(1234),
            error: None,
        };

        let serialized = serde_json::to_string(&sample).expect("序列化应成功");
        let deserialized: BatchLaunchResult =
            serde_json::from_str(&serialized).expect("反序列化应成功");

        assert_eq!(sample, deserialized);
    }
}

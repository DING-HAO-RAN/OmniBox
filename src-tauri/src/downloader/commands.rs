//! 高速多线程下载器 Tauri 命令层 (IPC Commands)
//!
//! 暴露供前端 Vue 界面调用的任务创建、列表查询、暂停、继续、取消删除、URL 预探测与本地文件定位接口。

use super::client::probe_url_meta;
use super::engine::DownloadManager;
use super::types::{DownloadTask, NewTaskParams, UrlMeta};
use std::path::Path;
use std::sync::Arc;

/// 创建新的多线程下载任务并异步拉起下载
#[tauri::command]
pub fn downloader_create_task(
    manager: tauri::State<'_, Arc<DownloadManager>>,
    params: NewTaskParams,
) -> Result<DownloadTask, String> {
    manager.create_task(params)
}

/// 获取当前所有下载任务的快照列表
#[tauri::command]
pub fn downloader_list_tasks(
    manager: tauri::State<'_, Arc<DownloadManager>>,
) -> Result<Vec<DownloadTask>, String> {
    Ok(manager.get_tasks())
}

/// 暂停正在下载的任务
#[tauri::command]
pub fn downloader_pause_task(
    manager: tauri::State<'_, Arc<DownloadManager>>,
    id: String,
) -> Result<(), String> {
    manager.pause_task(&id)
}

/// 恢复/继续已暂停或失败的任务
#[tauri::command]
pub fn downloader_resume_task(
    manager: tauri::State<'_, Arc<DownloadManager>>,
    id: String,
) -> Result<(), String> {
    manager.resume_task(&id)
}

/// 取消或删除任务
#[tauri::command]
pub fn downloader_delete_task(
    manager: tauri::State<'_, Arc<DownloadManager>>,
    id: String,
    delete_file: bool,
) -> Result<(), String> {
    manager.cancel_task(&id, delete_file)
}

/// 预探测目标 URL，提取文件总大小、建议文件名与 Range 支持
#[tauri::command]
pub fn downloader_probe_url(url: String) -> Result<UrlMeta, String> {
    probe_url_meta(&url, None)
}

/// 弹出 Windows 原生文件夹选择器，供用户指定下载保存路径
#[tauri::command]
pub fn downloader_choose_dir() -> Result<Option<String>, String> {
    Ok(crate::session_migrator::choose_directory_dialog())
}

/// 使用 Windows 默认程序直接打开已下载的文件
#[tauri::command]
pub fn downloader_open_file(path: String) -> Result<(), String> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let wide_open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let res = ShellExecuteW(
                0 as HWND,
                wide_open.as_ptr(),
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL as i32,
            );
            if (res as usize) <= 32 {
                return Err(format!("打开文件失败，系统错误代码: {}", res as usize));
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = open::that(&path);
    }

    Ok(())
}

/// 在 Windows 资源管理器中打开并选中指定文件
#[tauri::command]
pub fn downloader_open_folder(path: String) -> Result<(), String> {
    let target = Path::new(&path);
    if !target.exists() {
        // 如果文件本身不存在，尝试打开其父目录
        if let Some(parent) = target.parent() {
            if parent.exists() {
                std::process::Command::new("explorer")
                    .arg(parent)
                    .spawn()
                    .map_err(|e| format!("打开所在文件夹失败: {}", e))?;
                return Ok(());
            }
        }
        return Err(format!("目标路径不存在: {}", path));
    }

    // Windows explorer /select,"C:\path\to\file"
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", path))
        .spawn()
        .map_err(|e| format!("在资源管理器中定位文件失败: {}", e))?;

    Ok(())
}

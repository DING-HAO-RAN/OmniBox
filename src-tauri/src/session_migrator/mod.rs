//! 浏览器会话迁移器 (Browser Session Migrator) 模块
//!
//! 提供本地独立 CDP 浏览器管理、安全文件读写、SHA-256 计算与会话存储管理。

pub mod launcher;
pub mod types;

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub use launcher::{
    choose_directory_dialog, find_browser_executable, get_default_profile_dir, get_running_cdp_status,
    get_sessions_dir, launch_local_cdp, stop_local_cdp, write_profile_launcher_cmd,
};

/// 默认配置参数
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct SessionMigratorDefaults {
    pub default_chrome_profile: String,
    pub default_edge_profile: String,
    pub sessions_dir: String,
    pub default_domain: String,
    pub default_site_url: String,
    pub default_port: u16,
}

/// 获取会话迁移器默认配置
pub fn get_defaults() -> SessionMigratorDefaults {
    SessionMigratorDefaults {
        default_chrome_profile: get_default_profile_dir("chrome").to_string_lossy().into_owned(),
        default_edge_profile: get_default_profile_dir("edge").to_string_lossy().into_owned(),
        sessions_dir: get_sessions_dir().to_string_lossy().into_owned(),
        default_domain: "bilibili.com".to_string(),
        default_site_url: "https://www.bilibili.com/".to_string(),
        default_port: 9222,
    }
}

/// 计算指定文件的 SHA-256 哈希值
pub fn compute_file_sha256(path_str: &str) -> Result<String, String> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(format!("文件不存在: {path_str}"));
    }

    let bytes = fs::read(path).map_err(|e| format!("读取文件失败: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// 原子安全写入加密会话文件
pub fn write_encrypted_session_file(path_str: &str, content: &str) -> Result<String, String> {
    let destination = PathBuf::from(path_str);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }

    let temp = destination.with_extension("part");
    fs::write(&temp, content).map_err(|e| format!("写入临时文件失败: {e}"))?;
    fs::rename(&temp, &destination).map_err(|e| format!("原子替换文件失败: {e}"))?;

    compute_file_sha256(path_str)
}

/// 读取加密会话文件文本
pub fn read_encrypted_session_file(path_str: &str) -> Result<String, String> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(format!("文件不存在: {path_str}"));
    }

    // 限制单文件最大 20MB
    let metadata = fs::metadata(path).map_err(|e| format!("读取文件属性失败: {e}"))?;
    if metadata.len() > 20 * 1024 * 1024 {
        return Err("会话文件超过 20MB 安全限制".to_string());
    }

    fs::read_to_string(path).map_err(|e| format!("读取会话文件失败: {e}"))
}

/// 在 Windows 资源管理器中打开 sessions 目录
pub fn open_sessions_folder() -> Result<(), String> {
    let dir = get_sessions_dir();
    let _ = fs::create_dir_all(&dir);

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide_dir: Vec<u16> = dir.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
        let wide_open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            ShellExecuteW(
                0 as HWND,
                wide_open.as_ptr(),
                wide_dir.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL as i32,
            );
        }
    }

    Ok(())
}

//! 本地 Chromium (Chrome / Edge) 浏览器 CDP 进程启动与管理模块
//!
//! 负责探测浏览器安装路径、配置独立用户数据目录、启动带 127.0.0.1 远程调试端口的进程、
//! 执行健康检查，以及在桌面生成独立的启动脚本 (.cmd)。

use super::types::RunningBrowserInfo;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static RUNNING_BROWSER_PROCESS: Mutex<Option<(Child, RunningBrowserInfo)>> = Mutex::new(None);

/// 在 Windows 常见安装路径中查找 Chrome 或 Edge 可执行文件
pub fn find_browser_executable(browser: &str) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    let prog_files = env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
    let prog_files_x86 = env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
    let local_app_data = env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".to_string());

    match browser.to_lowercase().as_str() {
        "chrome" => {
            candidates.push(Path::new(&prog_files).join("Google\\Chrome\\Application\\chrome.exe"));
            candidates.push(Path::new(&prog_files_x86).join("Google\\Chrome\\Application\\chrome.exe"));
            candidates.push(Path::new(&local_app_data).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        "edge" => {
            candidates.push(Path::new(&prog_files_x86).join("Microsoft\\Edge\\Application\\msedge.exe"));
            candidates.push(Path::new(&prog_files).join("Microsoft\\Edge\\Application\\msedge.exe"));
            candidates.push(Path::new(&local_app_data).join("Microsoft\\Edge\\Application\\msedge.exe"));
        }
        other => return Err(format!("不支持的浏览器类型: {other}，仅支持 chrome 或 edge")),
    }

    for path in candidates {
        if path.is_file() {
            return Ok(path);
        }
    }

    // 尝试在 PATH 环境变量中寻找
    let exe_name = if browser.eq_ignore_ascii_case("edge") { "msedge.exe" } else { "chrome.exe" };
    if let Ok(paths) = env::var("PATH") {
        for p in env::split_paths(&paths) {
            let full = p.join(exe_name);
            if full.is_file() {
                return Ok(full);
            }
        }
    }

    Err(format!("未找到 {}。请先安装浏览器，或确认其已加入系统 PATH。", if browser.eq_ignore_ascii_case("edge") { "Microsoft Edge" } else { "Google Chrome" }))
}

/// 获取项目专用的独立配置目录 (不污染日常浏览器的 User Data 目录)
pub fn get_default_profile_dir(browser: &str) -> PathBuf {
    let local_app = env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".to_string());
    Path::new(&local_app)
        .join("BrowserSessionMigrator")
        .join("profiles")
        .join(format!("{}-cdp", browser.to_lowercase()))
}

/// 获取 sessions 加密会话文件默认保存目录
pub fn get_sessions_dir() -> PathBuf {
    let app_data = env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string());
    let dir = Path::new(&app_data).join("OmniBox").join("sessions");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 检查并读取本地 CDP 端点的 /json/version
pub fn check_cdp_version(port: u16, timeout_secs: u64) -> Result<String, String> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));

            let request = format!("GET /json/version HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nAccept: application/json\r\n\r\n");
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut buffer = String::new();
                if stream.read_to_string(&mut buffer).is_ok() && buffer.contains("200 OK") {
                    if let Some(body_start) = buffer.find("\r\n\r\n") {
                        let body = &buffer[body_start + 4..];
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
                            if let Some(b) = val.get("Browser").and_then(|v| v.as_str()) {
                                return Ok(b.to_string());
                            }
                        }
                    }
                    return Ok("Chromium / CDP Ready".to_string());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    Err(format!("在限定时间内浏览器未能响应本地 CDP 端口 (http://127.0.0.1:{port})"))
}

/// 一键启动本地 CDP 浏览器
pub fn launch_local_cdp(browser: &str, port: u16, profile_dir: &str) -> Result<RunningBrowserInfo, String> {
    let mut lock = RUNNING_BROWSER_PROCESS.lock().unwrap();

    // 如果之前已有实例在运行，先检查是否还存活
    if let Some((ref mut child, ref info)) = *lock {
        if child.try_wait().ok().flatten().is_none() {
            return Ok(info.clone());
        }
    }

    let executable = find_browser_executable(browser)?;
    let profile_path = if profile_dir.trim().is_empty() {
        get_default_profile_dir(browser)
    } else {
        PathBuf::from(profile_dir.trim())
    };

    fs::create_dir_all(&profile_path)
        .map_err(|e| format!("无法创建独立配置目录 [{}]: {e}", profile_path.display()))?;

    let endpoint = format!("http://127.0.0.1:{port}");

    // 启动参数完全对齐原项目的安全沙箱规则
    let mut cmd = Command::new(&executable);
    cmd.arg("--remote-debugging-address=127.0.0.1")
        .arg(format!("--remote-debugging-port={port}"))
        .arg(format!("--user-data-dir={}", profile_path.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--remote-allow-origins=*");

    let child = cmd.spawn()
        .map_err(|e| format!("启动浏览器进程失败 [{}]: {e}", executable.display()))?;

    let pid = child.id();

    // 健康检查等待 CDP 响应 (超时 15 秒)
    let version = match check_cdp_version(port, 15) {
        Ok(v) => v,
        Err(e) => {
            // 超时杀掉新起的进程并返回错误
            let mut c = child;
            let _ = c.kill();
            return Err(e);
        }
    };

    let info = RunningBrowserInfo {
        browser: browser.to_string(),
        endpoint,
        port,
        profile_dir: profile_path.to_string_lossy().into_owned(),
        pid,
        browser_version: version,
        is_running: true,
    };

    *lock = Some((child, info.clone()));

    Ok(info)
}

/// 停止当前由工具拉起的本地 CDP 浏览器
pub fn stop_local_cdp() -> Result<(), String> {
    let mut lock = RUNNING_BROWSER_PROCESS.lock().unwrap();
    if let Some((mut child, _)) = lock.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

/// 查询当前本地 CDP 浏览器运行状态
pub fn get_running_cdp_status() -> Option<RunningBrowserInfo> {
    let mut lock = RUNNING_BROWSER_PROCESS.lock().unwrap();
    if let Some((ref mut child, ref mut info)) = *lock {
        if child.try_wait().ok().flatten().is_none() {
            info.is_running = true;
            return Some(info.clone());
        } else {
            *lock = None;
        }
    }
    None
}

/// 生成只启动指定独立 Profile 的 Windows 桌面 `.cmd` 启动脚本
pub fn write_profile_launcher_cmd(browser: &str, port: u16, profile_dir: &str, dest_path: &str) -> Result<String, String> {
    let executable = find_browser_executable(browser)?;
    let profile = if profile_dir.trim().is_empty() {
        get_default_profile_dir(browser)
    } else {
        PathBuf::from(profile_dir.trim())
    };

    let destination = if dest_path.trim().is_empty() {
        let desktop = env::var("USERPROFILE")
            .map(|u| Path::new(&u).join("Desktop"))
            .unwrap_or_else(|_| PathBuf::from("C:\\"));
        desktop.join(format!("BrowserSessionMigrator-{}-cdp.cmd", browser.to_lowercase()))
    } else {
        PathBuf::from(dest_path.trim())
    };

    if let Some(parent) = destination.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let script = format!(
        "@echo off\r\nsetlocal EnableExtensions\r\nstart \"\" \"{}\" --remote-debugging-address=127.0.0.1 --remote-debugging-port={} --user-data-dir=\"{}\" --no-first-run --no-default-browser-check --remote-allow-origins=*\r\nexit /b 0\r\n",
        executable.display(),
        port,
        profile.display()
    );

    fs::write(&destination, script)
        .map_err(|e| format!("写入启动脚本失败 [{}]: {e}", destination.display()))?;

    Ok(destination.to_string_lossy().into_owned())
}

/// 通过 Windows 原生文件夹选择对话框选择目录
pub fn choose_directory_dialog() -> Option<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{
            SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS,
            BROWSEINFOW,
        };

        unsafe {
            let title: Vec<u16> = "选择独立浏览器配置目录\0".encode_utf16().collect();
            let mut bi: BROWSEINFOW = std::mem::zeroed();
            bi.lpszTitle = title.as_ptr();
            bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;

            let pidl = SHBrowseForFolderW(&bi);
            if !pidl.is_null() {
                let mut path_buf = [0u16; 512];
                if SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) != 0 {
                    let end = path_buf.iter().position(|&c| c == 0).unwrap_or(path_buf.len());
                    let dir = String::from_utf16_lossy(&path_buf[..end]).trim().to_string();
                    if !dir.is_empty() {
                        return Some(dir);
                    }
                }
            }
        }
    }
    None
}

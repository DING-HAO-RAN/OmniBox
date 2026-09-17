//! Tauri 前端交互命令层
//!
//! 暴露供前端 Vue 界面调用的系统管理与启动项调度 Commands。

use crate::storage;
use crate::system::types::{AppSettings, CleanResult, CloakedItem, LaunchItem};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Markdown 文档读写结果
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDocument {
    pub path: String,
    pub content: String,
}

/// 本地图片内容，前端会转换成 data URL 供预览和 PDF 导出使用
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownImageData {
    pub mime_type: String,
    pub data_base64: String,
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
        .unwrap_or(false)
}

fn ensure_markdown_path(path: &Path) -> Result<(), String> {
    if is_markdown_path(path) {
        Ok(())
    } else {
        Err("只支持 .md 或 .markdown 文件。".to_string())
    }
}

fn canonical_document_parent(path: &Path) -> Result<(PathBuf, PathBuf), String> {
    let document_path =
        fs::canonicalize(path).map_err(|error| format!("无法定位 Markdown 文档：{error}"))?;
    let document_parent = document_path
        .parent()
        .ok_or_else(|| "Markdown 文档缺少有效目录。".to_string())?
        .to_path_buf();
    Ok((document_path, document_parent))
}

fn image_mime_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

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
pub fn get_memory_status() -> Result<crate::system::info::SystemMemoryInfo, String> {
    Ok(crate::system::info::collect_memory_info())
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

// ==========================================
// 窗口控制系统命令（直接通过 Rust 原生窗口句柄操作）
// ==========================================

/// 最小化当前主窗口
#[tauri::command]
pub fn app_minimize_window(window: tauri::Window) -> Result<(), String> {
    window
        .minimize()
        .map_err(|e| format!("最小化窗口失败: {e}"))
}

/// 切换当前窗口最大化/还原状态
#[tauri::command]
pub fn app_toggle_maximize_window(window: tauri::Window) -> Result<bool, String> {
    let is_max = window
        .is_maximized()
        .map_err(|e| format!("获取最大化状态失败: {e}"))?;
    if is_max {
        window
            .unmaximize()
            .map_err(|e| format!("还原窗口失败: {e}"))?;
        Ok(false)
    } else {
        window
            .maximize()
            .map_err(|e| format!("最大化窗口失败: {e}"))?;
        Ok(true)
    }
}

/// 关闭当前窗口
#[tauri::command]
pub fn app_close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| format!("关闭窗口失败: {e}"))
}

/// 查询当前窗口是否处于最大化状态
#[tauri::command]
pub fn app_is_maximized(window: tauri::Window) -> Result<bool, String> {
    window
        .is_maximized()
        .map_err(|e| format!("查询最大化状态失败: {e}"))
}

// ==========================================
// 系统设置与权限管理命令
// ==========================================

/// 检查当前程序是否已获取管理员权限
#[tauri::command]
pub fn get_admin_status() -> Result<bool, String> {
    Ok(crate::system::memory::is_running_as_admin())
}

/// 触发以管理员身份重新启动当前程序
#[tauri::command]
pub fn request_restart_as_admin() -> Result<(), String> {
    crate::system::settings::restart_as_admin()
}

/// 加载应用程序全局设置
#[tauri::command]
pub fn get_app_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let mut settings = storage::load_app_settings(&app)?;
    // 与 Windows 注册表开机自启真实状态保持同步
    if let Ok(reg_auto_start) = crate::system::settings::get_auto_start_status() {
        settings.auto_start = reg_auto_start;
    }
    Ok(settings)
}

/// 更新并保存应用程序全局设置
#[tauri::command]
pub fn update_app_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let _ = crate::system::settings::set_auto_start_status(settings.auto_start);
    storage::save_app_settings(&app, &settings)
}

// ==========================================
// 深度隐藏文件/文件夹管理器命令
// ==========================================

/// 对指定文件或文件夹施加系统级深度隐藏 (Super Hidden)，并登记到隐藏清单中
#[tauri::command]
pub fn cloak_file_or_dir(
    app: tauri::AppHandle,
    path: String,
    note: Option<String>,
) -> Result<CloakedItem, String> {
    let trimmed_path = path.trim().trim_matches('"');
    let target = Path::new(trimmed_path);
    if !target.exists() {
        return Err(format!("目标路径不存在: {trimmed_path}"));
    }

    let is_dir = target.is_dir();
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| trimmed_path.to_string());

    // 施加系统级隐身
    crate::system::cloaker::cloak_path(trimmed_path)?;

    let mut list = storage::load_cloaked_items(&app)?;
    let id = format!(
        "cloak-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // 如果列表中已存在相同路径，则更新状态，否则新增
    if let Some(existing) = list
        .iter_mut()
        .find(|item| item.path.eq_ignore_ascii_case(trimmed_path))
    {
        existing.is_cloaked = true;
        if let Some(n) = note {
            existing.note = n;
        }
        let result = existing.clone();
        storage::save_cloaked_items(&app, &list)?;
        return Ok(result);
    }

    let new_item = CloakedItem {
        id,
        name,
        path: trimmed_path.to_string(),
        is_dir,
        added_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        is_cloaked: true,
        note: note.unwrap_or_default(),
    };

    list.push(new_item.clone());
    storage::save_cloaked_items(&app, &list)?;

    Ok(new_item)
}

/// 解除指定条目的深度隐藏状态，恢复正常可见
#[tauri::command]
pub fn uncloak_file_or_dir(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut list = storage::load_cloaked_items(&app)?;
    let item = list
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or_else(|| "未找到指定的隐藏记录".to_string())?;

    crate::system::cloaker::uncloak_path(&item.path)?;
    item.is_cloaked = false;

    storage::save_cloaked_items(&app, &list)?;
    Ok(())
}

/// 重新对已记录的项目施加深度隐藏
#[tauri::command]
pub fn recloak_file_or_dir(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut list = storage::load_cloaked_items(&app)?;
    let item = list
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or_else(|| "未找到指定的记录".to_string())?;

    crate::system::cloaker::cloak_path(&item.path)?;
    item.is_cloaked = true;

    storage::save_cloaked_items(&app, &list)?;
    Ok(())
}

/// 获取所有已登记的深度隐形项目列表（会自动校准实际磁盘属性状态）
#[tauri::command]
pub fn load_cloaked_list(app: tauri::AppHandle) -> Result<Vec<CloakedItem>, String> {
    let mut list = storage::load_cloaked_items(&app)?;
    for item in list.iter_mut() {
        if let Ok(is_cloaked) = crate::system::cloaker::is_path_cloaked(&item.path) {
            item.is_cloaked = is_cloaked;
        }
    }
    let _ = storage::save_cloaked_items(&app, &list);
    Ok(list)
}

/// 移除隐藏记录（若当前仍为隐藏，会自动先解除隐藏，避免用户找不到文件）
#[tauri::command]
pub fn remove_cloaked_record(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut list = storage::load_cloaked_items(&app)?;
    if let Some(pos) = list.iter().position(|i| i.id == id) {
        let item = &list[pos];
        if item.is_cloaked {
            let _ = crate::system::cloaker::uncloak_path(&item.path);
        }
        list.remove(pos);
        storage::save_cloaked_items(&app, &list)?;
    }
    Ok(())
}

/// 核心联动：将隐藏的文件一键发送至“快速启动器”
/// 用户在桌面上彻底隐藏该文件后，在 OmniBox 启动器中依然可以随时一键启动！
#[tauri::command]
pub fn send_cloaked_to_launcher(
    app: tauri::AppHandle,
    id: String,
    silent: bool,
) -> Result<LaunchItem, String> {
    let list = storage::load_cloaked_items(&app)?;
    let cloaked = list
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "未找到指定隐藏项".to_string())?;

    let mut launchers = storage::load_launcher_items(&app)?;

    // 检查是否已有相同路径
    if let Some(existing) = launchers
        .iter()
        .find(|l| l.path.eq_ignore_ascii_case(&cloaked.path))
    {
        return Ok(existing.clone());
    }

    let new_launch = LaunchItem {
        id: format!(
            "launch-from-cloak-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ),
        name: format!("🔒 {}", cloaked.name),
        path: cloaked.path.clone(),
        args: "".to_string(),
        work_dir: Path::new(&cloaked.path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned()),
        silent,
        enabled: true,
    };

    launchers.push(new_launch.clone());
    storage::save_launcher_items(&app, &launchers)?;

    Ok(new_launch)
}

/// 弹出 Windows 原生选择文件对话框
#[tauri::command]
pub fn choose_any_file() -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        Ok(choose_file_path(FileDialogMode::OpenAny))
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

// ==========================================
// 硬件性能实时监控命令
// ==========================================

/// 获取系统硬件配置及当前实时性能快照 (CPU/GPU/内存/磁盘/网络)
#[tauri::command]
pub fn get_performance_snapshot() -> Result<crate::system::HardwarePerformance, String> {
    Ok(crate::system::get_hardware_performance())
}

/// 采集全量多维度系统与硬件诊断报告 (覆盖整机/主板/CPU/多显卡/物理DIMM/NVMe/网络/显示音频外设/进程/安全)
#[tauri::command]
pub fn get_system_full_report() -> Result<crate::system::info::SystemFullReport, String> {
    Ok(crate::system::info::collect_full_system_report())
}

/// 导出完整格式化 JSON 系统诊断报告 (支持按需脱敏保护隐私)
#[tauri::command]
pub fn export_system_report(sanitize: bool) -> Result<String, String> {
    crate::system::info::export_system_report_json(sanitize)
}

// ==========================================
// Windows 原生系统工具快捷唤起命令
// ==========================================

/// 获取全套 Windows 实用系统工具列表
#[tauri::command]
pub fn get_system_tools() -> Result<Vec<crate::system::SystemToolItem>, String> {
    Ok(crate::system::get_system_tools_list())
}

/// 唤起指定的原生系统工具 (如 gpedit.msc, regedit 等)
#[tauri::command]
pub fn launch_system_tool_cmd(command: String) -> Result<(), String> {
    crate::system::launch_tool_command(&command)
}

// ==========================================
// 系统特性一键优化与禁用命令
// ==========================================

/// 获取系统调优特性当前状态列表 (Windows 更新、安全中心等)
#[tauri::command]
pub fn get_system_tweaks() -> Result<Vec<crate::system::SystemTweakItem>, String> {
    Ok(crate::system::get_all_tweaks())
}

/// 执行指定系统特性的快速禁用或恢复
#[tauri::command]
pub fn apply_system_tweak(id: String, disable: bool) -> Result<(), String> {
    crate::system::toggle_tweak(&id, disable)
}

/// 打开 Markdown 文件并返回规范化路径与 UTF-8 文本。
#[tauri::command]
pub fn read_markdown_file(path: String) -> Result<MarkdownDocument, String> {
    let requested_path = PathBuf::from(path);
    ensure_markdown_path(&requested_path)?;
    let document_path = fs::canonicalize(&requested_path)
        .map_err(|error| format!("无法打开 Markdown 文件：{error}"))?;
    let content = fs::read_to_string(&document_path)
        .map_err(|error| format!("Markdown 文件不是有效的 UTF-8 文本，或无法读取：{error}"))?;

    Ok(MarkdownDocument {
        path: document_path.to_string_lossy().into_owned(),
        content,
    })
}

/// 保存 Markdown 文本；目标文件必须使用 .md 或 .markdown 扩展名。
#[tauri::command]
pub fn save_markdown_file(path: String, content: String) -> Result<MarkdownDocument, String> {
    let requested_path = PathBuf::from(path);
    ensure_markdown_path(&requested_path)?;
    let parent = requested_path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .ok_or_else(|| "Markdown 文件缺少有效目录。".to_string())?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(|error| format!("无法写入 Markdown 所在目录：{error}"))?;
    let file_name = requested_path
        .file_name()
        .ok_or_else(|| "Markdown 文件缺少文件名。".to_string())?;
    let document_path = canonical_parent.join(file_name);

    fs::write(&document_path, content.as_bytes())
        .map_err(|error| format!("保存 Markdown 文件失败：{error}"))?;

    Ok(MarkdownDocument {
        path: document_path.to_string_lossy().into_owned(),
        content,
    })
}

/// 读取与当前 Markdown 文档位于同一目录下的图片。
#[tauri::command]
pub fn read_markdown_image(
    document_path: String,
    relative_path: String,
) -> Result<MarkdownImageData, String> {
    let (document_path, document_parent) = canonical_document_parent(Path::new(&document_path))?;
    ensure_markdown_path(&document_path)?;

    let relative_image_path = Path::new(&relative_path);
    if relative_image_path.is_absolute() || relative_path.trim().is_empty() {
        return Err("本地图片路径必须是 Markdown 文档目录下的相对路径。".to_string());
    }

    let image_path = fs::canonicalize(document_parent.join(relative_image_path))
        .map_err(|error| format!("无法读取本地图片：{error}"))?;
    if !image_path.starts_with(&document_parent) {
        return Err("本地图片路径超出了 Markdown 文档目录。".to_string());
    }

    let mime_type = image_mime_type(&image_path)
        .ok_or_else(|| "只支持 PNG、JPG、GIF 和 WebP 图片。".to_string())?;
    let bytes = fs::read(&image_path).map_err(|error| format!("读取本地图片失败：{error}"))?;

    use base64::Engine;
    Ok(MarkdownImageData {
        mime_type: mime_type.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

/// 通过 Windows 原生文件选择器选择 Markdown 文件。
#[tauri::command]
pub fn choose_markdown_file() -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        return Ok(choose_file_path(FileDialogMode::OpenMarkdown));
    }

    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

/// 通过 Windows 原生保存对话框选择 Markdown 输出路径。
#[tauri::command]
pub fn choose_markdown_output() -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        return Ok(choose_file_path(FileDialogMode::SaveMarkdown));
    }

    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

/// 通过 Windows 原生保存对话框选择 PDF 输出路径。
#[tauri::command]
pub fn choose_pdf_output() -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        return Ok(choose_file_path(FileDialogMode::SavePdf));
    }

    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

#[cfg(windows)]
enum FileDialogMode {
    OpenMarkdown,
    SaveMarkdown,
    SavePdf,
    OpenAny,
}

#[cfg(windows)]
fn choose_file_path(mode: FileDialogMode) -> Option<String> {
    use std::mem::size_of;
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_OVERWRITEPROMPT,
        OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let (filter_text, title_text, default_extension_text, save) = match mode {
        FileDialogMode::OpenMarkdown => (
            "Markdown 文件\0*.md;*.markdown\0所有文件\0*.*\0",
            "打开 Markdown 文件",
            "md",
            false,
        ),
        FileDialogMode::SaveMarkdown => (
            "Markdown 文件\0*.md;*.markdown\0所有文件\0*.*\0",
            "保存 Markdown 文件",
            "md",
            true,
        ),
        FileDialogMode::SavePdf => (
            "PDF 文件\0*.pdf\0所有文件\0*.*\0",
            "导出 Markdown 为 PDF",
            "pdf",
            true,
        ),
        FileDialogMode::OpenAny => (
            "所有文件 (*.*)\0*.*\0可执行文件 (*.exe;*.bat;*.cmd)\0*.exe;*.bat;*.cmd\0",
            "选择文件或程序",
            "",
            false,
        ),
    };
    let filter = wide_null(filter_text);
    let title = wide_null(title_text);
    let default_extension = wide_null(default_extension_text);
    let mut file_buffer = vec![0u16; 32_768];
    let mut file_dialog = OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: null_mut(),
        hInstance: null_mut(),
        lpstrFilter: filter.as_ptr(),
        lpstrCustomFilter: null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 1,
        lpstrFile: file_buffer.as_mut_ptr(),
        nMaxFile: file_buffer.len() as u32,
        lpstrFileTitle: null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: title.as_ptr(),
        Flags: OFN_EXPLORER
            | OFN_PATHMUSTEXIST
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            },
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: default_extension.as_ptr(),
        lCustData: 0,
        lpfnHook: None,
        lpTemplateName: std::ptr::null(),
        pvReserved: null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    };

    let succeeded = unsafe {
        if save {
            GetSaveFileNameW(&mut file_dialog) != 0
        } else {
            GetOpenFileNameW(&mut file_dialog) != 0
        }
    };

    if !succeeded {
        return None;
    }

    let length = file_buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(file_buffer.len());
    Some(String::from_utf16_lossy(&file_buffer[..length]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::Path;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, SYNCHRONIZE};

    fn assert_metric_is_well_formed<T>(metric: &crate::system::info::MetricValue<T>) {
        assert!(!metric.source.trim().is_empty(), "指标 source 不能为空");
        assert!(metric.timestamp > 0, "指标 timestamp 必须为正数");
        assert!(!metric.unit.contains('\0'), "指标 unit 不得包含 NUL 字符");
        match metric.quality {
            crate::system::info::MetricQuality::Good
            | crate::system::info::MetricQuality::Estimated => {
                assert!(metric.value.is_some(), "成功指标必须携带 value");
            }
            _ => {
                assert!(metric.value.is_none(), "非成功指标必须保持 value 为 null");
                assert!(
                    metric
                        .error
                        .as_deref()
                        .is_some_and(|error| !error.trim().is_empty()),
                    "失败指标必须包含 error"
                );
            }
        }
    }

    fn assert_collection_status_is_well_formed(status: &crate::system::info::CollectionStatus) {
        assert!(!status.source.trim().is_empty(), "集合 source 不能为空");
        assert!(status.timestamp > 0, "集合 timestamp 必须为正数");
        if !matches!(
            status.quality,
            crate::system::info::MetricQuality::Good
                | crate::system::info::MetricQuality::Estimated
        ) {
            assert_eq!(
                status.item_count.unwrap_or(0),
                0,
                "失败集合不得报告有效条目"
            );
            assert!(
                status
                    .error
                    .as_deref()
                    .is_some_and(|error| !error.trim().is_empty()),
                "失败集合必须包含 error"
            );
        }
    }

    fn assert_serialized_metrics(value: &Value) {
        match value {
            Value::Object(object) => {
                let is_metric = ["value", "unit", "quality", "source", "timestamp"]
                    .iter()
                    .all(|key| object.contains_key(*key));
                if is_metric {
                    let metric: crate::system::info::MetricValue<Value> =
                        serde_json::from_value(value.clone()).expect("MetricValue JSON 应合法");
                    assert_metric_is_well_formed(&metric);
                    if metric.unit == "%" {
                        if let Some(percent) = object.get("value").and_then(Value::as_f64) {
                            assert!((0.0..=100.0).contains(&percent));
                        }
                    }
                } else if [
                    "quality",
                    "source",
                    "timestamp",
                    "item_count",
                    "truncated",
                    "error",
                ]
                .iter()
                .all(|key| object.contains_key(*key))
                {
                    let status: crate::system::info::CollectionStatus =
                        serde_json::from_value(value.clone())
                            .expect("CollectionStatus JSON 应合法");
                    assert_collection_status_is_well_formed(&status);
                }
                for child in object.values() {
                    assert_serialized_metrics(child);
                }
            }
            Value::Array(items) => {
                for child in items {
                    assert_serialized_metrics(child);
                }
            }
            _ => {}
        }
    }

    /// 等待测试自身启动的子进程自然退出，避免强制终止其它进程。
    fn wait_for_process_exit(pid: u32) {
        unsafe {
            let handle = OpenProcess(SYNCHRONIZE, 0, pid);
            if handle.is_null() {
                return;
            }
            assert_eq!(
                WaitForSingleObject(handle, 5_000),
                0,
                "子进程未在期限内退出"
            );
            CloseHandle(handle);
        }
    }

    #[test]
    fn test_get_memory_status_command() {
        let status = get_memory_status().expect("获取内存状态命令应成功执行");
        for metric in [
            &status.total_physical_bytes,
            &status.available_physical_bytes,
            &status.used_physical_bytes,
            &status.total_page_file_bytes,
            &status.available_page_file_bytes,
            &status.total_virtual_bytes,
            &status.available_virtual_bytes,
            &status.committed_bytes,
            &status.commit_limit_bytes,
            &status.paged_pool_bytes,
            &status.non_paged_pool_bytes,
            &status.hardware_reserved_bytes,
        ] {
            assert_metric_is_well_formed(metric);
        }
        assert_metric_is_well_formed(&status.usage_percent);
        assert_collection_status_is_well_formed(&status.provider_status);

        let json = serde_json::to_value(&status).expect("SystemMemoryInfo 应可序列化");
        assert_serialized_metrics(&json);
    }

    #[test]
    fn test_get_performance_snapshot_command() {
        let snapshot = get_performance_snapshot().expect("获取性能快照命令应成功执行");
        assert!(snapshot.timestamp > 0, "性能快照 timestamp 必须为正数");
        assert!(
            !snapshot.provider_status.is_empty(),
            "性能快照必须包含 Provider 状态"
        );
        for status in &snapshot.provider_status {
            assert_collection_status_is_well_formed(status);
        }

        let json = serde_json::to_value(&snapshot).expect("HardwarePerformance 应可序列化");
        assert_serialized_metrics(&json);
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
        let pid = result.expect("合法启动项应返回 PID");
        assert!(pid > 0);
        wait_for_process_exit(pid);
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
            wait_for_process_exit(pid);
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

    #[test]
    fn markdown_file_extension_accepts_markdown_files_only() {
        assert!(is_markdown_path(Path::new("notes.md")));
        assert!(is_markdown_path(Path::new("notes.markdown")));
        assert!(is_markdown_path(Path::new("NOTES.MD")));
        assert!(!is_markdown_path(Path::new("notes.txt")));
        assert!(!is_markdown_path(Path::new("notes.md.bak")));
    }
}

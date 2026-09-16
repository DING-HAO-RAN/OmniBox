//! 启动项持久化存储模块
//!
//! 负责在用户标准应用配置目录下安全存储与加载 `launcher_items.json`，
//! 并提供初次运行时的默认启动项回退机制。

use crate::system::types::{AppSettings, CloakedItem, LaunchItem};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager};

/// 启动项存储文件名
pub const LAUNCHER_CONFIG_FILE: &str = "launcher_items.json";
/// 深度隐藏文件记录文件名
pub const CLOAKED_ITEMS_FILE: &str = "cloaked_items.json";
/// 应用程序全局设置文件名
pub const APP_SETTINGS_FILE: &str = "app_settings.json";

/// 生成友好且安全的 Windows 通用系统工具默认启动项列表
///
/// 包含 Windows 记事本和计算器，路径均为系统内置程序名，不包含任何个人或绝对路径。
pub fn get_default_launch_items() -> Vec<LaunchItem> {
    vec![
        LaunchItem {
            id: "default-notepad".to_string(),
            name: "记事本".to_string(),
            path: "notepad.exe".to_string(),
            args: "".to_string(),
            work_dir: None,
            silent: false,
            enabled: true,
        },
        LaunchItem {
            id: "default-calc".to_string(),
            name: "计算器".to_string(),
            path: "calc.exe".to_string(),
            args: "".to_string(),
            work_dir: None,
            silent: false,
            enabled: true,
        },
    ]
}

/// 从指定文件路径加载启动项配置，若文件不存在则返回默认配置
pub fn load_items_from_path(file_path: &Path) -> Result<Vec<LaunchItem>, String> {
    if !file_path.exists() {
        return Ok(get_default_launch_items());
    }

    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("读取启动项配置文件失败 [{}]: {}", file_path.display(), e))?;

    // 若文件为空，亦回退为默认配置
    if content.trim().is_empty() {
        return Ok(get_default_launch_items());
    }

    let items: Vec<LaunchItem> = serde_json::from_str(&content)
        .map_err(|e| format!("反序列化启动项配置失败 [{}]: {}", file_path.display(), e))?;

    Ok(items)
}

/// 将启动项列表序列化并保存至指定文件路径
pub fn save_items_to_path(file_path: &Path, items: &[LaunchItem]) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置存储目录失败 [{}]: {}", parent.display(), e))?;
    }

    let json_str = serde_json::to_string_pretty(items)
        .map_err(|e| format!("序列化启动项配置失败: {}", e))?;

    fs::write(file_path, json_str)
        .map_err(|e| format!("写入启动项配置文件失败 [{}]: {}", file_path.display(), e))?;

    Ok(())
}

/// 读取启动项配置列表
///
/// 从 Tauri 标准应用配置目录加载 `launcher_items.json`，
/// 若文件不存在则生成默认配置并保存后返回。
pub fn load_launcher_items(app: &AppHandle) -> Result<Vec<LaunchItem>, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取应用配置目录失败: {}", e))?;

    let file_path = config_dir.join(LAUNCHER_CONFIG_FILE);

    if !file_path.exists() {
        let default_items = get_default_launch_items();
        // 首次加载若文件不存在，尝试自动持久化一份默认配置
        let _ = save_items_to_path(&file_path, &default_items);
        return Ok(default_items);
    }

    load_items_from_path(&file_path)
}

/// 保存启动项配置列表
///
/// 将启动项保存至 Tauri 标准应用配置目录的 `launcher_items.json` 中。
pub fn save_launcher_items(app: &AppHandle, items: &[LaunchItem]) -> Result<(), String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取应用配置目录失败: {}", e))?;

    let file_path = config_dir.join(LAUNCHER_CONFIG_FILE);
    save_items_to_path(&file_path, items)
}

/// 读取深度隐藏文件记录列表
pub fn load_cloaked_items(app: &AppHandle) -> Result<Vec<CloakedItem>, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取应用配置目录失败: {}", e))?;

    let file_path = config_dir.join(CLOAKED_ITEMS_FILE);
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("读取隐藏文件记录失败: {e}"))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let items: Vec<CloakedItem> = serde_json::from_str(&content)
        .map_err(|e| format!("反序列化隐藏文件记录失败: {e}"))?;
    Ok(items)
}

/// 保存深度隐藏文件记录列表
pub fn save_cloaked_items(app: &AppHandle, items: &[CloakedItem]) -> Result<(), String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取应用配置目录失败: {}", e))?;

    let file_path = config_dir.join(CLOAKED_ITEMS_FILE);
    if let Some(parent) = file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let json_str = serde_json::to_string_pretty(items)
        .map_err(|e| format!("序列化隐藏文件记录失败: {e}"))?;
    fs::write(&file_path, json_str)
        .map_err(|e| format!("写入隐藏文件记录失败: {e}"))?;
    Ok(())
}

/// 读取应用程序通用设置
pub fn load_app_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取应用配置目录失败: {}", e))?;

    let file_path = config_dir.join(APP_SETTINGS_FILE);
    if !file_path.exists() {
        let defaults = AppSettings::default();
        let _ = save_app_settings(app, &defaults);
        return Ok(defaults);
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("读取应用程序设置失败: {e}"))?;
    if content.trim().is_empty() {
        return Ok(AppSettings::default());
    }

    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("反序列化应用程序设置失败: {e}"))?;
    Ok(settings)
}

/// 保存应用程序通用设置
pub fn save_app_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取应用配置目录失败: {}", e))?;

    let file_path = config_dir.join(APP_SETTINGS_FILE);
    if let Some(parent) = file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let json_str = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("序列化应用程序设置失败: {e}"))?;
    fs::write(&file_path, json_str)
        .map_err(|e| format!("写入应用程序设置失败: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 构造测试专用的隔离临时文件路径
    fn get_test_temp_file(test_name: &str) -> std::path::PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_name = format!("omnibox_test_{}_{}.json", test_name, timestamp);
        std::env::temp_dir().join(file_name)
    }

    #[test]
    fn test_get_default_launch_items() {
        let defaults = get_default_launch_items();
        assert_eq!(defaults.len(), 2, "默认启动项数量应为 2");

        let notepad = &defaults[0];
        assert_eq!(notepad.id, "default-notepad");
        assert_eq!(notepad.name, "记事本");
        assert_eq!(notepad.path, "notepad.exe");
        assert!(notepad.enabled);

        let calc = &defaults[1];
        assert_eq!(calc.id, "default-calc");
        assert_eq!(calc.name, "计算器");
        assert_eq!(calc.path, "calc.exe");
        assert!(calc.enabled);
    }

    #[test]
    fn test_load_from_non_existent_file() {
        let temp_path = get_test_temp_file("non_existent");
        if temp_path.exists() {
            let _ = fs::remove_file(&temp_path);
        }

        let loaded = load_items_from_path(&temp_path).expect("加载不存在的文件应成功回退默认配置");
        assert_eq!(loaded, get_default_launch_items());
    }

    #[test]
    fn test_load_from_empty_file() {
        let temp_path = get_test_temp_file("empty");
        fs::write(&temp_path, "   \n\t  ").expect("写入空内容应成功");

        let loaded = load_items_from_path(&temp_path).expect("加载空白文件应成功回退默认配置");
        assert_eq!(loaded, get_default_launch_items());

        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let temp_path = get_test_temp_file("round_trip");
        let custom_items = vec![
            LaunchItem {
                id: "custom-1".to_string(),
                name: "自定义工具 1".to_string(),
                path: "cmd.exe".to_string(),
                args: "/c echo hello".to_string(),
                work_dir: Some("C:\\".to_string()),
                silent: true,
                enabled: true,
            },
            LaunchItem {
                id: "custom-2".to_string(),
                name: "自定义工具 2".to_string(),
                path: "powershell.exe".to_string(),
                args: "-NoProfile".to_string(),
                work_dir: None,
                silent: false,
                enabled: false,
            },
        ];

        save_items_to_path(&temp_path, &custom_items).expect("保存启动项配置应成功");
        assert!(temp_path.exists(), "配置文件应当已经被创建");

        let loaded = load_items_from_path(&temp_path).expect("重新读取配置应成功");
        assert_eq!(loaded, custom_items, "读取的数据应与保存的数据完全一致");

        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_load_invalid_json() {
        let temp_path = get_test_temp_file("invalid_json");
        fs::write(&temp_path, "{ invalid_json_content }").expect("写入非法 JSON 应成功");

        let result = load_items_from_path(&temp_path);
        assert!(result.is_err(), "解析损坏的 JSON 应该返回错误");
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("反序列化启动项配置失败"));

        let _ = fs::remove_file(&temp_path);
    }
}

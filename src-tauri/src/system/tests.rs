//! 系统底层模块单元测试
//!
//! 覆盖内存状态获取、工作集清理以及进程普通与静默启动。

use super::*;
use std::env;

#[test]
fn test_get_memory_info_validity() {
    let mem = get_memory_info().expect("读取系统内存信息应当成功");
    
    // 物理内存总容量必须大于 0 (通常至少大于 1GB)
    assert!(mem.total_ram > 1024 * 1024 * 1024, "总内存应大于 1GB");
    // 可用内存不能大于总内存
    assert!(mem.available_ram <= mem.total_ram, "可用内存不应超过总内存");
    // 已用内存不能大于总内存
    assert!(mem.used_ram <= mem.total_ram, "已用内存不应超过总内存");
    // 已用内存与可用内存之和应近似等于总内存
    assert_eq!(mem.total_ram, mem.used_ram + mem.available_ram, "已用与可用之和应等于总内存");
    // 占用率范围必须在 0.0 到 100.0 之间
    assert!(mem.usage_percent >= 0.0 && mem.usage_percent <= 100.0, "内存使用百分比应在 0.0-100.0 之间");
}

#[test]
fn test_clean_process_working_sets_execution() {
    let result = clean_process_working_sets().expect("工作集修剪应安全执行不返回致命错误");

    // 验证清理前后的百分比在合法范围
    assert!(result.before_usage_percent >= 0.0 && result.before_usage_percent <= 100.0);
    assert!(result.after_usage_percent >= 0.0 && result.after_usage_percent <= 100.0);
    // 在拥有基本权限的 Windows 用户会话下，至少应当能够修剪部分当前用户进程（包括本测试进程）
    assert!(result.processes_trimmed > 0, "应至少修剪一个进程的工作集");
}

#[test]
fn test_launch_process_normal() {
    // 使用标准系统命令 cmd.exe /c exit 0 进行普通启动测试
    let comspec = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    let item = LaunchItem {
        id: "test-normal-cmd".to_string(),
        name: "Test Normal Command".to_string(),
        path: comspec,
        args: "/c exit 0".to_string(),
        work_dir: None,
        silent: false,
        enabled: true,
    };

    let pid = launch_process(&item).expect("普通模式启动进程应成功");
    assert!(pid > 0, "返回的进程 PID 应为正整数");
}

#[test]
fn test_launch_process_silent() {
    // 使用 cmd.exe /c exit 0 进行静默无窗口启动测试
    let comspec = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    let item = LaunchItem {
        id: "test-silent-cmd".to_string(),
        name: "Test Silent Command".to_string(),
        path: comspec,
        args: "/c exit 0".to_string(),
        work_dir: None,
        silent: true,
        enabled: true,
    };

    let pid = launch_process(&item).expect("静默模式启动进程应成功");
    assert!(pid > 0, "返回的静默进程 PID 应为正整数");
}

#[test]
fn test_launch_process_with_workdir() {
    let comspec = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    let temp_dir = env::temp_dir().to_string_lossy().to_string();
    let item = LaunchItem {
        id: "test-workdir-cmd".to_string(),
        name: "Test Workdir Command".to_string(),
        path: comspec,
        args: "/c exit 0".to_string(),
        work_dir: Some(temp_dir),
        silent: true,
        enabled: true,
    };

    let pid = launch_process(&item).expect("指定工作目录启动应成功");
    assert!(pid > 0, "返回的进程 PID 应为正整数");
}

#[test]
fn test_launch_process_invalid_path() {
    let item = LaunchItem {
        id: "test-invalid-path".to_string(),
        name: "Test Invalid Path".to_string(),
        path: "C:\\NonExistentPath\\definitely_not_exist_12345.exe".to_string(),
        args: "".to_string(),
        work_dir: None,
        silent: false,
        enabled: true,
    };

    let res = launch_process(&item);
    assert!(res.is_err(), "不存在的文件路径应返回错误");
}

#[test]
fn test_launch_process_empty_path() {
    let item = LaunchItem {
        id: "test-empty-path".to_string(),
        name: "Test Empty Path".to_string(),
        path: "   ".to_string(),
        args: "".to_string(),
        work_dir: None,
        silent: false,
        enabled: true,
    };

    let res = launch_process(&item);
    assert!(res.is_err(), "空路径应返回错误");
}

#[test]
fn test_launch_process_quoted_path() {
    let comspec = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    let quoted_path = format!("\"{}\"", comspec);
    let item = LaunchItem {
        id: "test-quoted-cmd".to_string(),
        name: "Test Quoted Command".to_string(),
        path: quoted_path,
        args: "/c exit 0".to_string(),
        work_dir: None,
        silent: true,
        enabled: true,
    };

    let pid = launch_process(&item).expect("带引号的路径启动应成功");
    assert!(pid > 0, "返回的进程 PID 应为正整数");
}

#[test]
fn test_types_serialization() {
    let mem = MemoryStatus {
        total_ram: 16000000000,
        available_ram: 8000000000,
        used_ram: 8000000000,
        usage_percent: 50.0,
    };
    let json = serde_json::to_string(&mem).expect("序列化 MemoryStatus 应成功");
    let deserialized: MemoryStatus = serde_json::from_str(&json).expect("反序列化 MemoryStatus 应成功");
    assert_eq!(mem, deserialized);

    let clean = CleanResult {
        freed_bytes: 104857600,
        freed_mb: 100.0,
        processes_trimmed: 15,
        before_usage_percent: 65.5,
        after_usage_percent: 60.2,
        standby_freed_bytes: 52428800,
        clean_mode: "PCL2 深度内核优化 (待机缓存+全工作集)".to_string(),
        is_admin: true,
    };
    let json_clean = serde_json::to_string(&clean).expect("序列化 CleanResult 应成功");
    let deserialized_clean: CleanResult = serde_json::from_str(&json_clean).expect("反序列化 CleanResult 应成功");
    assert_eq!(clean, deserialized_clean);

    let settings = AppSettings::default();
    assert_eq!(settings.language, "zh-CN");
    assert_eq!(settings.auto_clean_threshold, 80);

    let cloaked = CloakedItem {
        id: "test-cloak".to_string(),
        name: "测试机密文件.txt".to_string(),
        path: "C:\\test.txt".to_string(),
        is_dir: false,
        added_at: 100000,
        is_cloaked: true,
        note: "私密备忘".to_string(),
    };
    let json_cloaked = serde_json::to_string(&cloaked).expect("序列化 CloakedItem 应成功");
    let deserialized_cloaked: CloakedItem = serde_json::from_str(&json_cloaked).expect("反序列化 CloakedItem 应成功");
    assert_eq!(cloaked, deserialized_cloaked);
}

#[test]
fn test_cloaker_on_temp_file() {
    use std::fs;
    let temp_file = std::env::temp_dir().join(format!("omnibox_cloak_test_{}.tmp", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::write(&temp_file, "secret content").expect("写临时测试文件应成功");

    let path_str = temp_file.to_string_lossy().to_string();
    assert!(!super::cloaker::is_path_cloaked(&path_str).unwrap(), "初始状态不应为隐身");

    super::cloaker::cloak_path(&path_str).expect("隐身文件应成功");
    assert!(super::cloaker::is_path_cloaked(&path_str).unwrap(), "施加后应为隐身状态");

    super::cloaker::uncloak_path(&path_str).expect("解除隐身应成功");
    assert!(!super::cloaker::is_path_cloaked(&path_str).unwrap(), "解除后不应为隐身");

    let _ = fs::remove_file(&temp_file);
}

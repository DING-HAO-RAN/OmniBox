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
    assert_eq!(
        mem.total_ram,
        mem.used_ram + mem.available_ram,
        "已用与可用之和应等于总内存"
    );
    // 占用率范围必须在 0.0 到 100.0 之间
    assert!(
        mem.usage_percent >= 0.0 && mem.usage_percent <= 100.0,
        "内存使用百分比应在 0.0-100.0 之间"
    );
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
    let deserialized: MemoryStatus =
        serde_json::from_str(&json).expect("反序列化 MemoryStatus 应成功");
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
    let deserialized_clean: CleanResult =
        serde_json::from_str(&json_clean).expect("反序列化 CleanResult 应成功");
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
    let deserialized_cloaked: CloakedItem =
        serde_json::from_str(&json_cloaked).expect("反序列化 CloakedItem 应成功");
    assert_eq!(cloaked, deserialized_cloaked);
}

#[test]
fn test_cloaker_on_temp_file() {
    use std::fs;
    let temp_file = std::env::temp_dir().join(format!(
        "omnibox_cloak_test_{}.tmp",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(&temp_file, "secret content").expect("写临时测试文件应成功");

    let path_str = temp_file.to_string_lossy().to_string();
    assert!(
        !super::cloaker::is_path_cloaked(&path_str).unwrap(),
        "初始状态不应为隐身"
    );

    super::cloaker::cloak_path(&path_str).expect("隐身文件应成功");
    assert!(
        super::cloaker::is_path_cloaked(&path_str).unwrap(),
        "施加后应为隐身状态"
    );

    super::cloaker::uncloak_path(&path_str).expect("解除隐身应成功");
    assert!(
        !super::cloaker::is_path_cloaked(&path_str).unwrap(),
        "解除后不应为隐身"
    );

    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_hardware_performance_query() {
    let perf = super::hardware::get_hardware_performance();
    assert!(perf.timestamp > 0, "性能快照应包含采样时间");
    assert!(
        !perf.provider_status.is_empty(),
        "性能快照应包含 Provider 状态"
    );
    assert!(
        perf.memory.usage_percent.value.is_some()
            || perf.memory.usage_percent.quality != super::info::MetricQuality::Good
    );
}

#[test]
fn test_system_tools_list_completeness() {
    let tools = super::sys_tools::get_system_tools_list();
    assert!(tools.len() >= 15, "系统工具库应包含不少于 15 种常用工具");
    assert!(tools.iter().any(|t| t.id == "gpedit"), "应包含组策略编辑器");
    assert!(
        tools.iter().any(|t| t.id == "regedit"),
        "应包含注册表编辑器"
    );
    assert!(tools.iter().any(|t| t.id == "compmgmt"), "应包含计算机管理");
}

#[test]
fn test_system_tweaks_list() {
    let tweaks = super::tweaks::get_all_tweaks();
    assert!(tweaks.len() >= 5, "系统调优项目应至少包含 5 项");
    assert!(
        tweaks.iter().any(|t| t.id == "windows_update"),
        "应包含 Windows 更新开关"
    );
    assert!(
        tweaks.iter().any(|t| t.id == "windows_defender"),
        "应包含 Defender 开关"
    );
}

#[test]
fn test_collect_full_system_report() {
    let report = super::info::collect_full_system_report();
    assert!(report.timestamp > 0);
    assert!(!report
        .computer
        .hostname
        .value
        .as_deref()
        .unwrap_or("")
        .is_empty());
    assert!(!report.os.name.value.as_deref().unwrap_or("").is_empty());
    assert!(!report
        .cpu_static
        .name
        .value
        .as_deref()
        .unwrap_or("")
        .is_empty());
    assert!(!report.gpus.is_empty(), "应至少检测到一个 GPU 设备");
    assert!(
        !report.storage.physical_disks.is_empty(),
        "应至少检测到一个物理驱动器"
    );
    assert!(!report.storage.volumes.is_empty(), "应至少检测到一个逻辑卷");
    assert!(
        !report.network.adapters.is_empty(),
        "应至少检测到一个网络适配器"
    );
}

#[test]
fn test_export_and_sanitize_report() {
    let json_raw = super::info::export_system_report_json(false).expect("导出原始报告应成功");
    assert!(!json_raw.is_empty());

    let json_sanitized = super::info::export_system_report_json(true).expect("导出脱敏报告应成功");
    assert!(!json_sanitized.is_empty());
}

#[test]
fn rich_snapshot_serializes_null_quality_and_collection_status() {
    let metric =
        super::info::MetricValue::<u64>::unsupported_at("Bytes", "fixture", "missing", 1000);
    let status = super::info::CollectionStatus {
        quality: super::info::MetricQuality::Unsupported,
        source: "fixture".to_string(),
        timestamp: 1000,
        item_count: Some(0),
        truncated: false,
        error: Some("not supported".to_string()),
    };
    let json = serde_json::to_string(&(metric, status)).expect("rich JSON should serialize");
    assert!(json.contains("\"value\":null"));
    assert!(json.contains("\"quality\":\"Unsupported\""));
    assert!(json.contains("\"item_count\":0"));
}

#[test]
fn rich_performance_contract_round_trips_unsupported_metrics_and_status() {
    let timestamp = 1000;
    let unsupported_u64 = || {
        super::info::MetricValue::<u64>::unsupported_at("Bytes", "fixture", "missing", timestamp)
    };
    let unsupported_frequency = || -> super::info::MetricValue<u32> {
        super::info::MetricValue::unsupported_at("MHz", "fixture", "missing", timestamp)
    };
    let unsupported_f64 =
        || super::info::MetricValue::<f64>::unsupported_at("%", "fixture", "missing", timestamp);
    let unsupported_text =
        || super::info::MetricValue::<String>::unsupported_at("", "fixture", "missing", timestamp);
    let provider_status = super::info::CollectionStatus {
        quality: super::info::MetricQuality::Unsupported,
        source: "fixture".to_string(),
        timestamp,
        item_count: Some(0),
        truncated: false,
        error: Some("not supported".to_string()),
    };
    let report = super::info::collect_full_system_report();
    assert!(!report.provider_status.is_empty());

    let performance = super::hardware::HardwarePerformance {
        timestamp,
        cpu: super::info::CpuRuntimeInfo {
            total_usage_percent: unsupported_f64(),
            user_usage_percent: unsupported_f64(),
            kernel_usage_percent: unsupported_f64(),
            idle_percent: unsupported_f64(),
            base_frequency_mhz: unsupported_frequency(),
            current_frequency_mhz: unsupported_frequency(),
            package_temperature_c: unsupported_f64(),
            package_power_watts: unsupported_f64(),
        },
        memory: super::info::SystemMemoryInfo {
            total_physical_bytes: unsupported_u64(),
            available_physical_bytes: unsupported_u64(),
            used_physical_bytes: unsupported_u64(),
            usage_percent: unsupported_f64(),
            total_page_file_bytes: unsupported_u64(),
            available_page_file_bytes: unsupported_u64(),
            total_virtual_bytes: unsupported_u64(),
            available_virtual_bytes: unsupported_u64(),
            committed_bytes: unsupported_u64(),
            commit_limit_bytes: unsupported_u64(),
            paged_pool_bytes: unsupported_u64(),
            non_paged_pool_bytes: unsupported_u64(),
            hardware_reserved_bytes: unsupported_u64(),
            dimms: Vec::new(),
            provider_status: provider_status.clone(),
        },
        gpus: Vec::new(),
        disks: vec![super::hardware::RuntimeDiskInfo {
            id: "disk-fixture".to_string(),
            drive_letter: unsupported_text(),
            label: unsupported_text(),
            file_system: unsupported_text(),
            total_bytes: unsupported_u64(),
            available_bytes: unsupported_u64(),
            used_bytes: unsupported_u64(),
            usage_percent: unsupported_f64(),
            active_percent: unsupported_f64(),
            read_bytes_per_sec: unsupported_u64(),
            write_bytes_per_sec: unsupported_u64(),
            queue_length: unsupported_f64(),
        }],
        network: Vec::new(),
        provider_status: vec![provider_status],
    };

    let json = serde_json::to_string(&performance).expect("HardwarePerformance 应可序列化");
    assert!(json.contains("\"value\":null"));
    let decoded: super::hardware::HardwarePerformance =
        serde_json::from_str(&json).expect("HardwarePerformance 应可反序列化");
    assert_eq!(decoded, performance);
    assert_eq!(decoded.disks[0].total_bytes.value, None);
    assert_eq!(decoded.cpu.base_frequency_mhz.value, None);
}

#[test]
fn provider_panic_payload_is_not_exposed_in_status_or_json() {
    let secret_payload = "malicious panic payload";
    let result = super::info::collect_isolated("panic-fixture", (), || {
        panic!("{}", secret_payload);
    });

    assert_eq!(result.status.quality, super::info::MetricQuality::ReadError);
    assert_eq!(
        result.status.error.as_deref(),
        Some("Provider panic; safe fallback returned")
    );
    let json = serde_json::to_string(&result.status).expect("panic status 应可序列化");
    assert!(!json.contains(secret_payload));
}

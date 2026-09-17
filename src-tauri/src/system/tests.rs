//! 系统底层模块单元测试
//!
//! 验证系统采集契约、能力相关的空集合以及进程启动结果。

use super::info::{CollectionStatus, MetricQuality, MetricValue};
use super::*;
use serde_json::Value;
use std::collections::HashSet;
use std::env;

/// 检查单个指标的统一质量与元数据契约。
fn assert_metric_is_well_formed<T>(metric: &MetricValue<T>) {
    assert!(
        metric.source.trim().is_empty() == false,
        "指标 source 不能为空"
    );
    assert!(metric.timestamp > 0, "指标 timestamp 必须为正数");
    assert!(!metric.unit.contains('\0'), "指标 unit 不得包含 NUL 字符");

    match metric.quality {
        MetricQuality::Good | MetricQuality::Estimated => {
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

/// 检查 Provider 集合状态的结构与失败语义。
fn assert_collection_status_is_well_formed(status: &CollectionStatus) {
    assert!(!status.source.trim().is_empty(), "集合 source 不能为空");
    assert!(status.timestamp > 0, "集合 timestamp 必须为正数");
    if !matches!(
        status.quality,
        MetricQuality::Good | MetricQuality::Estimated
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

fn is_metric_object(object: &serde_json::Map<String, Value>) -> bool {
    ["value", "unit", "quality", "source", "timestamp"]
        .iter()
        .all(|key| object.contains_key(*key))
        && object.keys().all(|key| {
            ["value", "unit", "quality", "source", "timestamp", "error"].contains(&key.as_str())
        })
}

fn is_collection_status_object(object: &serde_json::Map<String, Value>) -> bool {
    [
        "quality",
        "source",
        "timestamp",
        "item_count",
        "truncated",
        "error",
    ]
    .iter()
    .all(|key| object.contains_key(*key))
}

/// 递归检查报告中所有 MetricValue 与 CollectionStatus 的 JSON 形状。
fn assert_serialized_contract(value: &Value, path: &str) {
    match value {
        Value::Object(object) => {
            if is_metric_object(object) {
                let metric: MetricValue<Value> = serde_json::from_value(value.clone())
                    .unwrap_or_else(|error| panic!("{path} 不是合法 MetricValue: {error}"));
                assert_metric_is_well_formed(&metric);
                if metric.unit == "%" {
                    if let Some(number) = object.get("value").and_then(Value::as_f64) {
                        assert!((0.0..=100.0).contains(&number), "{path} 百分比超出范围");
                    }
                }
            } else if is_collection_status_object(object) {
                let status: CollectionStatus = serde_json::from_value(value.clone())
                    .unwrap_or_else(|error| panic!("{path} 不是合法 CollectionStatus: {error}"));
                assert_collection_status_is_well_formed(&status);
            }

            for (key, child) in object {
                assert_serialized_contract(child, &format!("{path}.{key}"));
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                assert_serialized_contract(child, &format!("{path}[{index}]"));
            }
        }
        _ => {}
    }
}

/// 检查稳定硬件 ID 的前缀、哈希格式与集合内唯一性。
fn assert_stable_ids<'a, I>(ids: I, prefix: &str)
where
    I: IntoIterator<Item = &'a str>,
{
    let ids: Vec<&str> = ids.into_iter().collect();
    let unique: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "{prefix} 硬件 ID 不得重复");
    for id in ids {
        let suffix = id
            .strip_prefix(&format!("{prefix}-"))
            .unwrap_or_else(|| panic!("硬件 ID {id} 缺少稳定前缀 {prefix}-"));
        assert_eq!(suffix.len(), 16, "硬件 ID {id} 的稳定哈希长度错误");
        assert!(
            suffix
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
            "硬件 ID {id} 的稳定哈希格式错误"
        );
    }
}

/// 验证完整报告可以在硬件能力缺失时保持结构一致。
fn assert_report_metrics_are_consistent(report: &SystemFullReport) {
    assert!(report.timestamp > 0, "报告 timestamp 必须为正数");
    assert!(
        !report.provider_status.is_empty(),
        "报告必须包含 Provider 状态"
    );
    for status in &report.provider_status {
        assert_collection_status_is_well_formed(status);
    }

    let json = serde_json::to_value(report).expect("系统报告应可序列化");
    assert_serialized_contract(&json, "report");
    assert_stable_ids(report.gpus.iter().map(|device| device.id.as_str()), "gpu");
    assert_stable_ids(
        report
            .storage
            .physical_disks
            .iter()
            .map(|disk| disk.id.as_str()),
        "disk",
    );
    assert_stable_ids(
        report
            .media
            .displays
            .iter()
            .map(|display| display.id.as_str()),
        "display",
    );
}

fn wait_for_process_exit(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            // 极短命令可能已自然退出；此时没有残留句柄需要清理。
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
fn test_get_memory_info_validity() {
    let Ok(mem) = get_memory_info() else {
        // Windows API 失败或当前环境不具备能力时，错误返回本身是合法结果。
        return;
    };

    // 不假设机器容量；只验证 API 返回的数值关系与范围。
    assert!(mem.available_ram <= mem.total_ram, "可用内存不应超过总内存");
    assert!(mem.used_ram <= mem.total_ram, "已用内存不应超过总内存");
    assert!(
        (0.0..=100.0).contains(&mem.usage_percent),
        "内存使用百分比应在 0.0-100.0 之间"
    );
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
    wait_for_process_exit(pid);
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
    wait_for_process_exit(pid);
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
    wait_for_process_exit(pid);
}

#[test]
fn test_launch_process_invalid_path() {
    let item = LaunchItem {
        id: "test-invalid-path".to_string(),
        name: "Test Invalid Path".to_string(),
        path: "this_non_existent_executable_12345.exe".to_string(),
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
    wait_for_process_exit(pid);
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
    for status in &perf.provider_status {
        assert_collection_status_is_well_formed(status);
    }

    let json = serde_json::to_value(&perf).expect("性能快照应可序列化");
    assert_serialized_contract(&json, "performance");
    assert_stable_ids(perf.gpus.iter().map(|device| device.id.as_str()), "gpu");
    assert_stable_ids(perf.disks.iter().map(|disk| disk.id.as_str()), "volume");
    assert_stable_ids(
        perf.network.iter().map(|adapter| adapter.id.as_str()),
        "network",
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
    assert_report_metrics_are_consistent(&report);
}

#[test]
fn full_report_accepts_capability_dependent_empty_collections() {
    let mut report = super::info::collect_full_system_report();
    report.gpus.clear();
    report.storage.physical_disks.clear();
    report.storage.volumes.clear();
    report.network.adapters.clear();
    report.media.displays.clear();
    report.media.audio_devices.clear();
    report.devices.usb_devices.clear();
    report.devices.pci_devices.clear();
    report.devices.other_pnp_devices.clear();
    report.memory.dimms.clear();
    report.windows_env.startup_items.clear();
    report.windows_env.installed_apps_sample.clear();
    report.windows_env.active_services_sample.clear();
    report.processes.top_memory_processes.clear();
    report.dev_env.tools.clear();
    report.diagnostics.recent_crash_dumps.clear();
    report.diagnostics.whea_hardware_events.clear();

    assert_report_metrics_are_consistent(&report);
}

#[test]
fn test_export_and_sanitize_report() {
    let json_raw = super::info::export_system_report_json(false).expect("导出原始报告应成功");
    assert!(!json_raw.is_empty());

    let json_sanitized = super::info::export_system_report_json(true).expect("导出脱敏报告应成功");
    assert!(!json_sanitized.is_empty());
}

#[test]
fn sanitizer_fixture_removes_sensitive_values_and_preserves_metric_metadata() {
    let raw = r#"{
        "username": "fixture-username-12",
        "mac_address": "02:11:22:33:44:55",
        "ipv4_address": "192.0.2.10",
        "ssid": "fixture-ssid-12",
        "serial_number": {
            "value": "fixture-serial-12",
            "unit": "Bytes",
            "quality": "Good",
            "source": "fixture-provider",
            "timestamp": 1700000000000,
            "error": null
        },
        "uuid": "fixture-uuid-12",
        "hardware_id": "fixture-hardware-12",
        "startup_token": "fixture-startup-token-12"
    }"#;
    let sanitized = super::info::sanitize_report_json(raw);
    for secret in [
        "fixture-username-12",
        "02:11:22:33:44:55",
        "192.0.2.10",
        "fixture-ssid-12",
        "fixture-serial-12",
        "fixture-uuid-12",
        "fixture-hardware-12",
        "fixture-startup-token-12",
    ] {
        assert!(
            !sanitized.contains(secret),
            "脱敏结果泄漏 fixture 值 {secret}"
        );
    }

    let value: Value = serde_json::from_str(&sanitized).expect("脱敏结果应保持合法 JSON");
    assert!(value["serial_number"]["value"].is_null());
    assert_eq!(value["serial_number"]["unit"], "Bytes");
    assert_eq!(value["serial_number"]["quality"], "Good");
    assert_eq!(value["serial_number"]["source"], "fixture-provider");
    assert_eq!(value["serial_number"]["timestamp"], 1700000000000_u64);
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

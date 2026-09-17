//! Windows 系统硬件与运行状态信息综合采集体系。
//!
//! 所有 Provider 都在聚合边界独立隔离；单项失败只产生带质量状态的安全空值，
//! 不会让全量报告或 IPC 命令失败。

pub mod battery_power;
pub mod computer_os;
pub mod cpu;
pub mod dev_env;
pub mod devices;
pub mod diagnostics;
pub mod display_audio;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod processes;
pub mod quality;
pub mod sanitizer;
pub mod storage;
pub mod windows_env;

pub use battery_power::{collect_battery_power, BatteryPowerSnapshot};
pub use computer_os::{
    collect_bios_info, collect_computer_info, collect_motherboard_info, collect_windows_os_info,
    BiosInfo, ComputerInfo, MotherboardInfo, WindowsOsInfo,
};
pub use cpu::{collect_cpu_runtime_info, collect_cpu_static_info, CpuRuntimeInfo, CpuStaticInfo};
pub use dev_env::{collect_dev_environment, DevEnvironmentSnapshot, DevToolEntry};
pub use devices::{collect_devices_snapshot, DevicesSnapshot, PnpDeviceEntry};
pub use diagnostics::{collect_diagnostics_snapshot, DiagnosticsSnapshot};
pub use display_audio::{
    collect_audio_devices, collect_displays, collect_media_snapshot, AudioDeviceInfo,
    DisplayDevice, MediaDevicesSnapshot,
};
pub use gpu::{collect_gpu_devices, GpuDevice};
pub use memory::{collect_memory_info, DimmModule, SystemMemoryInfo};
pub use network::{
    collect_connections_summary, collect_network_adapters, collect_network_snapshot,
    collect_wifi_status, NetworkAdapterInfo, NetworkConnectionsSummary, NetworkSnapshot,
    WiFiConnectionInfo,
};
pub use processes::{collect_processes_snapshot, ProcessSnapshot, ProcessSummaryItem};
pub use quality::{
    classify_win32_error, current_timestamp_ms, stable_device_id, CollectionStatus,
    CollectorResult, MetricQuality, MetricValue,
};
pub use sanitizer::sanitize_report_json;
pub use storage::{
    collect_logical_volumes, collect_physical_disks, collect_storage_snapshot, NvmeHealthInfo,
    PhysicalDiskInfo, StorageSnapshot, VolumeInfo,
};
pub use windows_env::{
    collect_windows_env, InstalledAppEntry, SecurityStatusInfo, StartupEntry, WindowsEnvSnapshot,
};

use serde::{Deserialize, Serialize};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// 在 Provider 边界捕获 panic，并返回明确的错误状态与调用方提供的安全 fallback。
pub(crate) fn collect_isolated<T, F>(source: &str, fallback: T, f: F) -> CollectorResult<T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    let timestamp = current_timestamp_ms();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => CollectorResult {
            value,
            status: CollectionStatus {
                quality: MetricQuality::Good,
                source: source.to_string(),
                timestamp,
                item_count: None,
                truncated: false,
                error: None,
            },
        },
        Err(payload) => CollectorResult {
            value: fallback,
            status: CollectionStatus {
                quality: MetricQuality::ReadError,
                source: source.to_string(),
                timestamp,
                item_count: Some(0),
                truncated: false,
                error: Some(panic_reason(payload)),
            },
        },
    }
}

fn panic_reason(_payload: Box<dyn std::any::Any + Send>) -> String {
    "Provider panic; safe fallback returned".to_string()
}

fn empty_metric<T>(source: &str, timestamp: u64) -> MetricValue<T> {
    MetricValue::read_error_at("", source, "Provider panic fallback", timestamp)
}

fn empty_status(source: &str, timestamp: u64) -> CollectionStatus {
    CollectionStatus {
        quality: MetricQuality::ReadError,
        source: source.to_string(),
        timestamp,
        item_count: Some(0),
        truncated: false,
        error: Some("Provider panic fallback".to_string()),
    }
}

fn set_count(mut status: CollectionStatus, count: usize) -> CollectionStatus {
    status.item_count = Some(u32::try_from(count).unwrap_or(u32::MAX));
    status
}

fn empty_computer(timestamp: u64) -> ComputerInfo {
    let text = || empty_metric("ComputerInfo", timestamp);
    let number = || empty_metric("ComputerInfo", timestamp);
    ComputerInfo {
        hostname: text(),
        dns_hostname: text(),
        manufacturer: text(),
        model: text(),
        system_family: text(),
        serial_number: text(),
        uuid: text(),
        domain: text(),
        workgroup: text(),
        system_type: text(),
        current_user: text(),
        uptime_seconds: number(),
        uptime_formatted: text(),
    }
}

fn empty_motherboard(timestamp: u64) -> MotherboardInfo {
    let metric = || empty_metric("MotherboardInfo", timestamp);
    MotherboardInfo {
        manufacturer: metric(),
        product: metric(),
        version: metric(),
        serial_number: metric(),
        chipset: metric(),
    }
}

fn empty_bios(timestamp: u64) -> BiosInfo {
    let metric = || empty_metric("BiosInfo", timestamp);
    BiosInfo {
        vendor: metric(),
        version: metric(),
        release_date: metric(),
        smbios_version: metric(),
        firmware_mode: metric(),
    }
}

fn empty_os(timestamp: u64) -> WindowsOsInfo {
    let text = || empty_metric("WindowsOsInfo", timestamp);
    WindowsOsInfo {
        name: text(),
        edition: text(),
        display_version: text(),
        build_number: text(),
        ubr: empty_metric("WindowsOsInfo", timestamp),
        architecture: text(),
        install_date: text(),
        windows_directory: text(),
        system_directory: text(),
        system_drive: text(),
        locale: text(),
        timezone: text(),
    }
}

fn empty_cpu_static(timestamp: u64) -> CpuStaticInfo {
    CpuStaticInfo {
        name: empty_metric("CPU static", timestamp),
        vendor: empty_metric("CPU static", timestamp),
        brand: empty_metric("CPU static", timestamp),
        family: empty_metric("CPU static", timestamp),
        model: empty_metric("CPU static", timestamp),
        stepping: empty_metric("CPU static", timestamp),
        physical_cores: empty_metric("CPU static", timestamp),
        logical_processors: empty_metric("CPU static", timestamp),
        l1_data_cache_kb: empty_metric("CPU static", timestamp),
        l1_inst_cache_kb: empty_metric("CPU static", timestamp),
        l2_cache_kb: empty_metric("CPU static", timestamp),
        l3_cache_kb: empty_metric("CPU static", timestamp),
        features: Vec::new(),
        virtualization: empty_metric("CPU static", timestamp),
    }
}

fn empty_cpu_runtime(timestamp: u64) -> CpuRuntimeInfo {
    CpuRuntimeInfo {
        total_usage_percent: empty_metric("CPU runtime", timestamp),
        user_usage_percent: empty_metric("CPU runtime", timestamp),
        kernel_usage_percent: empty_metric("CPU runtime", timestamp),
        idle_percent: empty_metric("CPU runtime", timestamp),
        base_frequency_mhz: empty_metric("CPU runtime", timestamp),
        current_frequency_mhz: empty_metric("CPU runtime", timestamp),
        package_temperature_c: empty_metric("CPU runtime", timestamp),
        package_power_watts: empty_metric("CPU runtime", timestamp),
    }
}

fn empty_memory(timestamp: u64) -> SystemMemoryInfo {
    let metric = || empty_metric("Memory", timestamp);
    SystemMemoryInfo {
        total_physical_bytes: metric(),
        available_physical_bytes: metric(),
        used_physical_bytes: metric(),
        usage_percent: metric(),
        total_page_file_bytes: metric(),
        available_page_file_bytes: metric(),
        total_virtual_bytes: metric(),
        available_virtual_bytes: metric(),
        committed_bytes: metric(),
        commit_limit_bytes: metric(),
        paged_pool_bytes: metric(),
        non_paged_pool_bytes: metric(),
        hardware_reserved_bytes: metric(),
        dimms: Vec::new(),
        provider_status: empty_status("DIMM provider", timestamp),
    }
}

fn empty_network(timestamp: u64) -> NetworkSnapshot {
    NetworkSnapshot {
        adapters: Vec::new(),
        wifi_info: WiFiConnectionInfo {
            is_connected: empty_metric("Network", timestamp),
            ssid: empty_metric("Network", timestamp),
            bssid: empty_metric("Network", timestamp),
            signal_quality_percent: empty_metric("Network", timestamp),
            rssi_dbm: empty_metric("Network", timestamp),
            channel: empty_metric("Network", timestamp),
            radio_frequency_ghz: empty_metric("Network", timestamp),
            security_cipher: empty_metric("Network", timestamp),
        },
        connections_summary: NetworkConnectionsSummary {
            tcp_established_count: empty_metric("Network", timestamp),
            tcp_listening_count: empty_metric("Network", timestamp),
            tcp_time_wait_count: empty_metric("Network", timestamp),
            tcp_total_connections: empty_metric("Network", timestamp),
            udp_endpoints_count: empty_metric("Network", timestamp),
        },
    }
}

fn empty_battery(timestamp: u64) -> BatteryPowerSnapshot {
    BatteryPowerSnapshot {
        has_battery: empty_metric("Battery", timestamp),
        ac_connected: empty_metric("Battery", timestamp),
        battery_percent: empty_metric("Battery", timestamp),
        charging_status: empty_metric("Battery", timestamp),
        estimated_runtime_minutes: empty_metric("Battery", timestamp),
        power_scheme: empty_metric("Battery", timestamp),
    }
}

fn empty_windows_env(timestamp: u64) -> WindowsEnvSnapshot {
    WindowsEnvSnapshot {
        startup_items: Vec::new(),
        installed_apps_sample: Vec::new(),
        security_status: SecurityStatusInfo {
            secure_boot_enabled: empty_metric("Windows environment", timestamp),
            tpm_present: empty_metric("Windows environment", timestamp),
            tpm_version: empty_metric("Windows environment", timestamp),
            defender_enabled: empty_metric("Windows environment", timestamp),
            defender_realtime_protection: empty_metric("Windows environment", timestamp),
            firewall_domain_enabled: empty_metric("Windows environment", timestamp),
            firewall_private_enabled: empty_metric("Windows environment", timestamp),
            firewall_public_enabled: empty_metric("Windows environment", timestamp),
        },
        active_services_sample: Vec::new(),
    }
}

fn empty_diagnostics(timestamp: u64) -> DiagnosticsSnapshot {
    DiagnosticsSnapshot {
        minidump_count: empty_metric("Diagnostics", timestamp),
        recent_crash_dumps: Vec::new(),
        minidump_truncated: false,
        unexpected_shutdowns_count: empty_metric("Diagnostics", timestamp),
        whea_hardware_events: Vec::new(),
        whea_hardware_events_status: empty_status("Windows_EventLog", timestamp),
        overall_health_assessment: empty_metric("Diagnostics", timestamp),
    }
}

/// 全量系统与硬件全景诊断报告。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SystemFullReport {
    pub computer: ComputerInfo,
    pub motherboard: MotherboardInfo,
    pub bios: BiosInfo,
    pub os: WindowsOsInfo,
    pub cpu_static: CpuStaticInfo,
    pub cpu_runtime: CpuRuntimeInfo,
    pub gpus: Vec<GpuDevice>,
    pub memory: SystemMemoryInfo,
    pub storage: StorageSnapshot,
    pub network: NetworkSnapshot,
    pub media: MediaDevicesSnapshot,
    pub devices: DevicesSnapshot,
    pub battery_power: BatteryPowerSnapshot,
    pub processes: ProcessSnapshot,
    pub windows_env: WindowsEnvSnapshot,
    pub dev_env: DevEnvironmentSnapshot,
    pub diagnostics: DiagnosticsSnapshot,
    pub timestamp: u64,
    pub provider_status: Vec<CollectionStatus>,
}

/// 采集全量硬件与系统诊断报告；各 Provider 均独立隔离。
pub fn collect_full_system_report() -> SystemFullReport {
    let timestamp = current_timestamp_ms();
    let computer = collect_isolated(
        "ComputerInfo",
        empty_computer(timestamp),
        collect_computer_info,
    );
    let motherboard = collect_isolated(
        "MotherboardInfo",
        empty_motherboard(timestamp),
        collect_motherboard_info,
    );
    let bios = collect_isolated("BiosInfo", empty_bios(timestamp), collect_bios_info);
    let os = collect_isolated(
        "WindowsOsInfo",
        empty_os(timestamp),
        collect_windows_os_info,
    );
    let cpu_static = collect_isolated(
        "CPU static",
        empty_cpu_static(timestamp),
        collect_cpu_static_info,
    );
    let cpu_runtime = collect_isolated(
        "CPU runtime",
        empty_cpu_runtime(timestamp),
        collect_cpu_runtime_info,
    );
    let gpus = collect_isolated("GPU", Vec::new(), collect_gpu_devices);
    let memory = collect_isolated("Memory", empty_memory(timestamp), collect_memory_info);
    let storage = collect_isolated(
        "Storage",
        StorageSnapshot {
            physical_disks: Vec::new(),
            volumes: Vec::new(),
        },
        collect_storage_snapshot,
    );
    let network = collect_isolated(
        "Network",
        empty_network(timestamp),
        collect_network_snapshot,
    );
    let media = collect_isolated(
        "Media",
        MediaDevicesSnapshot {
            displays: Vec::new(),
            audio_devices: Vec::new(),
        },
        collect_media_snapshot,
    );
    let devices = collect_isolated(
        "Devices",
        DevicesSnapshot {
            usb_devices: Vec::new(),
            pci_devices: Vec::new(),
            other_pnp_devices: Vec::new(),
        },
        collect_devices_snapshot,
    );
    let battery = collect_isolated("Battery", empty_battery(timestamp), collect_battery_power);
    let processes = collect_isolated(
        "Processes",
        ProcessSnapshot {
            total_processes: 0,
            total_threads: 0,
            top_memory_processes: Vec::new(),
        },
        collect_processes_snapshot,
    );
    let windows_env = collect_isolated(
        "Windows environment",
        empty_windows_env(timestamp),
        collect_windows_env,
    );
    let dev_env = collect_isolated(
        "Development environment",
        DevEnvironmentSnapshot { tools: Vec::new() },
        collect_dev_environment,
    );
    let diagnostics = collect_isolated(
        "Diagnostics",
        empty_diagnostics(timestamp),
        collect_diagnostics_snapshot,
    );

    let memory_status = memory.value.provider_status.clone();
    let provider_status = vec![
        computer.status,
        motherboard.status,
        bios.status,
        os.status,
        cpu_static.status,
        cpu_runtime.status,
        set_count(gpus.status, gpus.value.len()),
        memory.status,
        memory_status,
        set_count(
            storage.status,
            storage.value.physical_disks.len() + storage.value.volumes.len(),
        ),
        set_count(network.status, network.value.adapters.len()),
        set_count(
            media.status,
            media.value.displays.len() + media.value.audio_devices.len(),
        ),
        set_count(
            devices.status,
            devices.value.usb_devices.len()
                + devices.value.pci_devices.len()
                + devices.value.other_pnp_devices.len(),
        ),
        battery.status,
        set_count(processes.status, processes.value.top_memory_processes.len()),
        set_count(
            windows_env.status,
            windows_env.value.startup_items.len()
                + windows_env.value.installed_apps_sample.len()
                + windows_env.value.active_services_sample.len(),
        ),
        set_count(dev_env.status, dev_env.value.tools.len()),
        set_count(
            diagnostics.status,
            diagnostics.value.recent_crash_dumps.len()
                + diagnostics.value.whea_hardware_events.len(),
        ),
    ];

    SystemFullReport {
        computer: computer.value,
        motherboard: motherboard.value,
        bios: bios.value,
        os: os.value,
        cpu_static: cpu_static.value,
        cpu_runtime: cpu_runtime.value,
        gpus: gpus.value,
        memory: memory.value,
        storage: storage.value,
        network: network.value,
        media: media.value,
        devices: devices.value,
        battery_power: battery.value,
        processes: processes.value,
        windows_env: windows_env.value,
        dev_env: dev_env.value,
        diagnostics: diagnostics.value,
        timestamp,
        provider_status,
    }
}

/// 导出格式化的 JSON 报告字符串，支持按需脱敏。
pub fn export_system_report_json(sanitize: bool) -> Result<String, String> {
    let report = collect_full_system_report();
    let json =
        serde_json::to_string_pretty(&report).map_err(|e| format!("序列化系统报告失败: {e}"))?;

    if sanitize {
        Ok(sanitize_report_json(&json))
    } else {
        Ok(json)
    }
}

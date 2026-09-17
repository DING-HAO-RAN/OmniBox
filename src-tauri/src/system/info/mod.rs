//! Windows 系统硬件与运行状态信息综合采集体系 (System Info Collector Architecture)
//!
//! 整合整机、操作系统、CPU、多显卡 GPU、物理内存 DIMM、NVMe 存储、网络 Wi-Fi、
//! 显示音频外设、电池电源、进程调度、系统环境与诊断日志。
//! 遵循原则：只读、低开销、可降级、可追踪、不伪造、严格隐私保护。

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
pub mod quality;
pub mod processes;
pub mod sanitizer;
pub mod storage;
pub mod windows_env;

pub use battery_power::{collect_battery_power, BatteryPowerSnapshot};
pub use computer_os::{
    collect_bios_info, collect_computer_info, collect_motherboard_info, collect_windows_os_info,
    BiosInfo, ComputerInfo, MotherboardInfo, WindowsOsInfo,
};
pub use cpu::{
    collect_cpu_runtime_info, collect_cpu_static_info, CpuRuntimeInfo, CpuStaticInfo,
};
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
    collect_wifi_status, NetworkAdapterInfo, NetworkSnapshot, WiFiConnectionInfo,
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

/// 全量系统与硬件全景诊断报告
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
}

/// 采集全量硬件与系统诊断报告 (每个 Provider 均独立容错，互不影响)
pub fn collect_full_system_report() -> SystemFullReport {
    SystemFullReport {
        computer: collect_computer_info(),
        motherboard: collect_motherboard_info(),
        bios: collect_bios_info(),
        os: collect_windows_os_info(),
        cpu_static: collect_cpu_static_info(),
        cpu_runtime: collect_cpu_runtime_info(),
        gpus: collect_gpu_devices(),
        memory: collect_memory_info(),
        storage: collect_storage_snapshot(),
        network: collect_network_snapshot(),
        media: collect_media_snapshot(),
        devices: collect_devices_snapshot(),
        battery_power: collect_battery_power(),
        processes: collect_processes_snapshot(),
        windows_env: collect_windows_env(),
        dev_env: collect_dev_environment(),
        diagnostics: collect_diagnostics_snapshot(),
        timestamp: current_timestamp_ms(),
    }
}

/// 导出格式化的 JSON 报告字符串，支持按需脱敏
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

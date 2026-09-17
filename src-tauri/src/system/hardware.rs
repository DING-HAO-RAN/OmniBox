//! 硬件实时性能快照的最终 IPC DTO 与 Provider 投影。
//!
//! 性能数据全部复用 system/info 中已完成的真实 Provider；未接入的动态指标
//! 保持 `Unsupported + None`，不再保留旧的示例性能模型。

use crate::system::info::quality::{
    stable_device_id, CollectionStatus, MetricQuality, MetricValue,
};
use crate::system::info::{
    collect_cpu_runtime_info, collect_memory_info, CpuRuntimeInfo, NetworkAdapterInfo,
    NetworkConnectionsSummary, NetworkSnapshot, StorageSnapshot, SystemMemoryInfo,
    WiFiConnectionInfo,
};
use serde::{Deserialize, Serialize};

/// 逻辑卷的实时性能投影。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuntimeDiskInfo {
    pub id: String,
    pub drive_letter: MetricValue<String>,
    pub label: MetricValue<String>,
    pub file_system: MetricValue<String>,
    pub total_bytes: MetricValue<u64>,
    pub available_bytes: MetricValue<u64>,
    pub used_bytes: MetricValue<u64>,
    pub usage_percent: MetricValue<f64>,
    pub active_percent: MetricValue<f64>,
    pub read_bytes_per_sec: MetricValue<u64>,
    pub write_bytes_per_sec: MetricValue<u64>,
    pub queue_length: MetricValue<f64>,
}

/// 网络适配器的实时吞吐投影。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuntimeNetworkInfo {
    pub id: String,
    pub name: MetricValue<String>,
    pub rx_bytes_total: MetricValue<u64>,
    pub tx_bytes_total: MetricValue<u64>,
    pub rx_bytes_per_sec: MetricValue<u64>,
    pub tx_bytes_per_sec: MetricValue<u64>,
}

/// 统一硬件性能快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HardwarePerformance {
    pub timestamp: u64,
    pub cpu: CpuRuntimeInfo,
    pub memory: SystemMemoryInfo,
    pub gpus: Vec<crate::system::info::GpuDevice>,
    pub disks: Vec<RuntimeDiskInfo>,
    pub network: Vec<RuntimeNetworkInfo>,
    pub provider_status: Vec<CollectionStatus>,
}

fn unsupported<T>(unit: &str, source: &str, reason: &str, timestamp: u64) -> MetricValue<T> {
    MetricValue::unsupported_at(unit, source, reason, timestamp)
}

fn retimestamp_metric<T>(metric: MetricValue<T>, timestamp: u64) -> MetricValue<T> {
    MetricValue {
        timestamp,
        ..metric
    }
}

fn retimestamp_cpu(mut cpu: CpuRuntimeInfo, timestamp: u64) -> CpuRuntimeInfo {
    cpu.total_usage_percent = retimestamp_metric(cpu.total_usage_percent, timestamp);
    cpu.user_usage_percent = retimestamp_metric(cpu.user_usage_percent, timestamp);
    cpu.kernel_usage_percent = retimestamp_metric(cpu.kernel_usage_percent, timestamp);
    cpu.idle_percent = retimestamp_metric(cpu.idle_percent, timestamp);
    cpu.base_frequency_mhz = retimestamp_metric(cpu.base_frequency_mhz, timestamp);
    cpu.current_frequency_mhz = retimestamp_metric(cpu.current_frequency_mhz, timestamp);
    cpu.package_temperature_c = retimestamp_metric(cpu.package_temperature_c, timestamp);
    cpu.package_power_watts = retimestamp_metric(cpu.package_power_watts, timestamp);
    cpu
}

fn retimestamp_memory(mut memory: SystemMemoryInfo, timestamp: u64) -> SystemMemoryInfo {
    memory.total_physical_bytes = retimestamp_metric(memory.total_physical_bytes, timestamp);
    memory.available_physical_bytes =
        retimestamp_metric(memory.available_physical_bytes, timestamp);
    memory.used_physical_bytes = retimestamp_metric(memory.used_physical_bytes, timestamp);
    memory.usage_percent = retimestamp_metric(memory.usage_percent, timestamp);
    memory.total_page_file_bytes = retimestamp_metric(memory.total_page_file_bytes, timestamp);
    memory.available_page_file_bytes =
        retimestamp_metric(memory.available_page_file_bytes, timestamp);
    memory.total_virtual_bytes = retimestamp_metric(memory.total_virtual_bytes, timestamp);
    memory.available_virtual_bytes = retimestamp_metric(memory.available_virtual_bytes, timestamp);
    memory.committed_bytes = retimestamp_metric(memory.committed_bytes, timestamp);
    memory.commit_limit_bytes = retimestamp_metric(memory.commit_limit_bytes, timestamp);
    memory.paged_pool_bytes = retimestamp_metric(memory.paged_pool_bytes, timestamp);
    memory.non_paged_pool_bytes = retimestamp_metric(memory.non_paged_pool_bytes, timestamp);
    memory.hardware_reserved_bytes = retimestamp_metric(memory.hardware_reserved_bytes, timestamp);
    memory.provider_status.timestamp = timestamp;
    memory
}

fn empty_cpu_runtime(timestamp: u64) -> CpuRuntimeInfo {
    CpuRuntimeInfo {
        total_usage_percent: unsupported("%", "CPU provider", "Provider panic fallback", timestamp),
        user_usage_percent: unsupported("%", "CPU provider", "Provider panic fallback", timestamp),
        kernel_usage_percent: unsupported(
            "%",
            "CPU provider",
            "Provider panic fallback",
            timestamp,
        ),
        idle_percent: unsupported("%", "CPU provider", "Provider panic fallback", timestamp),
        base_frequency_mhz: unsupported(
            "MHz",
            "CPU provider",
            "Provider panic fallback",
            timestamp,
        ),
        current_frequency_mhz: unsupported(
            "MHz",
            "CPU provider",
            "Provider panic fallback",
            timestamp,
        ),
        package_temperature_c: unsupported(
            "°C",
            "CPU provider",
            "Provider panic fallback",
            timestamp,
        ),
        package_power_watts: unsupported("W", "CPU provider", "Provider panic fallback", timestamp),
    }
}

fn empty_memory(timestamp: u64) -> SystemMemoryInfo {
    let bytes = || {
        unsupported(
            "Bytes",
            "Memory provider",
            "Provider panic fallback",
            timestamp,
        )
    };
    let percent = || unsupported("%", "Memory provider", "Provider panic fallback", timestamp);
    SystemMemoryInfo {
        total_physical_bytes: bytes(),
        available_physical_bytes: bytes(),
        used_physical_bytes: bytes(),
        usage_percent: percent(),
        total_page_file_bytes: bytes(),
        available_page_file_bytes: bytes(),
        total_virtual_bytes: bytes(),
        available_virtual_bytes: bytes(),
        committed_bytes: bytes(),
        commit_limit_bytes: bytes(),
        paged_pool_bytes: bytes(),
        non_paged_pool_bytes: bytes(),
        hardware_reserved_bytes: bytes(),
        dimms: Vec::new(),
        provider_status: CollectionStatus {
            quality: MetricQuality::ReadError,
            source: "Memory provider".to_string(),
            timestamp,
            item_count: Some(0),
            truncated: false,
            error: Some("Provider panic fallback".to_string()),
        },
    }
}

fn runtime_disk_from_volume(
    volume: &crate::system::info::VolumeInfo,
    timestamp: u64,
) -> RuntimeDiskInfo {
    let source = "Storage_VolumeInfo";
    RuntimeDiskInfo {
        id: stable_device_id("volume", &[&volume.drive_letter]),
        drive_letter: MetricValue::good_at(volume.drive_letter.clone(), "", source, timestamp),
        label: MetricValue::good_at(volume.label.clone(), "", source, timestamp),
        file_system: MetricValue::good_at(volume.file_system.clone(), "", source, timestamp),
        total_bytes: MetricValue::good_at(volume.total_bytes, "Bytes", source, timestamp),
        available_bytes: MetricValue::good_at(volume.available_bytes, "Bytes", source, timestamp),
        used_bytes: MetricValue::good_at(volume.used_bytes, "Bytes", source, timestamp),
        usage_percent: MetricValue::good_at(volume.usage_percent, "%", source, timestamp),
        active_percent: unsupported(
            "%",
            "RuntimeDiskProvider",
            "当前没有磁盘 active provider",
            timestamp,
        ),
        read_bytes_per_sec: unsupported(
            "Bytes/s",
            "RuntimeDiskProvider",
            "当前没有磁盘 read provider",
            timestamp,
        ),
        write_bytes_per_sec: unsupported(
            "Bytes/s",
            "RuntimeDiskProvider",
            "当前没有磁盘 write provider",
            timestamp,
        ),
        queue_length: unsupported(
            "",
            "RuntimeDiskProvider",
            "当前没有磁盘 queue provider",
            timestamp,
        ),
    }
}

fn empty_network_snapshot(timestamp: u64) -> NetworkSnapshot {
    NetworkSnapshot {
        adapters: Vec::new(),
        wifi_info: WiFiConnectionInfo {
            is_connected: unsupported("", "Network provider", "Provider panic fallback", timestamp),
            ssid: unsupported("", "Network provider", "Provider panic fallback", timestamp),
            bssid: unsupported("", "Network provider", "Provider panic fallback", timestamp),
            signal_quality_percent: unsupported(
                "%",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            rssi_dbm: unsupported(
                "dBm",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            channel: unsupported("", "Network provider", "Provider panic fallback", timestamp),
            radio_frequency_ghz: unsupported(
                "GHz",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            security_cipher: unsupported(
                "",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
        },
        connections_summary: NetworkConnectionsSummary {
            tcp_established_count: unsupported(
                "connections",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            tcp_listening_count: unsupported(
                "connections",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            tcp_time_wait_count: unsupported(
                "connections",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            tcp_total_connections: unsupported(
                "connections",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
            udp_endpoints_count: unsupported(
                "endpoints",
                "Network provider",
                "Provider panic fallback",
                timestamp,
            ),
        },
    }
}

fn runtime_network_from_adapter(
    adapter: &NetworkAdapterInfo,
    timestamp: u64,
) -> RuntimeNetworkInfo {
    // 现有 DTO 未暴露 LUID，使用真实接口索引和描述构造稳定 ID；限制写入 Provider 状态。
    RuntimeNetworkInfo {
        id: stable_device_id(
            "network",
            &[&adapter.index.to_string(), &adapter.description],
        ),
        name: MetricValue::good_at(adapter.name.clone(), "", "IP_Helper", timestamp),
        rx_bytes_total: retimestamp_metric(adapter.total_rx_bytes.clone(), timestamp),
        tx_bytes_total: retimestamp_metric(adapter.total_tx_bytes.clone(), timestamp),
        rx_bytes_per_sec: retimestamp_metric(adapter.rx_speed_bps.clone(), timestamp),
        tx_bytes_per_sec: retimestamp_metric(adapter.tx_speed_bps.clone(), timestamp),
    }
}

/// 采集统一硬件性能快照；单个 Provider 失败不会让 IPC 命令失败。
pub fn get_hardware_performance() -> HardwarePerformance {
    let timestamp = crate::system::info::current_timestamp_ms();
    let cpu = crate::system::info::collect_isolated(
        "CPU runtime",
        empty_cpu_runtime(timestamp),
        collect_cpu_runtime_info,
    );
    let memory = crate::system::info::collect_isolated(
        "Memory",
        empty_memory(timestamp),
        collect_memory_info,
    );
    let gpus = crate::system::info::collect_isolated_with_status(
        "GPU",
        Vec::new(),
        crate::system::info::gpu::collect_gpu_devices_with_status,
    );
    let storage = crate::system::info::collect_isolated_with_status(
        "Storage",
        StorageSnapshot {
            physical_disks: Vec::new(),
            volumes: Vec::new(),
        },
        crate::system::info::storage::collect_storage_snapshot_with_status,
    );
    let network = crate::system::info::collect_isolated_with_status(
        "Network",
        empty_network_snapshot(timestamp),
        crate::system::info::network::collect_network_snapshot_with_status,
    );

    let mut provider_status = vec![
        cpu.status,
        memory.status,
        memory.value.provider_status.clone(),
        gpus.status,
        storage.status,
        network.status,
    ];

    let memory_value = retimestamp_memory(memory.value, timestamp);
    let cpu_value = retimestamp_cpu(cpu.value, timestamp);
    let disks = storage
        .value
        .volumes
        .iter()
        .map(|volume| runtime_disk_from_volume(volume, timestamp))
        .collect();
    let network_values = network
        .value
        .adapters
        .iter()
        .map(|adapter| runtime_network_from_adapter(adapter, timestamp))
        .collect();

    // 保证每个返回快照都有可追踪的 Provider 状态。
    provider_status.retain(|status| !status.source.is_empty());
    HardwarePerformance {
        timestamp,
        cpu: cpu_value,
        memory: memory_value,
        gpus: gpus.value,
        disks,
        network: network_values,
        provider_status,
    }
}

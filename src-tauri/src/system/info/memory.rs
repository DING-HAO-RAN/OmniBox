//! 内存运行状态与物理内存条 (DIMM) 硬件模块
//!
//! 采用 `GlobalMemoryStatusEx` 与 `GetPerformanceInfo` 采集实时内存分布；
//! DIMM 硬件信息在尚无可信 Provider 时明确标记为不支持。

use super::quality::{current_timestamp_ms, CollectionStatus, MetricQuality, MetricValue};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows_sys::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/// 物理内存条 (DIMM) 硬件详情
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DimmModule {
    pub slot: String, // e.g. "DIMM 1 (Channel A)"
    pub capacity_bytes: u64,
    pub speed_mhz: u32,
    pub memory_type: String, // "DDR4" | "DDR5"
    pub manufacturer: String,
    pub part_number: String,
    pub serial_number: String,
    pub configured_voltage: f64,
}

/// 全局内存系统完整指标
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SystemMemoryInfo {
    pub total_physical_bytes: MetricValue<u64>,
    pub available_physical_bytes: MetricValue<u64>,
    pub used_physical_bytes: MetricValue<u64>,
    pub usage_percent: MetricValue<f64>,
    pub total_page_file_bytes: MetricValue<u64>,
    pub available_page_file_bytes: MetricValue<u64>,
    pub total_virtual_bytes: MetricValue<u64>,
    pub available_virtual_bytes: MetricValue<u64>,
    pub committed_bytes: MetricValue<u64>,
    pub commit_limit_bytes: MetricValue<u64>,
    pub paged_pool_bytes: MetricValue<u64>,
    pub non_paged_pool_bytes: MetricValue<u64>,
    pub hardware_reserved_bytes: MetricValue<u64>,
    pub dimms: Vec<DimmModule>,
    pub provider_status: CollectionStatus,
}

const GLOBAL_MEMORY_SOURCE: &str = "GlobalMemoryStatusEx";
const DERIVED_MEMORY_SOURCE: &str = "GlobalMemoryStatusEx/derived";
const PERFORMANCE_SOURCE: &str = "GetPerformanceInfo";
const DIMM_PROVIDER_SOURCE: &str = "DIMM provider";

/// 计算物理内存使用率；输入不满足真实推导条件时不返回伪造的 0%。
fn calculate_usage(total: u64, available: u64) -> Option<f64> {
    if total == 0 || available > total {
        return None;
    }

    let used = total - available;
    Some(((used as f64 / total as f64) * 1000.0).round() / 10.0)
}

fn read_error_metric<T>(unit: &str, source: &str, reason: &str, timestamp: u64) -> MetricValue<T> {
    MetricValue::read_error_at(unit, source, reason, timestamp)
}

fn performance_missing_metric<T>(
    unit: &str,
    quality: MetricQuality,
    reason: &str,
    timestamp: u64,
) -> MetricValue<T> {
    match quality {
        MetricQuality::Invalid => {
            MetricValue::invalid_at(unit, PERFORMANCE_SOURCE, reason, timestamp)
        }
        _ => MetricValue::read_error_at(unit, PERFORMANCE_SOURCE, reason, timestamp),
    }
}

/// 采集全局内存运行状态与物理内存条
pub fn collect_memory_info() -> SystemMemoryInfo {
    let timestamp = current_timestamp_ms();
    let mut mem_status = MEMORYSTATUSEX {
        // Win32 要求调用前提供结构体大小，否则读取结果无效。
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    let global_status_succeeded = unsafe { GlobalMemoryStatusEx(&mut mem_status) != 0 };
    let (
        total_physical_bytes,
        available_physical_bytes,
        used_physical_bytes,
        usage_percent,
        total_page_file_bytes,
        available_page_file_bytes,
        total_virtual_bytes,
        available_virtual_bytes,
    ) = if global_status_succeeded {
        let total = mem_status.ullTotalPhys;
        let available = mem_status.ullAvailPhys;
        let (used, usage) = match calculate_usage(total, available) {
            Some(usage) => (
                MetricValue::estimated_at(
                    total - available,
                    "Bytes",
                    DERIVED_MEMORY_SOURCE,
                    timestamp,
                ),
                MetricValue::estimated_at(usage, "%", DERIVED_MEMORY_SOURCE, timestamp),
            ),
            None => (
                MetricValue::invalid_at(
                    "Bytes",
                    DERIVED_MEMORY_SOURCE,
                    "GlobalMemoryStatusEx 返回的物理内存总量或可用量无法推导已用值",
                    timestamp,
                ),
                MetricValue::invalid_at(
                    "%",
                    DERIVED_MEMORY_SOURCE,
                    "GlobalMemoryStatusEx 返回的物理内存总量或可用量无法推导使用率",
                    timestamp,
                ),
            ),
        };

        (
            MetricValue::good_at(total, "Bytes", GLOBAL_MEMORY_SOURCE, timestamp),
            MetricValue::good_at(available, "Bytes", GLOBAL_MEMORY_SOURCE, timestamp),
            used,
            usage,
            MetricValue::good_at(
                mem_status.ullTotalPageFile,
                "Bytes",
                GLOBAL_MEMORY_SOURCE,
                timestamp,
            ),
            MetricValue::good_at(
                mem_status.ullAvailPageFile,
                "Bytes",
                GLOBAL_MEMORY_SOURCE,
                timestamp,
            ),
            MetricValue::good_at(
                mem_status.ullTotalVirtual,
                "Bytes",
                GLOBAL_MEMORY_SOURCE,
                timestamp,
            ),
            MetricValue::good_at(
                mem_status.ullAvailVirtual,
                "Bytes",
                GLOBAL_MEMORY_SOURCE,
                timestamp,
            ),
        )
    } else {
        let reason = "GlobalMemoryStatusEx 调用失败，无法读取全局内存状态";
        (
            read_error_metric::<u64>("Bytes", GLOBAL_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<u64>("Bytes", GLOBAL_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<u64>("Bytes", DERIVED_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<f64>("%", DERIVED_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<u64>("Bytes", GLOBAL_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<u64>("Bytes", GLOBAL_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<u64>("Bytes", GLOBAL_MEMORY_SOURCE, reason, timestamp),
            read_error_metric::<u64>("Bytes", GLOBAL_MEMORY_SOURCE, reason, timestamp),
        )
    };

    let mut perf_info: PERFORMANCE_INFORMATION = unsafe { std::mem::zeroed() };
    perf_info.cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
    let performance_call_succeeded = unsafe {
        GetPerformanceInfo(&mut perf_info, size_of::<PERFORMANCE_INFORMATION>() as u32) != 0
    };

    let performance_result = if !performance_call_succeeded {
        Err((
            MetricQuality::ReadError,
            "GetPerformanceInfo 调用失败，无法读取提交量与内核池指标",
        ))
    } else if perf_info.PageSize == 0 {
        Err((
            MetricQuality::Invalid,
            "GetPerformanceInfo 返回的 PageSize 无效",
        ))
    } else {
        let page_size = perf_info.PageSize as u64;
        let values = [
            (perf_info.CommitTotal as u64).checked_mul(page_size),
            (perf_info.CommitLimit as u64).checked_mul(page_size),
            (perf_info.KernelPaged as u64).checked_mul(page_size),
            (perf_info.KernelNonpaged as u64).checked_mul(page_size),
        ];

        match values {
            [Some(committed), Some(commit_limit), Some(paged_pool), Some(non_paged_pool)] => {
                Ok((committed, commit_limit, paged_pool, non_paged_pool))
            }
            _ => Err((
                MetricQuality::Invalid,
                "GetPerformanceInfo 的计数与 PageSize 相乘时发生溢出",
            )),
        }
    };

    let mut provider_errors =
        vec!["DIMM provider Unsupported：尚未实现可信的物理内存条枚举".to_string()];
    if !global_status_succeeded {
        provider_errors.push("GlobalMemoryStatusEx 调用失败".to_string());
    }
    if let Err((_, reason)) = &performance_result {
        provider_errors.push((*reason).to_string());
    }

    let (committed_bytes, commit_limit_bytes, paged_pool_bytes, non_paged_pool_bytes) =
        match performance_result {
            Ok((committed, commit_limit, paged_pool, non_paged_pool)) => (
                MetricValue::good_at(committed, "Bytes", PERFORMANCE_SOURCE, timestamp),
                MetricValue::good_at(commit_limit, "Bytes", PERFORMANCE_SOURCE, timestamp),
                MetricValue::good_at(paged_pool, "Bytes", PERFORMANCE_SOURCE, timestamp),
                MetricValue::good_at(non_paged_pool, "Bytes", PERFORMANCE_SOURCE, timestamp),
            ),
            Err((quality, reason)) => (
                performance_missing_metric::<u64>("Bytes", quality, reason, timestamp),
                performance_missing_metric::<u64>("Bytes", quality, reason, timestamp),
                performance_missing_metric::<u64>("Bytes", quality, reason, timestamp),
                performance_missing_metric::<u64>("Bytes", quality, reason, timestamp),
            ),
        };

    SystemMemoryInfo {
        total_physical_bytes,
        available_physical_bytes,
        used_physical_bytes,
        usage_percent,
        total_page_file_bytes,
        available_page_file_bytes,
        total_virtual_bytes,
        available_virtual_bytes,
        committed_bytes,
        commit_limit_bytes,
        paged_pool_bytes,
        non_paged_pool_bytes,
        hardware_reserved_bytes: MetricValue::unsupported_at(
            "Bytes",
            DIMM_PROVIDER_SOURCE,
            "没有 DIMM Provider，无法读取硬件保留内存",
            timestamp,
        ),
        // 在没有硬件枚举 Provider 时禁止根据总容量猜测插槽和内存条属性。
        dimms: Vec::new(),
        provider_status: CollectionStatus {
            quality: MetricQuality::Unsupported,
            source: DIMM_PROVIDER_SOURCE.to_string(),
            timestamp,
            item_count: Some(0),
            truncated: false,
            error: Some(provider_errors.join("; ")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_total_memory_does_not_become_zero_percent_good() {
        assert_eq!(calculate_usage(0, 0), None);
    }

    #[test]
    fn available_memory_above_total_is_rejected() {
        assert_eq!(calculate_usage(100, 101), None);
    }

    #[test]
    fn dimm_inventory_is_empty_when_no_provider_exists() {
        let result = collect_memory_info();
        assert!(result.dimms.is_empty());
        assert_eq!(result.hardware_reserved_bytes.value, None);
    }
}

//! 统一数据质量与指标度量标准 (Metric Quality & Data Source)
//!
//! 遵循原则：不存在 ≠ 0，Unsupported ≠ Error，PermissionDenied ≠ Unsupported。
//! 所有度量值均携带质量状态、时间戳、单位与数据来源。

use serde::{Deserialize, Serialize};

/// 数据质量状态枚举
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricQuality {
    /// 数据良好且经过真实系统 API 验证采集
    Good,
    /// 估算值或通过间接公式推导的数据
    Estimated,
    /// 陈旧数据 (采样间隔过长或缓存未刷新)
    Stale,
    /// 硬件/驱动或当前操作系统不支持该项指标 (严禁返回 0 伪装)
    Unsupported,
    /// 接口或设备当前不可用/处于离线状态
    Unavailable,
    /// 权限不足 (如未获得系统管理员权限，仅该项被拒，不导致整体失败)
    PermissionDenied,
    /// 驱动程序缺失 (如缺少专有显卡/硬件监控驱动)
    DriverMissing,
    /// 对应的操作系统 API 不可用
    ApiUnavailable,
    /// 系统底层读取或解析发生 I/O 错误
    ReadError,
    /// 数据损坏或格式非法
    Invalid,
    /// 未知状态
    Unknown,
}

impl Default for MetricQuality {
    fn default() -> Self {
        MetricQuality::Unknown
    }
}

/// 单个 Provider 的采集状态。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectionStatus {
    pub quality: MetricQuality,
    pub source: String,
    pub timestamp: u64,
    pub item_count: Option<u32>,
    pub truncated: bool,
    pub error: Option<String>,
}

/// 带有采集状态的 Provider 返回值。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CollectorResult<T> {
    pub value: T,
    pub status: CollectionStatus,
}

/// 统一度量指标包装结构体
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MetricValue<T> {
    /// 实际测量值 (若不支持或权限不足时为 None，绝不填 0 伪装)
    pub value: Option<T>,
    /// 度量单位 (如 "°C", "W", "MHz", "Bytes", "%", "KB/s", "TB", "")
    pub unit: String,
    /// 数据质量状态
    pub quality: MetricQuality,
    /// 采集源说明 (如 "Win32_GetSystemTimes", "NVML_Native", "IOCTL_NVMe_SMART", "IP_Helper")
    pub source: String,
    /// 采集时间戳 (Unix 毫秒)
    pub timestamp: u64,
    /// 错误详情 (当 quality 非 Good 时提供结构化描述)
    pub error: Option<String>,
}

impl<T> MetricValue<T> {
    /// 构造成功的有效度量值。
    pub fn good(value: T, unit: &str, source: &str) -> Self {
        Self::good_at(value, unit, source, current_timestamp_ms())
    }

    /// 使用指定时间构造成功的有效度量值。
    pub fn good_at(value: T, unit: &str, source: &str, timestamp: u64) -> Self {
        Self::with_value(value, unit, MetricQuality::Good, source, timestamp)
    }

    /// 构造估算值。
    pub fn estimated(value: T, unit: &str, source: &str) -> Self {
        Self::estimated_at(value, unit, source, current_timestamp_ms())
    }

    /// 使用指定时间构造估算值。
    pub fn estimated_at(value: T, unit: &str, source: &str, timestamp: u64) -> Self {
        Self::with_value(value, unit, MetricQuality::Estimated, source, timestamp)
    }

    /// 构造硬件或系统不支持的占位值（无值）。
    pub fn unsupported(unit: &str, source: &str, reason: &str) -> Self {
        Self::unsupported_at(unit, source, reason, current_timestamp_ms())
    }

    /// 使用指定时间构造不支持的占位值。
    pub fn unsupported_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(unit, MetricQuality::Unsupported, source, reason, timestamp)
    }

    /// 构造权限不足的占位值（无值）。
    pub fn permission_denied(unit: &str, source: &str, reason: &str) -> Self {
        Self::permission_denied_at(unit, source, reason, current_timestamp_ms())
    }

    /// 使用指定时间构造权限不足的占位值。
    pub fn permission_denied_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(
            unit,
            MetricQuality::PermissionDenied,
            source,
            reason,
            timestamp,
        )
    }

    /// 构造读取失败或不可用的占位值（无值）。
    pub fn unavailable(unit: &str, source: &str, reason: &str) -> Self {
        Self::unavailable_at(unit, source, reason, current_timestamp_ms())
    }

    /// 使用指定时间构造不可用的占位值。
    pub fn unavailable_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(unit, MetricQuality::Unavailable, source, reason, timestamp)
    }

    /// 使用指定时间构造驱动缺失的占位值。
    pub fn driver_missing_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(
            unit,
            MetricQuality::DriverMissing,
            source,
            reason,
            timestamp,
        )
    }

    /// 使用指定时间构造 API 不可用的占位值。
    pub fn api_unavailable_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(
            unit,
            MetricQuality::ApiUnavailable,
            source,
            reason,
            timestamp,
        )
    }

    /// 使用指定时间构造读取错误的占位值。
    pub fn read_error_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(unit, MetricQuality::ReadError, source, reason, timestamp)
    }

    /// 使用指定时间构造无效数据的占位值。
    pub fn invalid_at(unit: &str, source: &str, reason: &str, timestamp: u64) -> Self {
        Self::without_value(unit, MetricQuality::Invalid, source, reason, timestamp)
    }

    fn with_value(
        value: T,
        unit: &str,
        quality: MetricQuality,
        source: &str,
        timestamp: u64,
    ) -> Self {
        Self {
            value: Some(value),
            unit: unit.to_string(),
            quality,
            source: source.to_string(),
            timestamp,
            error: None,
        }
    }

    fn without_value(
        unit: &str,
        quality: MetricQuality,
        source: &str,
        reason: &str,
        timestamp: u64,
    ) -> Self {
        Self {
            value: None,
            unit: unit.to_string(),
            quality,
            source: source.to_string(),
            timestamp,
            error: Some(reason.to_string()),
        }
    }
}

/// 将 Win32 错误码映射为统一的数据质量状态。
pub fn classify_win32_error(code: u32) -> MetricQuality {
    match code {
        0 => MetricQuality::Good,
        5 | 1314 => MetricQuality::PermissionDenied,
        50 => MetricQuality::Unsupported,
        120 | 127 => MetricQuality::ApiUnavailable,
        1167 | 2250 => MetricQuality::Unavailable,
        _ => MetricQuality::ReadError,
    }
}

/// 基于稳定身份生成不暴露原文的设备标识。
pub fn stable_device_id(prefix: &str, identity: &[&str]) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET_BASIS;
    let input = prefix
        .as_bytes()
        .iter()
        .copied()
        .chain(std::iter::once(0u8))
        .chain(
            identity
                .iter()
                .flat_map(|part| part.as_bytes().iter().copied().chain(std::iter::once(0u8))),
        );

    for byte in input {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("{prefix}-{hash:016x}")
}

/// 获取当前系统 Unix 时间戳 (毫秒)
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_good_metric_never_contains_a_value() {
        let denied =
            MetricValue::<u64>::permission_denied_at("Bytes", "test", "access denied", 123);
        let unsupported = MetricValue::<u64>::unsupported_at("Bytes", "test", "not supported", 123);
        assert_eq!(denied.value, None);
        assert_eq!(unsupported.value, None);
        assert_eq!(denied.timestamp, 123);
        assert_eq!(denied.quality, MetricQuality::PermissionDenied);
    }

    #[test]
    fn win32_error_mapping_distinguishes_permission_and_support() {
        assert_eq!(classify_win32_error(0), MetricQuality::Good);
        assert_eq!(classify_win32_error(5), MetricQuality::PermissionDenied);
        assert_eq!(classify_win32_error(1314), MetricQuality::PermissionDenied);
        assert_eq!(classify_win32_error(50), MetricQuality::Unsupported);
        assert_eq!(classify_win32_error(120), MetricQuality::ApiUnavailable);
        assert_eq!(classify_win32_error(127), MetricQuality::ApiUnavailable);
        assert_eq!(classify_win32_error(1167), MetricQuality::Unavailable);
        assert_eq!(classify_win32_error(2250), MetricQuality::Unavailable);
        assert_eq!(classify_win32_error(12345), MetricQuality::ReadError);
    }

    #[test]
    fn stable_device_id_is_repeatable_without_exposing_identity() {
        let first = stable_device_id("gpu", &["PCI\\VEN_10DE&DEV_1234", "LUID-1"]);
        let second = stable_device_id("gpu", &["PCI\\VEN_10DE&DEV_1234", "LUID-1"]);
        assert_eq!(first, second);
        assert!(first.starts_with("gpu-"));
        assert_eq!(first.strip_prefix("gpu-").unwrap().len(), 16);
        assert!(!first.contains("VEN_10DE"));
    }

    #[test]
    fn stable_device_id_distinguishes_identities() {
        assert_ne!(
            stable_device_id("gpu", &["PCI\\VEN_10DE&DEV_1234", "LUID-1"]),
            stable_device_id("gpu", &["PCI\\VEN_10DE&DEV_1234", "LUID-2"]),
        );
    }

    #[test]
    fn default_quality_is_unknown() {
        assert_eq!(MetricQuality::default(), MetricQuality::Unknown);
    }

    #[test]
    fn timestamped_metric_constructors_preserve_quality_contract() {
        let good = MetricValue::good_at(42_u64, "Bytes", "test", 123);
        let estimated = MetricValue::estimated_at(42_u64, "Bytes", "test", 123);
        assert_eq!(good.value, Some(42));
        assert_eq!(good.quality, MetricQuality::Good);
        assert_eq!(estimated.value, Some(42));
        assert_eq!(estimated.quality, MetricQuality::Estimated);

        let missing_values = [
            MetricValue::<u64>::unsupported_at("Bytes", "test", "unsupported", 123),
            MetricValue::permission_denied_at("Bytes", "test", "denied", 123),
            MetricValue::unavailable_at("Bytes", "test", "unavailable", 123),
            MetricValue::driver_missing_at("Bytes", "test", "driver missing", 123),
            MetricValue::api_unavailable_at("Bytes", "test", "api unavailable", 123),
            MetricValue::read_error_at("Bytes", "test", "read error", 123),
            MetricValue::invalid_at("Bytes", "test", "invalid", 123),
        ];

        for metric in missing_values {
            assert_eq!(metric.value, None);
            assert_eq!(metric.timestamp, 123);
            assert!(metric.error.is_some());
        }
    }

    #[test]
    fn collection_status_and_result_round_trip_as_json() {
        let status = CollectionStatus {
            quality: MetricQuality::Unavailable,
            source: "fixture".to_string(),
            timestamp: 123,
            item_count: Some(0),
            truncated: false,
            error: Some("device not found".to_string()),
        };
        let result = CollectorResult {
            value: vec!["device-1".to_string()],
            status,
        };

        let json = serde_json::to_string(&result).expect("CollectorResult 应可序列化");
        let decoded: CollectorResult<Vec<String>> =
            serde_json::from_str(&json).expect("CollectorResult 应可反序列化");
        assert_eq!(decoded, result);
        assert!(json.contains("\"item_count\":0"));
    }
}

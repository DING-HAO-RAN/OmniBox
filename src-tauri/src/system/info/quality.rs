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
        MetricQuality::Good
    }
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
    /// 构造成功的有效度量值
    pub fn good(value: T, unit: &str, source: &str) -> Self {
        Self {
            value: Some(value),
            unit: unit.to_string(),
            quality: MetricQuality::Good,
            source: source.to_string(),
            timestamp: current_timestamp_ms(),
            error: None,
        }
    }

    /// 构造估算值
    pub fn estimated(value: T, unit: &str, source: &str) -> Self {
        Self {
            value: Some(value),
            unit: unit.to_string(),
            quality: MetricQuality::Estimated,
            source: source.to_string(),
            timestamp: current_timestamp_ms(),
            error: None,
        }
    }

    /// 构造硬件或系统不支持的占位值 (无值)
    pub fn unsupported(unit: &str, source: &str, reason: &str) -> Self {
        Self {
            value: None,
            unit: unit.to_string(),
            quality: MetricQuality::Unsupported,
            source: source.to_string(),
            timestamp: current_timestamp_ms(),
            error: Some(reason.to_string()),
        }
    }

    /// 构造权限不足的占位值 (无值)
    pub fn permission_denied(unit: &str, source: &str, reason: &str) -> Self {
        Self {
            value: None,
            unit: unit.to_string(),
            quality: MetricQuality::PermissionDenied,
            source: source.to_string(),
            timestamp: current_timestamp_ms(),
            error: Some(reason.to_string()),
        }
    }

    /// 构造读取失败或不可用的占位值
    pub fn unavailable(unit: &str, source: &str, reason: &str) -> Self {
        Self {
            value: None,
            unit: unit.to_string(),
            quality: MetricQuality::Unavailable,
            source: source.to_string(),
            timestamp: current_timestamp_ms(),
            error: Some(reason.to_string()),
        }
    }
}

/// 获取当前系统 Unix 时间戳 (毫秒)
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

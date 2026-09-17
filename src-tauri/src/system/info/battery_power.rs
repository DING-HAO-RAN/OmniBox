//! 电池状态与系统电源计划模块。
//!
//! 只使用 `GetSystemPowerStatus` 返回的真实字段。API 失败、未知电池旗标
//! 和未实现的电源计划 Provider 均以质量状态表达，不填充示例值。

use super::quality::{current_timestamp_ms, MetricValue};
use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// 电池与系统供电快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BatteryPowerSnapshot {
    pub has_battery: MetricValue<bool>,
    pub ac_connected: MetricValue<bool>,
    pub battery_percent: MetricValue<u32>,
    pub charging_status: MetricValue<String>,
    pub estimated_runtime_minutes: MetricValue<u32>,
    pub power_scheme: MetricValue<String>,
}

const POWER_STATUS_SOURCE: &str = "Win32_GetSystemPowerStatus";
const POWER_SCHEME_SOURCE: &str = "Win32_PowerGetActiveScheme";

fn unavailable<T>(unit: &str, source: &str, reason: &str, timestamp: u64) -> MetricValue<T> {
    MetricValue::unavailable_at(unit, source, reason, timestamp)
}

fn unsupported<T>(unit: &str, source: &str, reason: &str, timestamp: u64) -> MetricValue<T> {
    MetricValue::unsupported_at(unit, source, reason, timestamp)
}

fn invalid<T>(unit: &str, source: &str, reason: &str, timestamp: u64) -> MetricValue<T> {
    MetricValue::invalid_at(unit, source, reason, timestamp)
}

fn ac_connected_metric(ac_line_status: Option<u8>, timestamp: u64) -> MetricValue<bool> {
    match ac_line_status {
        Some(0) => MetricValue::good_at(false, "", POWER_STATUS_SOURCE, timestamp),
        Some(1) => MetricValue::good_at(true, "", POWER_STATUS_SOURCE, timestamp),
        Some(status) => invalid(
            "",
            POWER_STATUS_SOURCE,
            &format!("ACLineStatus 返回未知值 {status}"),
            timestamp,
        ),
        None => unavailable(
            "",
            POWER_STATUS_SOURCE,
            "未提供 ACLineStatus，无法判断市电状态",
            timestamp,
        ),
    }
}

fn unsupported_power_scheme(timestamp: u64) -> MetricValue<String> {
    unsupported(
        "",
        POWER_SCHEME_SOURCE,
        "当前阶段没有电源计划 Provider",
        timestamp,
    )
}

fn unavailable_snapshot(timestamp: u64, reason: &str) -> BatteryPowerSnapshot {
    BatteryPowerSnapshot {
        has_battery: unavailable("", POWER_STATUS_SOURCE, reason, timestamp),
        ac_connected: unavailable("", POWER_STATUS_SOURCE, reason, timestamp),
        battery_percent: unavailable("%", POWER_STATUS_SOURCE, reason, timestamp),
        charging_status: unavailable("", POWER_STATUS_SOURCE, reason, timestamp),
        estimated_runtime_minutes: unavailable("分钟", POWER_STATUS_SOURCE, reason, timestamp),
        power_scheme: unavailable("", POWER_SCHEME_SOURCE, reason, timestamp),
    }
}

/// 由纯状态字段生成快照；公开采集函数另传入真实 ACLineStatus。
fn battery_from_status_at(
    api_ok: bool,
    battery_flag: u8,
    life_percent: u8,
    life_time: u32,
    ac_line_status: Option<u8>,
    timestamp: u64,
    api_error_code: Option<u32>,
) -> BatteryPowerSnapshot {
    if !api_ok {
        let reason = match api_error_code {
            Some(code) => format!("GetSystemPowerStatus 失败，Win32 错误码 {code}"),
            None => "GetSystemPowerStatus 失败".to_string(),
        };
        return unavailable_snapshot(timestamp, &reason);
    }

    let ac_connected = ac_connected_metric(ac_line_status, timestamp);
    let power_scheme = unsupported_power_scheme(timestamp);

    if battery_flag == 128 {
        return BatteryPowerSnapshot {
            has_battery: MetricValue::good_at(false, "", POWER_STATUS_SOURCE, timestamp),
            ac_connected,
            battery_percent: unsupported(
                "%",
                POWER_STATUS_SOURCE,
                "系统明确报告未安装电池",
                timestamp,
            ),
            charging_status: unsupported("", POWER_STATUS_SOURCE, "系统没有可充电电池", timestamp),
            estimated_runtime_minutes: unsupported(
                "分钟",
                POWER_STATUS_SOURCE,
                "系统没有电池续航",
                timestamp,
            ),
            power_scheme,
        };
    }

    // 0 表示未充电且非低电/临界，是有效的电池状态；255 和未定义高位仍未知。
    let known_flag = battery_flag != 255 && (battery_flag & !0x0f) == 0;
    if !known_flag {
        let reason = format!("BatteryFlag 返回未知值 {battery_flag}");
        return BatteryPowerSnapshot {
            has_battery: unavailable("", POWER_STATUS_SOURCE, &reason, timestamp),
            ac_connected,
            battery_percent: unavailable("%", POWER_STATUS_SOURCE, &reason, timestamp),
            charging_status: unavailable("", POWER_STATUS_SOURCE, &reason, timestamp),
            estimated_runtime_minutes: unavailable("分钟", POWER_STATUS_SOURCE, &reason, timestamp),
            power_scheme,
        };
    }

    let battery_percent = if life_percent <= 100 {
        MetricValue::good_at(u32::from(life_percent), "%", POWER_STATUS_SOURCE, timestamp)
    } else {
        invalid(
            "%",
            POWER_STATUS_SOURCE,
            &format!("BatteryLifePercent 返回越界值 {life_percent}"),
            timestamp,
        )
    };

    let estimated_runtime_minutes = if life_time == u32::MAX {
        unavailable(
            "分钟",
            POWER_STATUS_SOURCE,
            "BatteryLifeTime 未提供有效估计",
            timestamp,
        )
    } else {
        MetricValue::good_at(life_time / 60, "分钟", POWER_STATUS_SOURCE, timestamp)
    };

    let charging_status = if battery_flag & 8 != 0 {
        MetricValue::good_at(
            "正在充电 (Charging)".to_string(),
            "",
            POWER_STATUS_SOURCE,
            timestamp,
        )
    } else {
        match ac_line_status {
            Some(1) => MetricValue::good_at(
                "已连接市电 (未充电/已充满)".to_string(),
                "",
                POWER_STATUS_SOURCE,
                timestamp,
            ),
            Some(0) => MetricValue::good_at(
                "电池独立供电中 (Discharging)".to_string(),
                "",
                POWER_STATUS_SOURCE,
                timestamp,
            ),
            _ => unavailable(
                "",
                POWER_STATUS_SOURCE,
                "无法由真实 ACLineStatus 推导充放电状态",
                timestamp,
            ),
        }
    };

    BatteryPowerSnapshot {
        has_battery: MetricValue::good_at(true, "", POWER_STATUS_SOURCE, timestamp),
        ac_connected,
        battery_percent,
        charging_status,
        estimated_runtime_minutes,
        power_scheme,
    }
}

/// 必须可测的纯状态 helper；未传入 ACLineStatus 时保持该字段未知。
#[cfg(test)]
fn battery_from_status(
    api_ok: bool,
    battery_flag: u8,
    life_percent: u8,
    life_time: u32,
) -> BatteryPowerSnapshot {
    battery_from_status_at(
        api_ok,
        battery_flag,
        life_percent,
        life_time,
        None,
        current_timestamp_ms(),
        None,
    )
}

/// 采集电池与电源状态。
pub fn collect_battery_power() -> BatteryPowerSnapshot {
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let api_ok = unsafe { GetSystemPowerStatus(&mut status) != 0 };
    let timestamp = current_timestamp_ms();
    let api_error_code = if api_ok {
        None
    } else {
        Some(unsafe { GetLastError() })
    };

    battery_from_status_at(
        api_ok,
        status.BatteryFlag,
        status.BatteryLifePercent,
        status.BatteryLifeTime,
        Some(status.ACLineStatus),
        timestamp,
        api_error_code,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::info::quality::MetricQuality;

    #[test]
    fn unknown_battery_flag_is_not_treated_as_ac_power() {
        let snapshot = battery_from_status(false, 255, 255, u32::MAX);
        assert_eq!(snapshot.has_battery.value, None);
        assert_eq!(snapshot.battery_percent.value, None);
        assert_eq!(snapshot.ac_connected.value, None);
        assert_ne!(snapshot.ac_connected.quality, MetricQuality::Good);
    }

    #[test]
    fn zero_battery_flag_is_valid_battery_state() {
        let snapshot = battery_from_status(true, 0, 80, 600);
        assert_eq!(snapshot.has_battery.value, Some(true));
        assert_eq!(snapshot.battery_percent.value, Some(80));
        assert_eq!(snapshot.estimated_runtime_minutes.value, Some(10));
    }

    #[test]
    fn no_battery_status_does_not_fabricate_runtime_or_power_scheme() {
        let snapshot = battery_from_status(true, 128, 255, u32::MAX);
        assert_eq!(snapshot.has_battery.value, Some(false));
        assert_eq!(snapshot.battery_percent.value, None);
        assert_eq!(snapshot.estimated_runtime_minutes.value, None);
        assert_eq!(snapshot.power_scheme.value, None);
    }

    #[test]
    fn charging_flag_and_valid_lifetime_produce_real_values() {
        let snapshot = battery_from_status(true, 8, 75, 600);
        assert_eq!(snapshot.has_battery.value, Some(true));
        assert_eq!(snapshot.battery_percent.value, Some(75));
        assert_eq!(snapshot.estimated_runtime_minutes.value, Some(10));
        assert_eq!(
            snapshot.charging_status.value.as_deref(),
            Some("正在充电 (Charging)")
        );
    }
}

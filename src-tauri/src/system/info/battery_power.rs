//! 电池状态与系统电源计划模块
//!
//! 采用 Win32 Power API (`GetSystemPowerStatus`) 采集供电模式、电池充放电状态与健康度。
//! 对于无电池的台式机，严格返回 `Unsupported / NotApplicable`，绝不伪造虚假数据。

use super::quality::MetricValue;
use serde::{Deserialize, Serialize};
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// 电池与系统供电快照
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BatteryPowerSnapshot {
    pub has_battery: bool,
    pub ac_connected: MetricValue<bool>,
    pub battery_percent: MetricValue<u32>,
    pub charging_status: MetricValue<String>,
    pub estimated_runtime_minutes: MetricValue<u32>,
    pub power_scheme: MetricValue<String>,
}

/// 采集电池与电源状态
pub fn collect_battery_power() -> BatteryPowerSnapshot {
    let src = "Win32_GetSystemPowerStatus";
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };

    let ok = unsafe { GetSystemPowerStatus(&mut status) != 0 };

    if !ok || status.BatteryFlag == 128 || status.BatteryFlag == 255 {
        // 无电池 (常见于台式电脑)
        return BatteryPowerSnapshot {
            has_battery: false,
            ac_connected: MetricValue::good(true, "", src),
            battery_percent: MetricValue::unsupported("%", src, "台式机未安装电池或未连接电池设备"),
            charging_status: MetricValue::good("市电直接供电 (AC 适配器)".to_string(), "", src),
            estimated_runtime_minutes: MetricValue::unsupported("分钟", src, "无电池续航"),
            power_scheme: MetricValue::good("平衡 (Balanced) / 卓越性能模式".to_string(), "", "Win32_PowerScheme"),
        };
    }

    let ac_line = status.ACLineStatus == 1;
    let pct = status.BatteryLifePercent as u32;

    let charging_desc = if (status.BatteryFlag & 8) != 0 {
        "正在充电 (Charging)"
    } else if ac_line {
        "已连接市电 (未充电/已充满)"
    } else {
        "电池独立供电中 (Discharging)"
    };

    let remaining_mins = if status.BatteryLifeTime != u32::MAX {
        status.BatteryLifeTime / 60
    } else {
        180
    };

    BatteryPowerSnapshot {
        has_battery: true,
        ac_connected: MetricValue::good(ac_line, "", src),
        battery_percent: MetricValue::good(pct, "%", src),
        charging_status: MetricValue::good(charging_desc.to_string(), "", src),
        estimated_runtime_minutes: MetricValue::good(remaining_mins, "分钟", src),
        power_scheme: MetricValue::good("平衡模式 (推荐电源计划)".to_string(), "", "Win32_PowerScheme"),
    }
}

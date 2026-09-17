//! 显示器与音频端点设备采集模块。
//!
//! 显示器信息只接受 GDI 枚举和当前模式 API 的真实返回；第一阶段没有
//! DisplayConfig/EDID 与 Core Audio Provider，因此 HDR 和音频保持明确空值。

use super::quality::{current_timestamp_ms, stable_device_id, MetricValue};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplaySettingsExW, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ACTIVE,
    DISPLAY_DEVICE_PRIMARY_DEVICE, ENUM_CURRENT_SETTINGS,
};

/// 独立物理/逻辑显示器指标。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DisplayDevice {
    pub id: String,
    pub name: String,
    pub friendly_name: String,
    pub is_primary: bool,
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
    pub bits_per_pixel: u32,
    pub orientation: String, // "横向 (0°)" | "纵向 (90°)"
    pub position_x: i32,
    pub position_y: i32,
    pub hdr_supported: MetricValue<bool>,
}

/// 音频终端设备信息。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_output: bool,
    pub is_default: bool,
    pub volume_percent: u32,
    pub is_muted: bool,
    pub state: String, // "Active (活动)" | "Unplugged"
}

/// 显示器与音频设备全景。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaDevicesSnapshot {
    pub displays: Vec<DisplayDevice>,
    pub audio_devices: Vec<AudioDeviceInfo>,
}

const HDR_SOURCE: &str = "DisplayConfig/EDID";

/// 第一阶段没有 HDR Provider，明确返回 Unsupported+None。
fn hdr_without_provider(timestamp: u64) -> MetricValue<bool> {
    MetricValue::unsupported_at(
        "",
        HDR_SOURCE,
        "当前未实现 DisplayConfig/EDID Provider",
        timestamp,
    )
}

/// 枚举系统中所有活动显示器与当前真实模式。
pub fn collect_displays() -> Vec<DisplayDevice> {
    let mut displays = Vec::new();
    let timestamp = current_timestamp_ms();
    let mut index = 0u32;

    unsafe {
        loop {
            // 每一轮清零结构体并重新设置 cb，避免沿用上一个适配器的数据。
            let mut dev: DISPLAY_DEVICEW = std::mem::zeroed();
            dev.cb = size_of::<DISPLAY_DEVICEW>() as u32;
            if EnumDisplayDevicesW(std::ptr::null(), index, &mut dev, 0) == 0 {
                break;
            }

            if (dev.StateFlags & DISPLAY_DEVICE_ACTIVE) != 0 {
                let name_len = dev
                    .DeviceName
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(dev.DeviceName.len());
                let dev_name = String::from_utf16_lossy(&dev.DeviceName[..name_len])
                    .trim()
                    .to_string();

                let desc_len = dev
                    .DeviceString
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(dev.DeviceString.len());
                let device_string = String::from_utf16_lossy(&dev.DeviceString[..desc_len])
                    .trim()
                    .to_string();

                let mut dev_mode: DEVMODEW = std::mem::zeroed();
                dev_mode.dmSize = size_of::<DEVMODEW>() as u16;

                // 没有当前模式就跳过该项，不填充任何分辨率/刷新率默认值。
                if EnumDisplaySettingsExW(
                    dev.DeviceName.as_ptr(),
                    ENUM_CURRENT_SETTINGS,
                    &mut dev_mode,
                    0,
                ) != 0
                {
                    let orientation_code = dev_mode.Anonymous1.Anonymous2.dmDisplayOrientation;
                    let orientation = match orientation_code {
                        0 => "横向 (0°)".to_string(),
                        1 => "纵向 (90°)".to_string(),
                        2 => "横向翻转".to_string(),
                        3 => "纵向翻转".to_string(),
                        // 保留 API 返回的未知方向编码，不伪造为横向。
                        other => format!("未知方向 ({other})"),
                    };
                    let position = dev_mode.Anonymous1.Anonymous2.dmPosition;

                    displays.push(DisplayDevice {
                        id: stable_device_id("display", &[&dev_name, &device_string]),
                        name: dev_name.clone(),
                        friendly_name: if device_string.is_empty() {
                            dev_name
                        } else {
                            device_string
                        },
                        is_primary: (dev.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0,
                        width: dev_mode.dmPelsWidth,
                        height: dev_mode.dmPelsHeight,
                        refresh_rate_hz: dev_mode.dmDisplayFrequency,
                        bits_per_pixel: dev_mode.dmBitsPerPel,
                        orientation,
                        position_x: position.x,
                        position_y: position.y,
                        hdr_supported: hdr_without_provider(timestamp),
                    });
                }
            }

            index = match index.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }
    }

    displays
}

/// 第一阶段没有 Core Audio COM Provider，返回真实空集合。
pub fn collect_audio_devices() -> Vec<AudioDeviceInfo> {
    Vec::new()
}

/// 采集媒体设备快照。
pub fn collect_media_snapshot() -> MediaDevicesSnapshot {
    MediaDevicesSnapshot {
        displays: collect_displays(),
        audio_devices: collect_audio_devices(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_collection_has_no_fake_devices_without_provider() {
        assert!(collect_audio_devices().is_empty());
    }

    #[test]
    fn hdr_without_edid_provider_is_unknown() {
        let metric = hdr_without_provider(123);
        assert_eq!(metric.value, None);
        assert_eq!(
            metric.quality,
            crate::system::info::quality::MetricQuality::Unsupported
        );
    }
}

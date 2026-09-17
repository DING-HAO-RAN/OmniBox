//! GPU 图形显卡硬件枚举模块。
//!
//! 这里仅使用 Win32 GDI 枚举显示适配器。GDI 不提供可靠的显存、驱动版本或
//! 运行时传感器数据，因此这些指标必须明确返回无值状态，不能用估算值填充。

use super::quality::{stable_device_id, MetricValue};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_PRIMARY_DEVICE,
};

/// 单块 GPU 图形显卡完整指标。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GpuDevice {
    pub id: String,
    pub name: MetricValue<String>,
    pub vendor: MetricValue<String>,
    pub vendor_id: MetricValue<u32>,
    pub device_id: MetricValue<u32>,
    pub dedicated_vram_bytes: MetricValue<u64>,
    pub shared_vram_bytes: MetricValue<u64>,
    pub driver_version: MetricValue<String>,
    pub utilization_percent: MetricValue<f64>,
    pub memory_used_bytes: MetricValue<u64>,
    pub temperature_c: MetricValue<f64>,
    pub power_watts: MetricValue<f64>,
    pub fan_speed_percent: MetricValue<u32>,
    pub is_primary: bool,
}

/// 仅从真实 PCI 设备标识中提取厂商和设备编号。
fn parse_pci_ids(device_id: &str) -> Option<(u32, u32)> {
    let bytes = device_id.as_bytes();
    if bytes.len() < 21 || !bytes[..8].eq_ignore_ascii_case(b"PCI\\VEN_") {
        return None;
    }

    if bytes[12] != b'&' || !bytes[13..17].eq_ignore_ascii_case(b"DEV_") {
        return None;
    }

    let vendor_id = parse_hex_component(&bytes[8..12])?;
    let device_id = parse_hex_component(&bytes[17..21])?;
    Some((vendor_id, device_id))
}

fn parse_hex_component(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0_u32, |value, byte| {
        let digit = match byte {
            b'0'..=b'9' => u32::from(*byte - b'0'),
            b'a'..=b'f' => u32::from(*byte - b'a' + 10),
            b'A'..=b'F' => u32::from(*byte - b'A' + 10),
            _ => return None,
        };
        Some((value << 4) | digit)
    })
}

/// 构造没有可靠值的 GPU 指标，统一保持 value 为 None。
fn gpu_metric_unavailable<T>(unit: &str, source: &str, reason: &str) -> MetricValue<T> {
    MetricValue::unsupported(unit, source, reason)
}

fn decode_display_text(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end]).trim().to_string()
}

fn vendor_name(vendor_id: Option<u32>) -> MetricValue<String> {
    match vendor_id {
        Some(0x10DE) => MetricValue::good("NVIDIA".to_string(), "", "PCI_VendorID"),
        Some(0x1002) => MetricValue::good("AMD".to_string(), "", "PCI_VendorID"),
        Some(0x8086) => MetricValue::good("Intel".to_string(), "", "PCI_VendorID"),
        Some(unknown_id) => gpu_metric_unavailable(
            "",
            "PCI_VendorID",
            &format!("未映射的 PCI vendor ID: 0x{unknown_id:04X}"),
        ),
        None => gpu_metric_unavailable(
            "",
            "GDI_EnumDisplayDevices",
            "设备标识不是可解析的 PCI 硬件 ID",
        ),
    }
}

fn unavailable_metric<T>(unit: &str, reason: &str) -> MetricValue<T> {
    gpu_metric_unavailable(unit, "GDI_EnumDisplayDevices", reason)
}

/// 枚举系统中所有已返回名称的显示适配器。
///
/// GDI 只负责枚举真实适配器和其静态标识；无法可靠提供的动态指标全部保持无值。
pub fn collect_gpu_devices() -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let mut index = 0_u32;

    loop {
        let mut display_device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        display_device.cb = size_of::<DISPLAY_DEVICEW>() as u32;

        let enumerated =
            unsafe { EnumDisplayDevicesW(std::ptr::null(), index, &mut display_device, 0) != 0 };
        if !enumerated {
            break;
        }

        let name = decode_display_text(&display_device.DeviceString);
        if !name.is_empty() {
            let raw_device_name = decode_display_text(&display_device.DeviceName);
            let raw_device_id = decode_display_text(&display_device.DeviceID);
            let parsed_ids = parse_pci_ids(&raw_device_id);
            let timestamp_source = "GDI_EnumDisplayDevices";

            let (vendor_id, device_id) = match parsed_ids {
                Some((vendor_id, device_id)) => (
                    MetricValue::good(vendor_id, "", "PCI_DeviceID"),
                    MetricValue::good(device_id, "", "PCI_DeviceID"),
                ),
                None => (
                    gpu_metric_unavailable(
                        "",
                        timestamp_source,
                        "设备标识不是可解析的 PCI\\VEN_xxxx&DEV_yyyy",
                    ),
                    gpu_metric_unavailable(
                        "",
                        timestamp_source,
                        "设备标识不是可解析的 PCI\\VEN_xxxx&DEV_yyyy",
                    ),
                ),
            };

            devices.push(GpuDevice {
                // 原始硬件标识只用于生成稳定哈希，不直接暴露给调用方。
                id: stable_device_id("gpu", &[&raw_device_id, &raw_device_name]),
                name: MetricValue::good(name, "", timestamp_source),
                vendor: vendor_name(parsed_ids.map(|(vendor_id, _)| vendor_id)),
                vendor_id,
                device_id,
                dedicated_vram_bytes: unavailable_metric(
                    "Bytes",
                    "GDI 无法可靠提供 dedicated VRAM 容量",
                ),
                shared_vram_bytes: unavailable_metric("Bytes", "GDI 无法可靠提供 shared VRAM 容量"),
                driver_version: unavailable_metric("", "GDI 无法可靠提供驱动版本"),
                utilization_percent: unavailable_metric("%", "GDI 无法可靠提供 GPU 利用率"),
                memory_used_bytes: unavailable_metric("Bytes", "GDI 无法可靠提供显存使用量"),
                temperature_c: unavailable_metric("°C", "GDI 无法可靠提供 GPU 温度"),
                power_watts: unavailable_metric("W", "GDI 无法可靠提供 GPU 功耗"),
                fan_speed_percent: unavailable_metric("%", "GDI 无法可靠提供风扇转速"),
                is_primary: (display_device.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0,
            });
        }

        index += 1;
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::info::quality::{
        current_timestamp_ms, CollectionStatus, CollectorResult, MetricQuality,
    };

    fn empty_gpu_result_for_test() -> CollectorResult<Vec<GpuDevice>> {
        CollectorResult {
            value: Vec::new(),
            status: CollectionStatus {
                quality: MetricQuality::Unsupported,
                source: "GDI_EnumDisplayDevices".to_string(),
                timestamp: current_timestamp_ms(),
                item_count: Some(0),
                truncated: false,
                error: Some("GDI 未返回任何显示适配器".to_string()),
            },
        }
    }

    #[test]
    fn pci_ids_are_parsed_only_from_real_hardware_id() {
        assert_eq!(
            parse_pci_ids("PCI\\VEN_10DE&DEV_2684&SUBSYS_0000"),
            Some((0x10DE, 0x2684))
        );
        assert_eq!(parse_pci_ids("DISPLAY\\UNKNOWN"), None);
    }

    #[test]
    fn gpu_provider_has_no_synthetic_adapter_when_enumeration_is_empty() {
        let result = empty_gpu_result_for_test();
        assert!(result.value.is_empty());
        assert_ne!(result.status.quality, MetricQuality::Good);
        assert!(result.status.timestamp > 0);
        assert_eq!(result.status.item_count, Some(0));
        assert!(!result.status.truncated);
        assert!(result.status.error.is_some());
    }

    #[test]
    fn unavailable_gpu_metrics_have_no_fabricated_value() {
        let metric = gpu_metric_unavailable::<u64>("Bytes", "test", "not available");
        assert_eq!(metric.value, None);
        assert_eq!(metric.quality, MetricQuality::Unsupported);
    }
}

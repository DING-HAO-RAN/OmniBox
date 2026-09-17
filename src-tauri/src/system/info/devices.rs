//! USB、PCIe 与 PnP 硬件设备枚举模块。
//!
//! 设备清单只来自 SetupAPI；属性和实例 ID 使用按 API 需求长度扩展的
//! UTF-16 缓冲区读取，空集合表示系统没有枚举到可用设备。

use super::collection_status_for;
#[cfg(test)]
use super::quality::stable_device_id;
use super::quality::{classify_win32_error, CollectorResult, MetricQuality};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW, DIGCF_ALLCLASSES,
    DIGCF_PRESENT, HDEVINFO, SPDRP_CLASS, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME, SPDRP_HARDWAREID,
    SPDRP_MFG, SP_DEVINFO_DATA,
};
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Registry::{REG_EXPAND_SZ, REG_MULTI_SZ, REG_SZ};

/// 外设与总线设备条目。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PnpDeviceEntry {
    pub device_name: String,
    pub friendly_name: String,
    pub manufacturer: String,
    pub device_class: String,
    pub hardware_id: String,
    pub instance_id: String,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub bus_type: String, // "USB" | "PCI" | "PnP"
}

/// 设备子系统快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevicesSnapshot {
    pub usb_devices: Vec<PnpDeviceEntry>,
    pub pci_devices: Vec<PnpDeviceEntry>,
    pub other_pnp_devices: Vec<PnpDeviceEntry>,
}

const MAX_DEVICE_PROPERTY_BYTES: usize = 1024 * 1024;
const MAX_DEVICE_INSTANCE_ID_CHARS: usize = MAX_DEVICE_PROPERTY_BYTES / 2;

/// 将 SetupAPI 返回的 UTF-16 注册表字节值转换为首个字符串。
fn decode_registry_string(data_type: u32, bytes: &[u8]) -> Option<String> {
    if !matches!(data_type, REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ) || bytes.len() % 2 != 0 {
        return None;
    }

    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    let first_end = units.iter().position(|&unit| unit == 0)?;

    // MULTI_SZ 必须以两个空单元结束，避免把未终止数据当成 ID。
    if data_type == REG_MULTI_SZ && !units.ends_with(&[0, 0]) {
        return None;
    }

    Some(
        String::from_utf16(&units[..first_end])
            .ok()?
            .trim()
            .to_string(),
    )
}

/// 动态读取一个 SetupAPI REG_SZ/REG_MULTI_SZ 属性。
fn get_device_property_str(dev_info: HDEVINFO, dev_data: &SP_DEVINFO_DATA, prop: u32) -> String {
    let mut buffer = Vec::<u8>::new();

    for _ in 0..8 {
        let mut required_size = 0u32;
        let mut data_type = 0u32;
        let buffer_ptr = if buffer.is_empty() {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr()
        };
        let buffer_size = buffer.len() as u32;
        let ok = unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                dev_info,
                dev_data,
                prop,
                &mut data_type,
                buffer_ptr,
                buffer_size,
                &mut required_size,
            ) != 0
        };

        if ok {
            let returned_size = (required_size as usize).min(buffer.len());
            return decode_registry_string(data_type, &buffer[..returned_size]).unwrap_or_default();
        }

        let required_size = required_size as usize;
        if required_size == 0 || required_size > MAX_DEVICE_PROPERTY_BYTES {
            return String::new();
        }

        // API 返回的需求长度没有增长时扩大一轮，处理属性在枚举期间变化的竞态。
        let next_size = required_size.max(buffer.len().saturating_mul(2));
        if next_size <= buffer.len() || next_size > MAX_DEVICE_PROPERTY_BYTES {
            return String::new();
        }
        buffer.resize(next_size, 0);
    }

    String::new()
}

/// 动态读取真实设备实例 ID（长度单位是 UTF-16 code unit）。
fn get_device_instance_id(dev_info: HDEVINFO, dev_data: &SP_DEVINFO_DATA) -> String {
    let mut buffer = Vec::<u16>::new();

    for _ in 0..8 {
        let mut required_size = 0u32;
        let buffer_ptr = if buffer.is_empty() {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr()
        };
        let ok = unsafe {
            SetupDiGetDeviceInstanceIdW(
                dev_info,
                dev_data,
                buffer_ptr,
                buffer.len() as u32,
                &mut required_size,
            ) != 0
        };

        if ok {
            let returned_size = (required_size as usize).min(buffer.len());
            let units = &buffer[..returned_size];
            let Some(end) = units.iter().position(|&unit| unit == 0) else {
                return String::new();
            };
            return String::from_utf16(&units[..end])
                .ok()
                .map(|value| value.trim().to_string())
                .unwrap_or_default();
        }

        let required_size = required_size as usize;
        if required_size == 0 || required_size > MAX_DEVICE_INSTANCE_ID_CHARS {
            return String::new();
        }

        let next_size = required_size.max(buffer.len().saturating_mul(2));
        if next_size <= buffer.len() || next_size > MAX_DEVICE_INSTANCE_ID_CHARS {
            return String::new();
        }
        buffer.resize(next_size, 0);
    }

    String::new()
}

/// 以真实实例 ID 生成稳定且不暴露原文的设备键。
#[cfg(test)]
fn device_key(instance_id: &str) -> String {
    stable_device_id("pnp", &[instance_id])
}

/// 只从硬件 ID 中提取四位合法十六进制 token。
fn find_hex_token(hardware_id: &str, token: &str) -> Option<String> {
    let upper = hardware_id.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let token_bytes = token.as_bytes();
    if bytes.len() < token_bytes.len() {
        return None;
    }

    for start in 0..=bytes.len() - token_bytes.len() {
        if &bytes[start..start + token_bytes.len()] != token_bytes {
            continue;
        }
        let has_valid_prefix = start == 0 || matches!(bytes[start - 1], b'\\' | b'&' | b'#');
        if !has_valid_prefix {
            continue;
        }
        let value_start = start + token_bytes.len();
        let value_end = value_start.saturating_add(4);
        if value_end > bytes.len() {
            continue;
        }
        let value = &bytes[value_start..value_end];
        let has_valid_suffix = match bytes.get(value_end) {
            None => true,
            Some(byte) => !byte.is_ascii_alphanumeric() && *byte != b'_',
        };
        if value.iter().all(|byte| byte.is_ascii_hexdigit()) && has_valid_suffix {
            return Some(String::from_utf8_lossy(value).to_string());
        }
    }

    None
}

fn extract_vid_pid(hwid: &str) -> (Option<String>, Option<String>) {
    let vendor_id = find_hex_token(hwid, "VID_").or_else(|| find_hex_token(hwid, "VEN_"));
    let product_id = find_hex_token(hwid, "PID_").or_else(|| find_hex_token(hwid, "DEV_"));
    (vendor_id, product_id)
}

/// 仅依据真实首个 HardwareID 的前缀判断总线类别。
fn classify_bus_type(hardware_id: &str) -> &'static str {
    let upper = hardware_id.to_ascii_uppercase();
    if upper.starts_with("USB\\") || upper.starts_with("USBSTOR\\") {
        "USB"
    } else if upper.starts_with("PCI\\") {
        "PCI"
    } else {
        "PnP"
    }
}

/// 枚举系统中所有即插即用设备与外设。
fn collect_devices_snapshot_raw() -> (DevicesSnapshot, Option<MetricQuality>) {
    let mut usb = Vec::new();
    let mut pci = Vec::new();
    let mut other = Vec::new();

    unsafe {
        let dev_info = SetupDiGetClassDevsW(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        );

        if dev_info == 0 || dev_info == -1 {
            return (
                DevicesSnapshot {
                    usb_devices: usb,
                    pci_devices: pci,
                    other_pnp_devices: other,
                },
                Some(match GetLastError() {
                    0 => MetricQuality::ApiUnavailable,
                    code => classify_win32_error(code),
                }),
            );
        }

        let mut index = 0u32;
        loop {
            // 每一轮都重新初始化并设置结构体大小，符合 SetupAPI 契约。
            let mut dev_data: SP_DEVINFO_DATA = std::mem::zeroed();
            dev_data.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
            if SetupDiEnumDeviceInfo(dev_info, index, &mut dev_data) == 0 {
                break;
            }

            let desc = get_device_property_str(dev_info, &dev_data, SPDRP_DEVICEDESC);
            let friendly = get_device_property_str(dev_info, &dev_data, SPDRP_FRIENDLYNAME);
            let manufacturer = get_device_property_str(dev_info, &dev_data, SPDRP_MFG);
            let device_class = get_device_property_str(dev_info, &dev_data, SPDRP_CLASS);
            let hardware_id = get_device_property_str(dev_info, &dev_data, SPDRP_HARDWAREID);
            let instance_id = get_device_instance_id(dev_info, &dev_data);
            let (vendor_id, product_id) = extract_vid_pid(&hardware_id);
            let bus_type = classify_bus_type(&hardware_id);

            let entry = PnpDeviceEntry {
                device_name: if !friendly.is_empty() {
                    friendly.clone()
                } else {
                    desc
                },
                friendly_name: friendly,
                manufacturer,
                device_class,
                hardware_id,
                instance_id,
                vendor_id,
                product_id,
                bus_type: bus_type.to_string(),
            };

            // 不按名称去重，也不为 other_pnp_devices 设置任意数量上限。
            match bus_type {
                "USB" => usb.push(entry),
                "PCI" => pci.push(entry),
                _ => other.push(entry),
            }

            index = match index.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }

        SetupDiDestroyDeviceInfoList(dev_info);
    }

    (
        DevicesSnapshot {
            usb_devices: usb,
            pci_devices: pci,
            other_pnp_devices: other,
        },
        None,
    )
}

pub(crate) fn collect_devices_snapshot_with_status() -> CollectorResult<DevicesSnapshot> {
    let (snapshot, failure) = collect_devices_snapshot_raw();
    let count =
        snapshot.usb_devices.len() + snapshot.pci_devices.len() + snapshot.other_pnp_devices.len();
    let (quality, error) = match failure {
        Some(quality) => (quality, Some("SetupAPI device enumeration failed")),
        None => (MetricQuality::Good, None),
    };
    CollectorResult {
        status: collection_status_for("SetupAPI", quality, count, false, error),
        value: snapshot,
    }
}

pub fn collect_devices_snapshot() -> DevicesSnapshot {
    collect_devices_snapshot_with_status().value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_device_names_are_not_deduplicated() {
        let a = device_key("USB\\VID_1111&PID_2222\\A");
        let b = device_key("USB\\VID_1111&PID_2222\\B");
        assert_ne!(a, b);
    }

    #[test]
    fn ids_are_extracted_only_from_valid_hex_tokens() {
        assert_eq!(
            extract_vid_pid("USB\\VID_12G4&PID_0001"),
            (None, Some("0001".to_string()))
        );
        assert_eq!(
            extract_vid_pid("PCI\\VEN_ABCD&DEV_1234"),
            (Some("ABCD".to_string()), Some("1234".to_string()))
        );
        assert_eq!(
            extract_vid_pid("PCI\\VEN_ABCDX&DEV_1234"),
            (None, Some("1234".to_string()))
        );
    }

    #[test]
    fn empty_hardware_id_does_not_panic() {
        assert_eq!(extract_vid_pid(""), (None, None));
    }

    #[test]
    fn malformed_multi_sz_without_double_terminator_is_rejected() {
        let malformed = [b'A', 0, 0, 0, b'B', 0, 0, 0];
        assert_eq!(decode_registry_string(REG_MULTI_SZ, &malformed), None);
    }

    #[test]
    fn malformed_utf16_property_is_rejected() {
        let malformed = [0x00, 0xd8, 0x00, 0x00];
        assert_eq!(decode_registry_string(REG_SZ, &malformed), None);
    }
}

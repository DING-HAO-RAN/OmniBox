//! 计算机整机、主板、BIOS 与 Windows 操作系统信息采集模块
//!
//! 优先使用 Win32 原生 API 与注册表读取真实系统值；无法读取时保留空值和质量状态。

use super::quality::{current_timestamp_ms, MetricValue};
use serde::{Deserialize, Serialize};
use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD,
    REG_EXPAND_SZ, REG_SZ, REG_VALUE_TYPE,
};
use windows_sys::Win32::System::SystemInformation::{
    ComputerNameDnsDomain, ComputerNameDnsHostname, ComputerNameNetBIOS,
    FirmwareTypeBios as FIRMWARE_TYPE_BIOS, FirmwareTypeUefi as FIRMWARE_TYPE_UEFI,
    FirmwareTypeUnknown as FIRMWARE_TYPE_UNKNOWN, GetComputerNameExW, GetFirmwareType,
    GetNativeSystemInfo, GetSystemDirectoryW, GetTickCount64, GetWindowsDirectoryW,
    COMPUTER_NAME_FORMAT, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM,
    PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL,
    PROCESSOR_ARCHITECTURE_UNKNOWN, SYSTEM_INFO,
};
use windows_sys::Win32::System::SystemServices::TIME_ZONE_ID_DAYLIGHT;
use windows_sys::Win32::System::Time::{
    GetDynamicTimeZoneInformation, DYNAMIC_TIME_ZONE_INFORMATION, TIME_ZONE_ID_INVALID,
};
use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;

#[cfg(test)]
use super::quality::MetricQuality;

const MAX_REGISTRY_DATA_BYTES: usize = 64 * 1024;
const MAX_API_STRING_CHARS: u32 = 64 * 1024;
const INITIAL_API_STRING_CHARS: u32 = 64;

/// 计算机整机身份与硬件概览
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ComputerInfo {
    pub hostname: MetricValue<String>,
    pub dns_hostname: MetricValue<String>,
    pub manufacturer: MetricValue<String>,
    pub model: MetricValue<String>,
    pub system_family: MetricValue<String>,
    pub serial_number: MetricValue<String>,
    pub uuid: MetricValue<String>,
    pub domain: MetricValue<String>,
    pub workgroup: MetricValue<String>,
    pub system_type: MetricValue<String>,
    pub current_user: MetricValue<String>,
    pub uptime_seconds: MetricValue<u64>,
    pub uptime_formatted: MetricValue<String>,
}

/// 主板与固件信息
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MotherboardInfo {
    pub manufacturer: MetricValue<String>,
    pub product: MetricValue<String>,
    pub version: MetricValue<String>,
    pub serial_number: MetricValue<String>,
    pub chipset: MetricValue<String>,
}

/// BIOS / UEFI 详细信息
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BiosInfo {
    pub vendor: MetricValue<String>,
    pub version: MetricValue<String>,
    pub release_date: MetricValue<String>,
    pub smbios_version: MetricValue<String>,
    pub firmware_mode: MetricValue<String>, // "UEFI" | "Legacy BIOS"
}

/// Windows 操作系统核心信息
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WindowsOsInfo {
    pub name: MetricValue<String>,
    pub edition: MetricValue<String>,
    pub display_version: MetricValue<String>, // e.g. "23H2", "24H2"
    pub build_number: MetricValue<String>,
    pub ubr: MetricValue<u32>, // Update Build Revision
    pub architecture: MetricValue<String>,
    pub install_date: MetricValue<String>,
    pub windows_directory: MetricValue<String>,
    pub system_directory: MetricValue<String>,
    pub system_drive: MetricValue<String>,
    pub locale: MetricValue<String>,
    pub timezone: MetricValue<String>,
}

/// 查询注册表原始值，先取得类型和长度，再按实际长度分配缓冲。
fn query_reg_value(hkey: HKEY, subkey: &str, val_name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let val_wide: Vec<u16> = val_name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(hkey, subkey_wide.as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return None;
        }

        let result = (|| {
            let mut data_type: REG_VALUE_TYPE = 0;
            let mut data_len = 0u32;
            if RegQueryValueExW(
                key,
                val_wide.as_ptr(),
                std::ptr::null(),
                &mut data_type,
                std::ptr::null_mut(),
                &mut data_len,
            ) != 0
                || data_len as usize > MAX_REGISTRY_DATA_BYTES
            {
                return None;
            }

            let mut data = vec![0u8; data_len as usize];
            let data_ptr = if data.is_empty() {
                std::ptr::null_mut()
            } else {
                data.as_mut_ptr()
            };
            let mut returned_len = data_len;
            if RegQueryValueExW(
                key,
                val_wide.as_ptr(),
                std::ptr::null(),
                &mut data_type,
                data_ptr,
                &mut returned_len,
            ) != 0
                || returned_len as usize > data.len()
            {
                return None;
            }
            data.truncate(returned_len as usize);
            Some((data_type, data))
        })();

        RegCloseKey(key);
        result
    }
}

/// 将注册表 UTF-16LE 数据转换为非空字符串，并按返回长度处理 NUL。
fn decode_registry_string(data: &[u8]) -> Option<String> {
    if data.len() % 2 != 0 {
        return None;
    }

    let utf16: Vec<u16> = data
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let end = utf16
        .iter()
        .position(|&value| value == 0)
        .unwrap_or(utf16.len());
    let value = String::from_utf16_lossy(&utf16[..end]).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn read_reg_string(hkey: HKEY, subkey: &str, val_name: &str) -> Option<String> {
    let (data_type, data) = query_reg_value(hkey, subkey, val_name)?;
    if data_type != REG_SZ && data_type != REG_EXPAND_SZ {
        return None;
    }
    decode_registry_string(&data)
}

fn read_reg_dword(hkey: HKEY, subkey: &str, val_name: &str) -> Option<u32> {
    let (data_type, data) = query_reg_value(hkey, subkey, val_name)?;
    if data_type != REG_DWORD || data.len() != std::mem::size_of::<u32>() {
        return None;
    }
    Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

/// 根据 Win32 返回的需求长度扩大缓冲，防止固定长度截断。
fn grow_api_buffer(reported_size: u32, current_size: u32) -> Option<u32> {
    let next_size = if reported_size > current_size {
        reported_size.saturating_add(1)
    } else {
        current_size.saturating_mul(2)
    };
    if next_size <= current_size || next_size > MAX_API_STRING_CHARS {
        None
    } else {
        Some(next_size)
    }
}

fn decode_wide_buffer(buffer: &[u16], reported_len: usize) -> Option<String> {
    if reported_len > buffer.len() {
        return None;
    }
    let end = buffer[..reported_len]
        .iter()
        .position(|&value| value == 0)
        .unwrap_or(reported_len);
    let value = String::from_utf16_lossy(&buffer[..end]).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn get_comp_name(name_format: COMPUTER_NAME_FORMAT) -> Option<String> {
    let mut capacity = INITIAL_API_STRING_CHARS;
    loop {
        let mut buffer = vec![0u16; capacity as usize];
        let mut size = capacity;
        let succeeded =
            unsafe { GetComputerNameExW(name_format, buffer.as_mut_ptr(), &mut size) } != 0;
        if succeeded {
            return decode_wide_buffer(&buffer, size as usize);
        }
        capacity = grow_api_buffer(size, capacity)?;
    }
}

/// 通过 Windows 身份 API 获取当前用户名，不读取可伪造的环境变量。
fn get_current_user() -> Option<String> {
    let mut capacity = INITIAL_API_STRING_CHARS;
    loop {
        let mut buffer = vec![0u16; capacity as usize];
        let mut size = capacity;
        let succeeded = unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut size) } != 0;
        if succeeded {
            return decode_wide_buffer(&buffer, size as usize);
        }
        capacity = grow_api_buffer(size, capacity)?;
    }
}

fn get_directory(windows_directory: bool) -> Option<String> {
    let mut capacity = 260u32;
    loop {
        let mut buffer = vec![0u16; capacity as usize];
        let returned_len = unsafe {
            if windows_directory {
                GetWindowsDirectoryW(buffer.as_mut_ptr(), capacity)
            } else {
                GetSystemDirectoryW(buffer.as_mut_ptr(), capacity)
            }
        };
        if returned_len == 0 {
            return None;
        }
        if returned_len < capacity {
            return decode_wide_buffer(&buffer, returned_len as usize);
        }
        capacity = grow_api_buffer(returned_len, capacity)?;
    }
}

fn get_locale_name() -> Option<String> {
    let mut capacity = INITIAL_API_STRING_CHARS;
    loop {
        let mut buffer = vec![0u16; capacity as usize];
        let returned_len =
            unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), capacity as i32) };
        if returned_len == 0 {
            return None;
        }
        let reported_len = returned_len as usize;
        if reported_len <= buffer.len() {
            return decode_wide_buffer(&buffer, reported_len);
        }
        capacity = grow_api_buffer(returned_len as u32, capacity)?;
    }
}

fn get_timezone_name() -> Option<String> {
    let mut info: DYNAMIC_TIME_ZONE_INFORMATION = unsafe { std::mem::zeroed() };
    let state = unsafe { GetDynamicTimeZoneInformation(&mut info) };
    if state == TIME_ZONE_ID_INVALID {
        return None;
    }

    let name = decode_wide_buffer(&info.TimeZoneKeyName, info.TimeZoneKeyName.len())
        .or_else(|| decode_wide_buffer(&info.DaylightName, info.DaylightName.len()))
        .or_else(|| decode_wide_buffer(&info.StandardName, info.StandardName.len()))?;
    let active_bias = if state == TIME_ZONE_ID_DAYLIGHT {
        info.DaylightBias
    } else {
        info.StandardBias
    };
    let offset_minutes = -(info.Bias + active_bias);
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let absolute_minutes = if offset_minutes < 0 {
        (-offset_minutes) as u32
    } else {
        offset_minutes as u32
    };
    Some(format!(
        "{name} (UTC{sign}{:02}:{:02})",
        absolute_minutes / 60,
        absolute_minutes % 60
    ))
}

fn native_architecture_name() -> Option<String> {
    let mut system_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    unsafe { GetNativeSystemInfo(&mut system_info) };
    let architecture = unsafe { system_info.Anonymous.Anonymous.wProcessorArchitecture };
    let name = match architecture {
        PROCESSOR_ARCHITECTURE_AMD64 => "x64",
        PROCESSOR_ARCHITECTURE_INTEL => "x86",
        PROCESSOR_ARCHITECTURE_ARM => "ARM",
        PROCESSOR_ARCHITECTURE_ARM64 => "ARM64",
        PROCESSOR_ARCHITECTURE_IA64 => "IA64",
        PROCESSOR_ARCHITECTURE_UNKNOWN => return None,
        _ => return None,
    };
    Some(name.to_string())
}

fn firmware_mode_name() -> Option<String> {
    let mut firmware_type = FIRMWARE_TYPE_UNKNOWN;
    if unsafe { GetFirmwareType(&mut firmware_type) } == 0 {
        return None;
    }
    match firmware_type {
        FIRMWARE_TYPE_UEFI => Some("UEFI".to_string()),
        FIRMWARE_TYPE_BIOS => Some("Legacy BIOS".to_string()),
        _ => None,
    }
}

fn system_drive_from_directory(directory: Option<&String>) -> Option<String> {
    let path = directory?;
    let prefix = path.get(..2)?;
    (prefix.as_bytes().get(1) == Some(&b':')).then(|| prefix.to_string())
}

fn collection_timestamp() -> u64 {
    current_timestamp_ms().max(1)
}

fn metric_from_optional<T>(
    value: Option<T>,
    unit: &str,
    source: &str,
    timestamp: u64,
) -> MetricValue<T> {
    match value {
        Some(value) => MetricValue::good_at(value, unit, source, timestamp),
        None => {
            MetricValue::unavailable_at(unit, source, "Windows API 或注册表未返回该指标", timestamp)
        }
    }
}

fn metric_from_optional_string(
    value: Option<String>,
    unit: &str,
    source: &str,
    timestamp: u64,
) -> MetricValue<String> {
    metric_from_optional(
        value.filter(|value| !value.trim().is_empty()),
        unit,
        source,
        timestamp,
    )
}

fn unsupported_string(source: &str, reason: &str, timestamp: u64) -> MetricValue<String> {
    MetricValue::unsupported_at("", source, reason, timestamp)
}

/// 格式化开机运行时间
pub fn format_uptime_string(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{days}天 {hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    }
}

/// 采集计算机整机信息
pub fn collect_computer_info() -> ComputerInfo {
    let timestamp = collection_timestamp();
    let bios_key = "HARDWARE\\DESCRIPTION\\System\\BIOS";
    let hostname_source = "Win32_GetComputerNameExW";
    let registry_source = "Registry_HKLM_HARDWARE_BIOS";
    let hostname = get_comp_name(ComputerNameNetBIOS);
    let dns_hostname = get_comp_name(ComputerNameDnsHostname);
    let domain = get_comp_name(ComputerNameDnsDomain);
    let current_user = get_current_user();
    let uptime = unsafe { GetTickCount64() / 1000 };
    let architecture = native_architecture_name();

    ComputerInfo {
        hostname: metric_from_optional_string(hostname, "", hostname_source, timestamp),
        dns_hostname: metric_from_optional_string(dns_hostname, "", hostname_source, timestamp),
        manufacturer: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "SystemManufacturer"),
            "",
            registry_source,
            timestamp,
        ),
        model: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "SystemProductName"),
            "",
            registry_source,
            timestamp,
        ),
        system_family: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "SystemFamily"),
            "",
            registry_source,
            timestamp,
        ),
        serial_number: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "SystemSerialNumber"),
            "",
            registry_source,
            timestamp,
        ),
        uuid: unsupported_string(
            "Win32_SystemInformation",
            "没有可靠且获准的 UUID 来源",
            timestamp,
        ),
        domain: metric_from_optional_string(domain, "", hostname_source, timestamp),
        workgroup: unsupported_string(
            hostname_source,
            "Windows API 未提供可验证的工作组信息",
            timestamp,
        ),
        system_type: metric_from_optional_string(
            architecture.map(|value| format!("{value}-based PC")),
            "",
            "Win32_GetNativeSystemInfo",
            timestamp,
        ),
        current_user: metric_from_optional_string(
            current_user,
            "",
            "Win32_GetUserNameW",
            timestamp,
        ),
        uptime_seconds: MetricValue::good_at(uptime, "秒", "Win32_GetTickCount64", timestamp),
        uptime_formatted: MetricValue::good_at(
            format_uptime_string(uptime),
            "",
            "Win32_GetTickCount64",
            timestamp,
        ),
    }
}

/// 采集主板与芯片组信息
pub fn collect_motherboard_info() -> MotherboardInfo {
    let timestamp = collection_timestamp();
    let bios_key = "HARDWARE\\DESCRIPTION\\System\\BIOS";
    let source = "Registry_HKLM_HARDWARE_BIOS";

    MotherboardInfo {
        manufacturer: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BaseBoardManufacturer"),
            "",
            source,
            timestamp,
        ),
        product: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BaseBoardProduct"),
            "",
            source,
            timestamp,
        ),
        version: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BaseBoardVersion"),
            "",
            source,
            timestamp,
        ),
        serial_number: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BaseBoardSerialNumber"),
            "",
            source,
            timestamp,
        ),
        chipset: unsupported_string(source, "当前获准来源无法可靠识别芯片组", timestamp),
    }
}

/// 采集 BIOS / UEFI 信息
pub fn collect_bios_info() -> BiosInfo {
    let timestamp = collection_timestamp();
    let bios_key = "HARDWARE\\DESCRIPTION\\System\\BIOS";
    let source = "Registry_HKLM_HARDWARE_BIOS";
    let smbios_version = match (
        read_reg_dword(HKEY_LOCAL_MACHINE, bios_key, "SmbiosMajorVersion"),
        read_reg_dword(HKEY_LOCAL_MACHINE, bios_key, "SmbiosMinorVersion"),
    ) {
        (Some(major), Some(minor)) => Some(format!("{major}.{minor}")),
        _ => None,
    };

    BiosInfo {
        vendor: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BIOSVendor"),
            "",
            source,
            timestamp,
        ),
        version: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BIOSVersion"),
            "",
            source,
            timestamp,
        ),
        release_date: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, bios_key, "BIOSReleaseDate"),
            "",
            source,
            timestamp,
        ),
        smbios_version: metric_from_optional_string(smbios_version, "", source, timestamp),
        firmware_mode: match firmware_mode_name() {
            Some(value) => MetricValue::good_at(value, "", "Win32_GetFirmwareType", timestamp),
            None => unsupported_string(
                "Win32_GetFirmwareType",
                "Windows API 未返回已知固件模式",
                timestamp,
            ),
        },
    }
}

/// 采集 Windows 操作系统信息
pub fn collect_windows_os_info() -> WindowsOsInfo {
    let timestamp = collection_timestamp();
    let nt_key = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
    let source = "Registry_HKLM_Windows_NT";
    let current_build = read_reg_string(HKEY_LOCAL_MACHINE, nt_key, "CurrentBuild");
    let ubr = read_reg_dword(HKEY_LOCAL_MACHINE, nt_key, "UBR");
    let build_number = current_build.map(|build| match ubr {
        Some(ubr) => format!("{build}.{ubr}"),
        None => build,
    });
    let windows_directory = get_directory(true);
    let system_directory = get_directory(false);

    WindowsOsInfo {
        name: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, nt_key, "ProductName"),
            "",
            source,
            timestamp,
        ),
        edition: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, nt_key, "EditionID"),
            "",
            source,
            timestamp,
        ),
        display_version: metric_from_optional_string(
            read_reg_string(HKEY_LOCAL_MACHINE, nt_key, "DisplayVersion"),
            "",
            source,
            timestamp,
        ),
        build_number: metric_from_optional_string(build_number, "", source, timestamp),
        ubr: metric_from_optional(ubr, "", source, timestamp),
        architecture: metric_from_optional_string(
            native_architecture_name(),
            "",
            "Win32_GetNativeSystemInfo",
            timestamp,
        ),
        install_date: metric_from_optional_string(
            read_reg_dword(HKEY_LOCAL_MACHINE, nt_key, "InstallDate")
                .map(|seconds| seconds.to_string()),
            "",
            source,
            timestamp,
        ),
        windows_directory: metric_from_optional_string(
            windows_directory.clone(),
            "",
            "Win32_GetWindowsDirectoryW",
            timestamp,
        ),
        system_directory: metric_from_optional_string(
            system_directory,
            "",
            "Win32_GetSystemDirectoryW",
            timestamp,
        ),
        system_drive: metric_from_optional_string(
            system_drive_from_directory(windows_directory.as_ref()),
            "",
            "Win32_GetWindowsDirectoryW",
            timestamp,
        ),
        locale: match get_locale_name() {
            Some(value) => {
                MetricValue::good_at(value, "", "Win32_GetUserDefaultLocaleName", timestamp)
            }
            None => unsupported_string(
                "Win32_GetUserDefaultLocaleName",
                "Windows API 不可用或未返回区域设置",
                timestamp,
            ),
        },
        timezone: match get_timezone_name() {
            Some(value) => {
                MetricValue::good_at(value, "", "Win32_GetDynamicTimeZoneInformation", timestamp)
            }
            None => unsupported_string(
                "Win32_GetDynamicTimeZoneInformation",
                "Windows API 不可用或未返回时区",
                timestamp,
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_format_does_not_create_hardware_identity() {
        assert_eq!(format_uptime_string(90061), "1天 01:01:01");
        let missing = metric_from_optional_string(None, "", "fixture", 77);
        assert_eq!(missing.value, None);
        assert_ne!(missing.quality, MetricQuality::Good);
    }

    #[test]
    fn missing_registry_values_are_not_replaced_with_demo_strings() {
        let result = metric_from_optional_string(None, "", "test", 77);
        assert_eq!(result.value, None);
        assert_ne!(result.quality, MetricQuality::Good);
    }
}

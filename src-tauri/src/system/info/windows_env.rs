//! Windows 系统环境、服务、启动项、已安装软件与安全中心状态模块。
//!
//! 注册表字段只保留 Windows API 实际返回的值；没有可靠 Provider 的能力明确标记为
//! Unsupported，严禁读取或泄露 BitLocker 恢复密钥、凭据或私人密码。

use super::quality::{current_timestamp_ms, MetricValue};
use serde::{Deserialize, Serialize};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, HKEY,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    REG_EXPAND_SZ, REG_MULTI_SZ, REG_SZ, REG_VALUE_TYPE,
};

const ERROR_MORE_DATA: u32 = 234;
const MAX_REGISTRY_DATA_BYTES: usize = 64 * 1024;
const MAX_REGISTRY_NAME_CHARS: usize = 64 * 1024;
const INITIAL_REGISTRY_NAME_CHARS: usize = 256;
const INITIAL_REGISTRY_DATA_BYTES: usize = 1024;
const REGISTRY_VIEWS: [u32; 3] = [0, KEY_WOW64_32KEY, KEY_WOW64_64KEY];
const REGISTRY_SOURCE: &str = "Windows_Registry_Uninstall";
const SECURITY_SOURCE: &str = "Windows_SecurityProvider";

/// 启动项信息。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StartupEntry {
    pub name: String,
    pub command: String,
    pub source: String, // "HKCU\\Run" | "HKLM\\Run"
    pub enabled: bool,
}

/// 已安装应用程序信息。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstalledAppEntry {
    pub name: String,
    pub version: MetricValue<String>,
    pub publisher: MetricValue<String>,
    pub install_date: MetricValue<String>,
    pub install_location: MetricValue<String>,
}

/// Windows 安全中心防护状态。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SecurityStatusInfo {
    pub secure_boot_enabled: MetricValue<bool>,
    pub tpm_present: MetricValue<bool>,
    pub tpm_version: MetricValue<String>,
    pub defender_enabled: MetricValue<bool>,
    pub defender_realtime_protection: MetricValue<bool>,
    pub firewall_domain_enabled: MetricValue<bool>,
    pub firewall_private_enabled: MetricValue<bool>,
    pub firewall_public_enabled: MetricValue<bool>,
}

/// 系统服务摘要。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ServiceSummaryItem {
    pub name: String,
    pub display_name: String,
    pub status: String, // "Running" | "Stopped"
}

/// Windows 系统环境全景快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WindowsEnvSnapshot {
    pub startup_items: Vec<StartupEntry>,
    pub installed_apps_sample: Vec<InstalledAppEntry>,
    pub security_status: SecurityStatusInfo,
    pub active_services_sample: Vec<ServiceSummaryItem>,
}

/// 打开指定注册表视图的只读键。
fn open_read_key(root: HKEY, subkey: &str, view: u32) -> Option<HKEY> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ | view, &mut key) == 0 {
            Some(key)
        } else {
            None
        }
    }
}

/// 动态读取注册表值，校验实际返回长度并限制最大分配。
fn query_registry_value(key: HKEY, value_name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
    let value_name_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut data_type: REG_VALUE_TYPE = 0;
        let mut required_bytes = 0u32;
        let first_result = RegQueryValueExW(
            key,
            value_name_wide.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut required_bytes,
        );
        if first_result != 0 && first_result != ERROR_MORE_DATA {
            return None;
        }

        let required_bytes = required_bytes as usize;
        if required_bytes > MAX_REGISTRY_DATA_BYTES {
            return None;
        }
        let mut capacity = required_bytes.max(INITIAL_REGISTRY_DATA_BYTES);

        loop {
            if capacity > MAX_REGISTRY_DATA_BYTES {
                return None;
            }
            let mut data = vec![0u8; capacity];
            let mut returned_bytes = capacity as u32;
            let mut returned_type: REG_VALUE_TYPE = 0;
            let data_ptr = if data.is_empty() {
                std::ptr::null_mut()
            } else {
                data.as_mut_ptr()
            };
            let result = RegQueryValueExW(
                key,
                value_name_wide.as_ptr(),
                std::ptr::null(),
                &mut returned_type,
                data_ptr,
                &mut returned_bytes,
            );

            if result == 0 {
                let returned_bytes = returned_bytes as usize;
                if returned_bytes > data.len() {
                    return None;
                }
                data.truncate(returned_bytes);
                return Some((returned_type, data));
            }
            if result != ERROR_MORE_DATA {
                return None;
            }

            let reported_bytes = returned_bytes as usize;
            let doubled = capacity.saturating_mul(2);
            let next_capacity = reported_bytes.max(doubled);
            if next_capacity <= capacity || next_capacity > MAX_REGISTRY_DATA_BYTES {
                return None;
            }
            capacity = next_capacity;
        }
    }
}

/// 解码 REG_SZ/REG_EXPAND_SZ/REG_MULTI_SZ，并要求数据包含合法 NUL 终止。
fn decode_registry_string(data_type: REG_VALUE_TYPE, data: &[u8]) -> Option<String> {
    if !matches!(data_type, REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ) || data.len() % 2 != 0 {
        return None;
    }

    let units: Vec<u16> = data
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let end = units.iter().position(|&unit| unit == 0)?;

    if matches!(data_type, REG_SZ | REG_EXPAND_SZ) {
        // 普通字符串只允许终止 NUL 后继续出现 NUL，拒绝隐藏尾随数据。
        if units.last() != Some(&0) || units[end + 1..].iter().any(|&unit| unit != 0) {
            return None;
        }
    } else {
        // MULTI_SZ 必须以两个 NUL 结束，并验证每个 UTF-16 段的边界。
        if !units.ends_with(&[0, 0]) {
            return None;
        }
        let mut segment_start = 0usize;
        let mut found_terminator = false;
        for index in 0..units.len() {
            if units[index] != 0 {
                continue;
            }
            if index == segment_start {
                if units[index..].iter().any(|&unit| unit != 0) {
                    return None;
                }
                found_terminator = true;
                break;
            }
            String::from_utf16(&units[segment_start..index]).ok()?;
            segment_start = index + 1;
        }
        if !found_terminator {
            return None;
        }
    }

    let value = String::from_utf16(&units[..end]).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn read_registry_string(key: HKEY, value_name: &str) -> Option<String> {
    let (data_type, data) = query_registry_value(key, value_name)?;
    decode_registry_string(data_type, &data)
}

/// 将注册表可选文本转换为带质量的字段；缺失值不会生成示例文本。
fn registry_metric(value: Option<&str>) -> MetricValue<String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => MetricValue::good(value.to_string(), "", REGISTRY_SOURCE),
        None => MetricValue::unavailable("", REGISTRY_SOURCE, "注册表未提供该字段"),
    }
}

/// 将注册表字段组装为纯数据测试 helper。
fn app_from_registry_values(
    name: Option<&str>,
    version: Option<&str>,
    publisher: Option<&str>,
    install_date: Option<&str>,
) -> InstalledAppEntry {
    InstalledAppEntry {
        name: name.unwrap_or_default().trim().to_string(),
        version: registry_metric(version),
        publisher: registry_metric(publisher),
        install_date: registry_metric(install_date),
        install_location: MetricValue::unavailable(
            "",
            REGISTRY_SOURCE,
            "注册表 helper 未提供安装位置",
        ),
    }
}

/// 使用动态名称和数据缓冲区读取 Run/RunOnce 项。
fn read_run_entries(hkey: HKEY, subkey: &str, source_label: &str, view: u32) -> Vec<StartupEntry> {
    let mut entries = Vec::new();
    let Some(key) = open_read_key(hkey, subkey, view) else {
        return entries;
    };

    unsafe {
        let mut index = 0u32;
        let mut value_name = vec![0u16; INITIAL_REGISTRY_NAME_CHARS];
        let mut value_data = vec![0u8; INITIAL_REGISTRY_DATA_BYTES];

        loop {
            let mut name_len = value_name.len() as u32;
            let mut data_len = value_data.len() as u32;
            let mut data_type: REG_VALUE_TYPE = 0;
            let result = RegEnumValueW(
                key,
                index,
                value_name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                &mut data_type,
                value_data.as_mut_ptr(),
                &mut data_len,
            );

            if result == 0 {
                let name_len = name_len as usize;
                let data_len = data_len as usize;
                if name_len >= value_name.len() || data_len > value_data.len() {
                    break;
                }
                let Some(name) = String::from_utf16(&value_name[..name_len])
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                else {
                    index = index.saturating_add(1);
                    continue;
                };
                let Some(command) = decode_registry_string(data_type, &value_data[..data_len])
                else {
                    index = index.saturating_add(1);
                    continue;
                };
                entries.push(StartupEntry {
                    name,
                    command,
                    source: source_label.to_string(),
                    enabled: true,
                });
                index = index.saturating_add(1);
                continue;
            }

            if result != ERROR_MORE_DATA {
                break;
            }

            // 错误返回的长度是下一轮需要的最小容量；同时翻倍应对竞态增长。
            let required_name = (name_len as usize).saturating_add(1);
            let required_data = data_len as usize;
            if required_name > MAX_REGISTRY_NAME_CHARS || required_data > MAX_REGISTRY_DATA_BYTES {
                break;
            }
            let next_name = required_name.max(value_name.len().saturating_mul(2));
            let next_data = required_data.max(value_data.len().saturating_mul(2));
            if next_name <= value_name.len() || next_data <= value_data.len() {
                break;
            }
            value_name.resize(next_name.min(MAX_REGISTRY_NAME_CHARS), 0);
            value_data.resize(next_data.min(MAX_REGISTRY_DATA_BYTES), 0);
        }

        RegCloseKey(key);
    }

    entries
}

/// 动态枚举一个卸载项根键下的子键。
fn enumerate_uninstall_subkeys(key: HKEY) -> Vec<String> {
    let mut names = Vec::new();
    let mut index = 0u32;
    let mut buffer = vec![0u16; INITIAL_REGISTRY_NAME_CHARS];

    unsafe {
        loop {
            let mut name_len = buffer.len() as u32;
            let result = RegEnumKeyExW(
                key,
                index,
                buffer.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if result == 0 {
                let name_len = name_len as usize;
                if name_len >= buffer.len() {
                    break;
                }
                if let Ok(name) = String::from_utf16(&buffer[..name_len]) {
                    let name = name.trim();
                    if !name.is_empty() {
                        names.push(name.to_string());
                    }
                }
                index = index.saturating_add(1);
                continue;
            }
            if result != ERROR_MORE_DATA {
                break;
            }

            let required_name = (name_len as usize).saturating_add(1);
            if required_name > MAX_REGISTRY_NAME_CHARS {
                break;
            }
            let next_name = required_name.max(buffer.len().saturating_mul(2));
            if next_name <= buffer.len() {
                break;
            }
            buffer.resize(next_name.min(MAX_REGISTRY_NAME_CHARS), 0);
        }
    }

    names
}

fn read_uninstall_apps() -> Vec<InstalledAppEntry> {
    let mut apps = Vec::new();
    let roots = [
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
    ];

    for (root, uninstall_path) in roots {
        for view in REGISTRY_VIEWS {
            let Some(uninstall_key) = open_read_key(root, uninstall_path, view) else {
                continue;
            };
            let subkeys = enumerate_uninstall_subkeys(uninstall_key);
            unsafe { RegCloseKey(uninstall_key) };

            for subkey_name in subkeys {
                let full_path = format!("{uninstall_path}\\{subkey_name}");
                let Some(app_key) = open_read_key(root, &full_path, view) else {
                    continue;
                };

                let name = read_registry_string(app_key, "DisplayName");
                let version = read_registry_string(app_key, "DisplayVersion");
                let publisher = read_registry_string(app_key, "Publisher");
                let install_date = read_registry_string(app_key, "InstallDate");
                let install_location = read_registry_string(app_key, "InstallLocation");
                unsafe { RegCloseKey(app_key) };

                let Some(name) = name else {
                    continue;
                };
                let mut app = app_from_registry_values(
                    Some(&name),
                    version.as_deref(),
                    publisher.as_deref(),
                    install_date.as_deref(),
                );
                app.install_location = registry_metric(install_location.as_deref());
                if !apps.iter().any(|existing: &InstalledAppEntry| {
                    existing.name.eq_ignore_ascii_case(&app.name)
                }) {
                    apps.push(app);
                }
            }
        }
    }

    apps
}

fn unsupported_security_bool(timestamp: u64) -> MetricValue<bool> {
    MetricValue::unsupported_at(
        "",
        SECURITY_SOURCE,
        "未接入可靠的 Secure Boot/TPM/Defender/Firewall Provider",
        timestamp,
    )
}

fn unsupported_security_string(timestamp: u64) -> MetricValue<String> {
    MetricValue::unsupported_at(
        "",
        SECURITY_SOURCE,
        "未接入可靠的 Secure Boot/TPM/Defender/Firewall Provider",
        timestamp,
    )
}

/// 采集 Windows 系统环境快照。
pub fn collect_windows_env() -> WindowsEnvSnapshot {
    let mut startup_items = Vec::new();
    let run_targets = [
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "HKCU\\Run",
        ),
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
            "HKCU\\RunOnce",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
            "HKLM\\Run",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
            "HKLM\\RunOnce",
        ),
    ];
    for (root, subkey, source) in run_targets {
        for view in REGISTRY_VIEWS {
            for entry in read_run_entries(root, subkey, source, view) {
                if !startup_items.contains(&entry) {
                    startup_items.push(entry);
                }
            }
        }
    }

    let timestamp = current_timestamp_ms();
    let security = SecurityStatusInfo {
        secure_boot_enabled: unsupported_security_bool(timestamp),
        tpm_present: unsupported_security_bool(timestamp),
        tpm_version: unsupported_security_string(timestamp),
        defender_enabled: unsupported_security_bool(timestamp),
        defender_realtime_protection: unsupported_security_bool(timestamp),
        firewall_domain_enabled: unsupported_security_bool(timestamp),
        firewall_private_enabled: unsupported_security_bool(timestamp),
        firewall_public_enabled: unsupported_security_bool(timestamp),
    };

    WindowsEnvSnapshot {
        startup_items,
        installed_apps_sample: read_uninstall_apps(),
        security_status: security,
        // 第一阶段没有 SCM Provider；空集合由后续聚合层标记 Unsupported。
        active_services_sample: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_uninstall_fields_remain_unavailable() {
        let app = app_from_registry_values(Some("Demo"), None, None, None);
        assert_eq!(app.name, "Demo");
        assert_eq!(app.version.value, None);
        assert_eq!(app.publisher.value, None);
    }
}

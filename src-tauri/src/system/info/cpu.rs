//! CPU 静态信息、拓扑、缓存与运行状态采集模块。
//!
//! 该模块只返回 CPU 指令、Windows API 和注册表实际提供的值；无法可靠读取的
//! 指标统一返回空值并保留对应的数据质量状态。

use super::quality::{current_timestamp_ms, MetricQuality, MetricValue};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, FILETIME};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD,
    REG_EXPAND_SZ, REG_SZ,
};
use windows_sys::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, RelationProcessorCore, GROUP_AFFINITY,
    PROCESSOR_RELATIONSHIP, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use windows_sys::Win32::System::Threading::{
    GetActiveProcessorCount, GetSystemTimes, ALL_PROCESSOR_GROUPS,
};

/// CPU 静态硬件信息与指令集。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CpuStaticInfo {
    pub name: MetricValue<String>,
    pub vendor: MetricValue<String>,
    pub brand: MetricValue<String>,
    pub family: MetricValue<u32>,
    pub model: MetricValue<u32>,
    pub stepping: MetricValue<u32>,
    pub physical_cores: MetricValue<u32>,
    pub logical_processors: MetricValue<u32>,
    pub l1_data_cache_kb: MetricValue<u32>,
    pub l1_inst_cache_kb: MetricValue<u32>,
    pub l2_cache_kb: MetricValue<u32>,
    pub l3_cache_kb: MetricValue<u32>,
    pub features: Vec<String>,
    pub virtualization: MetricValue<String>,
}

/// CPU 实时运行状态度量。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CpuRuntimeInfo {
    pub total_usage_percent: MetricValue<f64>,
    pub user_usage_percent: MetricValue<f64>,
    pub kernel_usage_percent: MetricValue<f64>,
    pub idle_percent: MetricValue<f64>,
    pub base_frequency_mhz: MetricValue<u32>,
    pub current_frequency_mhz: MetricValue<u32>,
    pub package_temperature_c: MetricValue<f64>,
    pub package_power_watts: MetricValue<f64>,
}

/// 由 CPUID 读取的处理器信息。
#[derive(Clone, Debug, PartialEq)]
struct CpuidInfo {
    vendor: String,
    brand: Option<String>,
    family: u32,
    model: u32,
    stepping: u32,
    features: Vec<String>,
    virtualization: Option<String>,
}

/// 由两个系统时间点计算出的 CPU 百分比。
#[derive(Clone, Copy, Debug, PartialEq)]
struct CpuPercent {
    total: f64,
    user: f64,
    kernel: f64,
    idle: f64,
}

/// 按缓存层级与类型汇总的确定性缓存容量。
#[derive(Clone, Copy, Debug, Default)]
struct CacheInfo {
    l1_data_bytes: Option<u64>,
    l1_inst_bytes: Option<u64>,
    l2_bytes: Option<u64>,
    l3_bytes: Option<u64>,
}

static PREV_CPU_STATE: Mutex<Option<(u64, u64, u64)>> = Mutex::new(None);

const MAX_REGISTRY_VALUE_BYTES: usize = 1024 * 1024;
const MAX_PROCESSOR_INFO_BYTES: usize = 16 * 1024 * 1024;
const MAX_PROCESSOR_INFO_RETRIES: usize = 3;

fn bounded_registry_length(length: u32) -> Result<usize, MetricQuality> {
    let length = usize::try_from(length).map_err(|_| MetricQuality::Invalid)?;
    (length <= MAX_REGISTRY_VALUE_BYTES)
        .then_some(length)
        .ok_or(MetricQuality::Invalid)
}

fn bounded_processor_info_length(length: u32) -> Result<usize, MetricQuality> {
    let length = usize::try_from(length).map_err(|_| MetricQuality::Invalid)?;
    (length <= MAX_PROCESSOR_INFO_BYTES)
        .then_some(length)
        .ok_or(MetricQuality::Invalid)
}

fn ft_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

/// 将指定质量状态转换为空值指标。
fn missing_metric<T>(
    unit: &str,
    source: &str,
    quality: MetricQuality,
    reason: &str,
) -> MetricValue<T> {
    let timestamp = current_timestamp_ms();
    match quality {
        MetricQuality::Unsupported => MetricValue::unsupported_at(unit, source, reason, timestamp),
        MetricQuality::Unavailable => MetricValue::unavailable_at(unit, source, reason, timestamp),
        MetricQuality::PermissionDenied => {
            MetricValue::permission_denied_at(unit, source, reason, timestamp)
        }
        MetricQuality::DriverMissing => {
            MetricValue::driver_missing_at(unit, source, reason, timestamp)
        }
        MetricQuality::ApiUnavailable => {
            MetricValue::api_unavailable_at(unit, source, reason, timestamp)
        }
        MetricQuality::ReadError => MetricValue::read_error_at(unit, source, reason, timestamp),
        MetricQuality::Invalid => MetricValue::invalid_at(unit, source, reason, timestamp),
        MetricQuality::Good
        | MetricQuality::Estimated
        | MetricQuality::Stale
        | MetricQuality::Unknown => MetricValue::unavailable_at(unit, source, reason, timestamp),
    }
}

/// 查询指定 CPU 注册表值，先查询长度后分配动态缓冲区。
fn query_registry_value(value_name: &str) -> Result<(u32, Vec<u8>), MetricQuality> {
    let subkey: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return Err(MetricQuality::ApiUnavailable);
        }

        let mut data_type = 0u32;
        let mut data_len = 0u32;
        let initial_result = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut data_len,
        );
        if initial_result != 0 {
            let _ = RegCloseKey(hkey);
            return Err(MetricQuality::ApiUnavailable);
        }

        let initial_len = match bounded_registry_length(data_len) {
            Ok(length) => length,
            Err(quality) => {
                let _ = RegCloseKey(hkey);
                return Err(quality);
            }
        };
        let mut data = vec![0u8; initial_len];
        let mut attempts = 0u8;
        loop {
            let mut actual_len = data.len() as u32;
            let data_ptr = if data.is_empty() {
                std::ptr::null_mut()
            } else {
                data.as_mut_ptr()
            };
            let result = RegQueryValueExW(
                hkey,
                value_name.as_ptr(),
                std::ptr::null(),
                &mut data_type,
                data_ptr,
                &mut actual_len,
            );

            if result == ERROR_MORE_DATA {
                attempts = attempts.saturating_add(1);
                if attempts >= 3 || actual_len == 0 {
                    let _ = RegCloseKey(hkey);
                    return Err(MetricQuality::ReadError);
                }
                let next_len = match bounded_registry_length(actual_len) {
                    Ok(length) => length,
                    Err(quality) => {
                        let _ = RegCloseKey(hkey);
                        return Err(quality);
                    }
                };
                data.resize(next_len, 0);
                continue;
            }

            if result != 0 || actual_len as usize > data.len() {
                let _ = RegCloseKey(hkey);
                return Err(MetricQuality::ReadError);
            }

            data.truncate(actual_len as usize);
            let _ = RegCloseKey(hkey);
            return Ok((data_type, data));
        }
    }
}

/// 解析带终止 NUL 的注册表 UTF-16 字符串。
fn decode_registry_string(data: &[u8]) -> Result<Option<String>, MetricQuality> {
    if data.len() % 2 != 0 {
        return Err(MetricQuality::ReadError);
    }

    let code_units: Vec<u16> = data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    let Some(end) = code_units.iter().position(|unit| *unit == 0) else {
        return Err(MetricQuality::ReadError);
    };
    let value = String::from_utf16(&code_units[..end]).map_err(|_| MetricQuality::ReadError)?;
    let value = value.trim().to_string();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

/// 读取真实的处理器名称注册表值及其质量状态。
fn get_cpu_name_from_reg_result() -> Result<Option<String>, MetricQuality> {
    let (data_type, data) = query_registry_value("ProcessorNameString")?;
    if data_type != REG_SZ && data_type != REG_EXPAND_SZ {
        return Err(MetricQuality::ReadError);
    }
    decode_registry_string(&data)
}

/// 读取处理器名称；注册表读取失败时保持历史 API 的空字符串约定。
pub fn get_cpu_name_from_reg() -> String {
    get_cpu_name_from_reg_result()
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// 读取真实的处理器基准频率注册表值。
fn get_cpu_base_mhz() -> Result<Option<u32>, MetricQuality> {
    let (data_type, data) = query_registry_value("~MHz")?;
    if data_type != REG_DWORD || data.len() != std::mem::size_of::<u32>() {
        return Err(MetricQuality::ReadError);
    }

    let bytes: [u8; 4] = data
        .as_slice()
        .try_into()
        .map_err(|_| MetricQuality::ReadError)?;
    let mhz = u32::from_le_bytes(bytes);
    if mhz == 0 {
        return Err(MetricQuality::Invalid);
    }
    Ok(Some(mhz))
}

#[cfg(target_arch = "x86_64")]
fn read_cpuid_limits() -> Result<(String, u32, u32), MetricQuality> {
    use core::arch::x86_64::__cpuid;

    let basic = __cpuid(0);
    let mut vendor_bytes = [0u8; 12];
    vendor_bytes[0..4].copy_from_slice(&basic.ebx.to_le_bytes());
    vendor_bytes[4..8].copy_from_slice(&basic.edx.to_le_bytes());
    vendor_bytes[8..12].copy_from_slice(&basic.ecx.to_le_bytes());
    let vendor = std::str::from_utf8(&vendor_bytes)
        .map_err(|_| MetricQuality::ReadError)?
        .trim_matches('\0')
        .trim()
        .to_string();
    if vendor.is_empty() {
        return Err(MetricQuality::ReadError);
    }

    let extended = __cpuid(0x8000_0000);
    Ok((vendor, basic.eax, extended.eax))
}

#[cfg(not(target_arch = "x86_64"))]
fn read_cpuid_limits() -> Result<(String, u32, u32), MetricQuality> {
    Err(MetricQuality::ApiUnavailable)
}

#[cfg(target_arch = "x86_64")]
fn read_xcr0_if_supported(osxsave: bool) -> u64 {
    if !osxsave {
        return 0;
    }

    // 只有 CPUID 已确认 OSXSAVE 时才执行 XGETBV，避免非法指令。
    unsafe { core::arch::x86_64::_xgetbv(0) }
}

#[cfg(not(target_arch = "x86_64"))]
fn read_xcr0_if_supported(_osxsave: bool) -> u64 {
    0
}

fn add_feature(features: &mut Vec<String>, feature: &str) {
    features.push(feature.to_string());
}

/// 判断依赖 AVX 状态的 CPUID 特征是否可安全使用。
///
/// FMA3 和 F16C 还要求 AVX 硬件位、OSXSAVE 与 XCR0[2:1] 状态均可用。
fn avx_dependent_feature_usable(
    hardware_feature_supported: bool,
    avx_supported: bool,
    osxsave: bool,
    avx_state_usable: bool,
) -> bool {
    hardware_feature_supported && avx_supported && osxsave && avx_state_usable
}

/// 执行 CPUID 并解析真实厂商、型号、品牌、特征与硬件虚拟化位。
fn query_cpuid_features() -> Result<CpuidInfo, MetricQuality> {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::{__cpuid, __cpuid_count};

        let (vendor, max_basic, max_extended) = read_cpuid_limits()?;
        if max_basic < 1 {
            return Err(MetricQuality::ApiUnavailable);
        }

        let leaf1 = __cpuid(1);
        let stepping = leaf1.eax & 0x0f;
        let base_model = (leaf1.eax >> 4) & 0x0f;
        let base_family = (leaf1.eax >> 8) & 0x0f;
        let extended_model = (leaf1.eax >> 16) & 0x0f;
        let extended_family = (leaf1.eax >> 20) & 0xff;
        let family = if base_family == 0x0f {
            base_family + extended_family
        } else {
            base_family
        };
        let model = if base_family == 0x06 || base_family == 0x0f {
            (extended_model << 4) | base_model
        } else {
            base_model
        };

        let osxsave = (leaf1.ecx & (1 << 27)) != 0;
        let xcr0 = read_xcr0_if_supported(osxsave);
        let avx_supported = (leaf1.ecx & (1 << 28)) != 0;
        let avx_state_usable = (xcr0 & 0x06) == 0x06;
        let avx512_state_usable = (xcr0 & 0xe6) == 0xe6;
        let mut features = Vec::new();

        if (leaf1.edx & (1 << 23)) != 0 {
            add_feature(&mut features, "MMX");
        }
        if (leaf1.edx & (1 << 25)) != 0 {
            add_feature(&mut features, "SSE");
        }
        if (leaf1.edx & (1 << 26)) != 0 {
            add_feature(&mut features, "SSE2");
        }
        if (leaf1.edx & (1 << 28)) != 0 {
            add_feature(&mut features, "HTT");
        }
        if (leaf1.ecx & (1 << 0)) != 0 {
            add_feature(&mut features, "SSE3");
        }
        if (leaf1.ecx & (1 << 9)) != 0 {
            add_feature(&mut features, "SSSE3");
        }
        if avx_dependent_feature_usable(
            (leaf1.ecx & (1 << 12)) != 0,
            avx_supported,
            osxsave,
            avx_state_usable,
        ) {
            add_feature(&mut features, "FMA3");
        }
        if (leaf1.ecx & (1 << 19)) != 0 {
            add_feature(&mut features, "SSE4.1");
        }
        if (leaf1.ecx & (1 << 20)) != 0 {
            add_feature(&mut features, "SSE4.2");
        }
        if (leaf1.ecx & (1 << 23)) != 0 {
            add_feature(&mut features, "POPCNT");
        }
        if (leaf1.ecx & (1 << 25)) != 0 {
            add_feature(&mut features, "AES-NI");
        }
        if osxsave {
            add_feature(&mut features, "OSXSAVE");
        }
        if avx_supported && avx_state_usable {
            add_feature(&mut features, "AVX");
        }
        if avx_dependent_feature_usable(
            (leaf1.ecx & (1 << 29)) != 0,
            avx_supported,
            osxsave,
            avx_state_usable,
        ) {
            add_feature(&mut features, "F16C");
        }
        if (leaf1.ecx & (1 << 30)) != 0 {
            add_feature(&mut features, "RDRAND");
        }

        let vmx_supported = (leaf1.ecx & (1 << 5)) != 0;
        if vmx_supported {
            add_feature(&mut features, "VMX");
        }

        if max_basic >= 7 {
            let leaf7 = __cpuid_count(7, 0);
            if (leaf7.ebx & (1 << 3)) != 0 {
                add_feature(&mut features, "BMI1");
            }
            if (leaf7.ebx & (1 << 5)) != 0 && avx_state_usable {
                add_feature(&mut features, "AVX2");
            }
            if (leaf7.ebx & (1 << 8)) != 0 {
                add_feature(&mut features, "BMI2");
            }
            if (leaf7.ebx & (1 << 16)) != 0 && avx512_state_usable {
                add_feature(&mut features, "AVX512F");
            }
            if (leaf7.ebx & (1 << 18)) != 0 {
                add_feature(&mut features, "RDSEED");
            }
            if (leaf7.ebx & (1 << 19)) != 0 {
                add_feature(&mut features, "ADX");
            }
            if (leaf7.ebx & (1 << 29)) != 0 {
                add_feature(&mut features, "SHA");
            }
        }

        let svm_supported = if max_extended >= 0x8000_0001 {
            let extended_leaf1 = __cpuid(0x8000_0001);
            (extended_leaf1.ecx & (1 << 2)) != 0
        } else {
            false
        };
        if svm_supported {
            add_feature(&mut features, "SVM");
        }

        let brand = if max_extended >= 0x8000_0004 {
            let mut brand_bytes = [0u8; 48];
            for (index, leaf) in (0x8000_0002..=0x8000_0004).enumerate() {
                let value = __cpuid(leaf);
                let offset = index * 16;
                brand_bytes[offset..offset + 4].copy_from_slice(&value.eax.to_le_bytes());
                brand_bytes[offset + 4..offset + 8].copy_from_slice(&value.ebx.to_le_bytes());
                brand_bytes[offset + 8..offset + 12].copy_from_slice(&value.ecx.to_le_bytes());
                brand_bytes[offset + 12..offset + 16].copy_from_slice(&value.edx.to_le_bytes());
            }
            let brand_text =
                std::str::from_utf8(&brand_bytes).map_err(|_| MetricQuality::ReadError)?;
            let end = brand_text.find('\0').unwrap_or(brand_text.len());
            let value = brand_text[..end].trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        } else {
            None
        };

        let virtualization = if vmx_supported {
            Some("VMX".to_string())
        } else if svm_supported {
            Some("SVM".to_string())
        } else {
            None
        };

        Ok(CpuidInfo {
            vendor,
            brand,
            family,
            model,
            stepping,
            features,
            virtualization,
        })
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Err(MetricQuality::ApiUnavailable)
    }
}

fn add_cache_size(slot: &mut Option<u64>, size_bytes: u64) -> Result<(), MetricQuality> {
    let current = (*slot).unwrap_or(0);
    let total = current
        .checked_add(size_bytes)
        .ok_or(MetricQuality::ReadError)?;
    *slot = Some(total);
    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn parse_deterministic_cache(leaf: u32) -> Result<CacheInfo, MetricQuality> {
    use core::arch::x86_64::__cpuid_count;

    let mut cache_info = CacheInfo::default();
    let mut saw_cache = false;
    let mut reached_end = false;

    for subleaf in 0..64_u32 {
        let value = __cpuid_count(leaf, subleaf);
        let cache_type = value.eax & 0x1f;
        if cache_type == 0 {
            reached_end = true;
            break;
        }
        if !matches!(cache_type, 1..=3) {
            return Err(MetricQuality::ReadError);
        }

        let level = (value.eax >> 5) & 0x07;
        if level == 0 {
            return Err(MetricQuality::ReadError);
        }

        let line_size = u64::from(value.ebx & 0x0fff) + 1;
        let partitions = u64::from((value.ebx >> 12) & 0x03ff) + 1;
        let ways = u64::from((value.ebx >> 22) & 0x03ff) + 1;
        let sets = u64::from(value.ecx) + 1;
        let size_bytes = line_size
            .checked_mul(partitions)
            .and_then(|size| size.checked_mul(ways))
            .and_then(|size| size.checked_mul(sets))
            .ok_or(MetricQuality::ReadError)?;
        if size_bytes == 0 {
            return Err(MetricQuality::ReadError);
        }
        saw_cache = true;

        match (level, cache_type) {
            (1, 1) => add_cache_size(&mut cache_info.l1_data_bytes, size_bytes)?,
            (1, 2) => add_cache_size(&mut cache_info.l1_inst_bytes, size_bytes)?,
            (1, 3) => {
                add_cache_size(&mut cache_info.l1_data_bytes, size_bytes)?;
                add_cache_size(&mut cache_info.l1_inst_bytes, size_bytes)?;
            }
            (2, 1..=3) => add_cache_size(&mut cache_info.l2_bytes, size_bytes)?,
            (3, 1..=3) => add_cache_size(&mut cache_info.l3_bytes, size_bytes)?,
            _ => {}
        }
    }

    if !reached_end {
        return Err(MetricQuality::ReadError);
    }
    if !saw_cache {
        return Err(MetricQuality::ApiUnavailable);
    }
    Ok(cache_info)
}

fn bytes_to_kb(size_bytes: u64) -> Option<u32> {
    if size_bytes == 0 || size_bytes % 1024 != 0 {
        return None;
    }
    u32::try_from(size_bytes / 1024).ok()
}

#[cfg(target_arch = "x86_64")]
fn query_cpuid_cache_info() -> Result<CacheInfo, MetricQuality> {
    let (vendor, max_basic, max_extended) = read_cpuid_limits()?;
    let leaf = if vendor == "GenuineIntel" && max_basic >= 4 {
        4
    } else if vendor == "AuthenticAMD" && max_extended >= 0x8000_001d {
        0x8000_001d
    } else {
        return Err(MetricQuality::ApiUnavailable);
    };
    parse_deterministic_cache(leaf)
}

#[cfg(not(target_arch = "x86_64"))]
fn query_cpuid_cache_info() -> Result<CacheInfo, MetricQuality> {
    Err(MetricQuality::ApiUnavailable)
}

#[cfg(target_arch = "x86_64")]
fn cache_metric(value: Option<u32>) -> MetricValue<u32> {
    match value {
        Some(size_kb) if size_kb > 0 => {
            MetricValue::good(size_kb, "KB", "CPUID_DeterministicCache")
        }
        _ => MetricValue::unsupported(
            "KB",
            "CPUID_DeterministicCache",
            "未能可靠解析确定性缓存叶片",
        ),
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn cache_metric(_value: Option<u32>) -> MetricValue<u32> {
    MetricValue::unsupported(
        "KB",
        "CPUID_DeterministicCache",
        "当前架构没有可用的确定性缓存叶片",
    )
}

/// 校验一个 RelationProcessorCore 变长记录，并返回其步进长度。
///
/// 这里只按字节读取固定头和 GroupCount，避免把包含变长 GroupMask 的外层结构体
/// 大小误当成每条记录的固定长度。
fn validate_processor_core_record(
    buffer: &[u8],
    offset: usize,
    returned_len: usize,
) -> Result<usize, MetricQuality> {
    const EX_HEADER_SIZE: usize = 8; // Relationship(4) + Size(4)
    let alignment = std::mem::align_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>();
    let processor_fixed_size = std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupMask);
    let group_count_offset = std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupCount);
    let group_mask_size = std::mem::size_of::<GROUP_AFFINITY>();

    if returned_len == 0
        || returned_len > buffer.len()
        || offset >= returned_len
        || offset % alignment != 0
    {
        return Err(MetricQuality::ReadError);
    }

    let remaining = returned_len
        .checked_sub(offset)
        .ok_or(MetricQuality::ReadError)?;
    if remaining < EX_HEADER_SIZE {
        return Err(MetricQuality::ReadError);
    }

    let relationship = i32::from_ne_bytes(
        buffer[offset..offset + 4]
            .try_into()
            .map_err(|_| MetricQuality::ReadError)?,
    );
    if relationship != RelationProcessorCore {
        return Err(MetricQuality::ReadError);
    }

    let record_size = u32::from_ne_bytes(
        buffer[offset + 4..offset + 8]
            .try_into()
            .map_err(|_| MetricQuality::ReadError)?,
    ) as usize;
    let minimum_record_size = EX_HEADER_SIZE
        .checked_add(processor_fixed_size)
        .ok_or(MetricQuality::ReadError)?;
    let record_end = offset
        .checked_add(record_size)
        .ok_or(MetricQuality::ReadError)?;
    if record_size < minimum_record_size
        || record_size % alignment != 0
        || record_end > returned_len
    {
        return Err(MetricQuality::ReadError);
    }

    let group_count_start = offset
        .checked_add(EX_HEADER_SIZE)
        .and_then(|start| start.checked_add(group_count_offset))
        .ok_or(MetricQuality::ReadError)?;
    let group_count_end = group_count_start
        .checked_add(std::mem::size_of::<u16>())
        .ok_or(MetricQuality::ReadError)?;
    if group_count_end > record_end {
        return Err(MetricQuality::ReadError);
    }
    let group_count = u16::from_ne_bytes(
        buffer[group_count_start..group_count_end]
            .try_into()
            .map_err(|_| MetricQuality::ReadError)?,
    );
    if group_count == 0 {
        return Err(MetricQuality::ReadError);
    }

    let group_masks_size = usize::from(group_count)
        .checked_mul(group_mask_size)
        .ok_or(MetricQuality::ReadError)?;
    let required_record_size = minimum_record_size
        .checked_add(group_masks_size)
        .ok_or(MetricQuality::ReadError)?;
    if record_size < required_record_size {
        return Err(MetricQuality::ReadError);
    }

    Ok(record_size)
}

/// 使用动态缓冲区统计 RelationProcessorCore 记录数。
fn get_physical_core_count() -> Result<u32, MetricQuality> {
    unsafe {
        let mut required_len = 0u32;
        let _ = GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            std::ptr::null_mut(),
            &mut required_len,
        );
        if required_len == 0 {
            return Err(MetricQuality::ApiUnavailable);
        }

        let word_size = std::mem::size_of::<usize>();
        for attempt in 0..MAX_PROCESSOR_INFO_RETRIES {
            let bounded_len = bounded_processor_info_length(required_len)?;
            let word_count = bounded_len
                .checked_add(word_size - 1)
                .and_then(|size| size.checked_div(word_size))
                .ok_or(MetricQuality::ReadError)?;
            let mut buffer = vec![0usize; word_count];
            let allocated_len = word_count
                .checked_mul(word_size)
                .ok_or(MetricQuality::ReadError)?;
            let mut returned_len =
                u32::try_from(allocated_len).map_err(|_| MetricQuality::ReadError)?;
            if GetLogicalProcessorInformationEx(
                RelationProcessorCore,
                buffer.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
                &mut returned_len,
            ) == 0
            {
                let returned = usize::try_from(returned_len).unwrap_or(0);
                if returned > allocated_len && attempt + 1 < MAX_PROCESSOR_INFO_RETRIES {
                    required_len = returned_len;
                    continue;
                }
                return Err(MetricQuality::ReadError);
            }

            let returned_len =
                usize::try_from(returned_len).map_err(|_| MetricQuality::ReadError)?;
            if returned_len == 0 || returned_len > allocated_len {
                return Err(MetricQuality::ReadError);
            }

            let buffer_bytes =
                std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), allocated_len);
            let mut offset = 0usize;
            let mut count = 0u32;
            while offset < returned_len {
                let record_size =
                    validate_processor_core_record(buffer_bytes, offset, returned_len)?;
                count = count.checked_add(1).ok_or(MetricQuality::ReadError)?;
                offset = offset
                    .checked_add(record_size)
                    .ok_or(MetricQuality::ReadError)?;
            }

            if offset != returned_len || count == 0 {
                return Err(MetricQuality::ReadError);
            }
            return Ok(count);
        }
        Err(MetricQuality::ReadError)
    }
}

/// 读取所有处理器组中的真实活动逻辑处理器数量。
fn get_logical_processor_count() -> Result<u32, MetricQuality> {
    let count = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if count == 0 {
        Err(MetricQuality::ApiUnavailable)
    } else {
        Ok(count)
    }
}

/// 将缓存解析结果转换为四个公开缓存指标。
fn collect_cache_metrics(cache_result: Result<CacheInfo, MetricQuality>) -> [MetricValue<u32>; 4] {
    match cache_result {
        Ok(cache_info) => [
            cache_metric(cache_info.l1_data_bytes.and_then(bytes_to_kb)),
            cache_metric(cache_info.l1_inst_bytes.and_then(bytes_to_kb)),
            cache_metric(cache_info.l2_bytes.and_then(bytes_to_kb)),
            cache_metric(cache_info.l3_bytes.and_then(bytes_to_kb)),
        ],
        Err(_) => [
            MetricValue::unsupported(
                "KB",
                "CPUID_DeterministicCache",
                "确定性缓存叶片不可用或解析失败",
            ),
            MetricValue::unsupported(
                "KB",
                "CPUID_DeterministicCache",
                "确定性缓存叶片不可用或解析失败",
            ),
            MetricValue::unsupported(
                "KB",
                "CPUID_DeterministicCache",
                "确定性缓存叶片不可用或解析失败",
            ),
            MetricValue::unsupported(
                "KB",
                "CPUID_DeterministicCache",
                "确定性缓存叶片不可用或解析失败",
            ),
        ],
    }
}

/// 采集 CPU 静态硬件信息。
pub fn collect_cpu_static_info() -> CpuStaticInfo {
    let registry_name = get_cpu_name_from_reg_result();
    let cpuid = query_cpuid_features();
    let cache_metrics = collect_cache_metrics(query_cpuid_cache_info());
    let [l1_data_cache_kb, l1_inst_cache_kb, l2_cache_kb, l3_cache_kb] = cache_metrics;

    let name = match &registry_name {
        Ok(Some(value)) => MetricValue::good(value.clone(), "", "Registry_ProcessorNameString"),
        _ => match &cpuid {
            Ok(info) => match &info.brand {
                Some(value) => MetricValue::good(value.clone(), "", "CPUID_ExtendedBrandString"),
                None => MetricValue::unavailable(
                    "",
                    "Registry_ProcessorNameString",
                    "注册表和 CPUID 均未提供处理器名称",
                ),
            },
            Err(quality) => missing_metric(
                "",
                "Registry_ProcessorNameString",
                *quality,
                "注册表和 CPUID 均未提供处理器名称",
            ),
        },
    };

    let brand = match &cpuid {
        Ok(info) => match &info.brand {
            Some(value) => MetricValue::good(value.clone(), "", "CPUID_ExtendedBrandString"),
            None => MetricValue::unavailable(
                "",
                "CPUID_ExtendedBrandString",
                "CPUID 没有扩展品牌字符串叶片或内容",
            ),
        },
        Err(quality) => missing_metric(
            "",
            "CPUID_ExtendedBrandString",
            *quality,
            "CPUID 品牌字符串读取失败",
        ),
    };

    let (vendor, family, model, stepping, features, virtualization) = match &cpuid {
        Ok(info) => {
            let virtualization = match &info.virtualization {
                Some(value) => MetricValue::good(value.clone(), "", "CPUID_FeatureFlag"),
                None => MetricValue::unsupported(
                    "",
                    "CPUID_FeatureFlag",
                    "CPUID 未报告 VMX 或 SVM 硬件支持位",
                ),
            };
            (
                MetricValue::good(info.vendor.clone(), "", "CPUID_Leaf0"),
                MetricValue::good(info.family, "", "CPUID_Leaf1"),
                MetricValue::good(info.model, "", "CPUID_Leaf1"),
                MetricValue::good(info.stepping, "", "CPUID_Leaf1"),
                info.features.clone(),
                virtualization,
            )
        }
        Err(quality) => (
            missing_metric(
                "",
                "CPUID_Leaf0",
                *quality,
                "CPUID 基础叶片不可用或解析失败",
            ),
            missing_metric(
                "",
                "CPUID_Leaf1",
                *quality,
                "CPUID 处理器信息叶片不可用或解析失败",
            ),
            missing_metric(
                "",
                "CPUID_Leaf1",
                *quality,
                "CPUID 处理器信息叶片不可用或解析失败",
            ),
            missing_metric(
                "",
                "CPUID_Leaf1",
                *quality,
                "CPUID 处理器信息叶片不可用或解析失败",
            ),
            Vec::new(),
            missing_metric(
                "",
                "CPUID_FeatureFlag",
                *quality,
                "CPUID 虚拟化硬件位读取失败",
            ),
        ),
    };

    let physical_cores = match get_physical_core_count() {
        Ok(value) => MetricValue::good(value, "核", "Win32_GetLogicalProcessorInformationEx"),
        Err(quality) => missing_metric(
            "核",
            "Win32_GetLogicalProcessorInformationEx",
            quality,
            "无法读取处理器核心拓扑",
        ),
    };
    let logical_processors = match get_logical_processor_count() {
        Ok(value) => MetricValue::good(value, "线程", "Win32_GetActiveProcessorCount"),
        Err(quality) => missing_metric(
            "线程",
            "Win32_GetActiveProcessorCount",
            quality,
            "无法读取活动逻辑处理器数量",
        ),
    };

    CpuStaticInfo {
        name,
        vendor,
        brand,
        family,
        model,
        stepping,
        physical_cores,
        logical_processors,
        l1_data_cache_kb,
        l1_inst_cache_kb,
        l2_cache_kb,
        l3_cache_kb,
        features,
        virtualization,
    }
}

/// 根据前后两次系统时间计数器计算 CPU 百分比。
fn compute_cpu_percent(
    previous: Option<(u64, u64, u64)>,
    current: (u64, u64, u64),
) -> Option<CpuPercent> {
    let (previous_idle, previous_kernel, previous_user) = previous?;
    let (current_idle, current_kernel, current_user) = current;
    let idle_delta = current_idle.checked_sub(previous_idle)?;
    let kernel_delta = current_kernel.checked_sub(previous_kernel)?;
    let user_delta = current_user.checked_sub(previous_user)?;
    let total_delta = kernel_delta.checked_add(user_delta)?;
    if total_delta == 0 || idle_delta > total_delta {
        return None;
    }
    let kernel_busy_delta = kernel_delta.checked_sub(idle_delta)?;
    let busy_delta = total_delta.checked_sub(idle_delta)?;
    let percentage = |value: u64| ((value as f64 / total_delta as f64) * 1000.0).round() / 10.0;

    Some(CpuPercent {
        total: percentage(busy_delta),
        user: percentage(user_delta),
        kernel: percentage(kernel_busy_delta),
        idle: percentage(idle_delta),
    })
}

fn percent_metrics(percent: CpuPercent) -> [MetricValue<f64>; 4] {
    [
        MetricValue::good(percent.total, "%", "Win32_GetSystemTimes"),
        MetricValue::good(percent.user, "%", "Win32_GetSystemTimes"),
        MetricValue::good(percent.kernel, "%", "Win32_GetSystemTimes"),
        MetricValue::good(percent.idle, "%", "Win32_GetSystemTimes"),
    ]
}

fn missing_percent_metrics(quality: MetricQuality, reason: &str) -> [MetricValue<f64>; 4] {
    [
        missing_metric("%", "Win32_GetSystemTimes", quality, reason),
        missing_metric("%", "Win32_GetSystemTimes", quality, reason),
        missing_metric("%", "Win32_GetSystemTimes", quality, reason),
        missing_metric("%", "Win32_GetSystemTimes", quality, reason),
    ]
}

fn read_system_times() -> Result<(u64, u64, u64), MetricQuality> {
    unsafe {
        let mut idle_time: FILETIME = std::mem::zeroed();
        let mut kernel_time: FILETIME = std::mem::zeroed();
        let mut user_time: FILETIME = std::mem::zeroed();
        if GetSystemTimes(&mut idle_time, &mut kernel_time, &mut user_time) == 0 {
            return Err(MetricQuality::ReadError);
        }
        Ok((
            ft_to_u64(idle_time),
            ft_to_u64(kernel_time),
            ft_to_u64(user_time),
        ))
    }
}

/// 采集 CPU 实时运行状态。
pub fn collect_cpu_runtime_info() -> CpuRuntimeInfo {
    let usage_metrics = match read_system_times() {
        Err(quality) => missing_percent_metrics(quality, "GetSystemTimes 读取失败"),
        Ok(current) => {
            let mut previous_guard = match PREV_CPU_STATE.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let previous = *previous_guard;
            *previous_guard = Some(current);

            match compute_cpu_percent(previous, current) {
                Some(percent) => percent_metrics(percent),
                None if previous.is_none() => {
                    missing_percent_metrics(MetricQuality::Unavailable, "首次采样尚无前一计数器")
                }
                None => missing_percent_metrics(
                    MetricQuality::Invalid,
                    "系统时间计数器回退、溢出或差值非法",
                ),
            }
        }
    };
    let [total_usage_percent, user_usage_percent, kernel_usage_percent, idle_percent] =
        usage_metrics;

    let base_frequency_mhz = match get_cpu_base_mhz() {
        Ok(Some(value)) => MetricValue::good(value, "MHz", "Registry_CentralProcessor"),
        Ok(None) => MetricValue::unavailable(
            "MHz",
            "Registry_CentralProcessor",
            "注册表未提供有效的 ~MHz 值",
        ),
        Err(quality) => missing_metric(
            "MHz",
            "Registry_CentralProcessor",
            quality,
            "注册表 ~MHz 值缺失、类型错误或读取失败",
        ),
    };

    let current_frequency_mhz = MetricValue::unsupported(
        "MHz",
        "Win32_ProcessorFrequency",
        "没有可靠的实时 CPU 频率用户态来源",
    );
    let package_temperature_c = MetricValue::unsupported(
        "°C",
        "Win32_SensorProvider",
        "Windows 标准用户态没有统一可靠的 CPU 温度接口",
    );
    let package_power_watts = MetricValue::unsupported(
        "W",
        "Win32_SensorProvider",
        "需要硬件专用功耗传感器或驱动支持",
    );

    CpuRuntimeInfo {
        total_usage_percent,
        user_usage_percent,
        kernel_usage_percent,
        idle_percent,
        base_frequency_mhz,
        current_frequency_mhz,
        package_temperature_c,
        package_power_watts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short_processor_core_fixture() -> Vec<u8> {
        let header_size = 8;
        let processor_relationship_fixed_size = 24;
        let group_mask_size = std::mem::size_of::<usize>() + 8;
        let required_size = header_size + processor_relationship_fixed_size + group_mask_size;
        let alignment = std::mem::align_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>();
        let record_size = (required_size + alignment - 1) / alignment * alignment;
        let mut record = vec![0u8; record_size];

        record[0..4].copy_from_slice(&(RelationProcessorCore as i32).to_ne_bytes());
        record[4..8].copy_from_slice(&(record_size as u32).to_ne_bytes());
        record[header_size + 22..header_size + 24].copy_from_slice(&1_u16.to_ne_bytes());
        record[header_size + processor_relationship_fixed_size] = 1;
        record
    }

    #[test]
    fn processor_core_length_accepts_short_one_group_fixture() {
        let record = short_processor_core_fixture();
        assert_eq!(
            validate_processor_core_record(&record, 0, record.len()),
            Ok(record.len())
        );
    }

    #[test]
    fn dynamic_cpu_lengths_reject_explicitly_over_limit_values() {
        assert_eq!(
            bounded_registry_length((MAX_REGISTRY_VALUE_BYTES + 1) as u32),
            Err(MetricQuality::Invalid)
        );
        assert_eq!(
            bounded_processor_info_length((MAX_PROCESSOR_INFO_BYTES + 1) as u32),
            Err(MetricQuality::Invalid)
        );
        assert_eq!(bounded_registry_length(4096), Ok(4096));
        assert_eq!(bounded_processor_info_length(4096), Ok(4096));
    }

    #[test]
    fn processor_core_length_rejects_truncated_fixture() {
        let record = short_processor_core_fixture();
        assert_eq!(
            validate_processor_core_record(&record, 0, record.len() - 1),
            Err(MetricQuality::ReadError)
        );
    }

    #[test]
    fn avx_dependent_features_require_hardware_and_os_state() {
        assert!(avx_dependent_feature_usable(true, true, true, true));
        assert!(!avx_dependent_feature_usable(false, true, true, true));
        assert!(!avx_dependent_feature_usable(true, false, true, true));
        assert!(!avx_dependent_feature_usable(true, true, false, true));
        assert!(!avx_dependent_feature_usable(true, true, true, false));
    }

    #[test]
    fn cpu_counter_reset_returns_no_percentage() {
        let previous = Some((100_u64, 200_u64, 300_u64));
        let current = (90_u64, 190_u64, 290_u64);
        assert_eq!(compute_cpu_percent(previous, current), None);
    }

    #[test]
    fn first_cpu_sample_is_not_a_fake_good_value() {
        assert_eq!(compute_cpu_percent(None, (100, 200, 300)), None);
    }

    #[test]
    fn invalid_idle_delta_returns_no_percentage() {
        let previous = Some((100_u64, 200_u64, 300_u64));
        let current = (200_u64, 250_u64, 300_u64);
        assert_eq!(compute_cpu_percent(previous, current), None);
    }

    #[test]
    fn valid_counter_delta_reports_percentages() {
        let previous = Some((100_u64, 200_u64, 300_u64));
        let current = (200_u64, 300_u64, 500_u64);
        let actual = compute_cpu_percent(previous, current).expect("有效计数器差值应产生百分比");

        assert_eq!(actual.total, 66.7);
        assert_eq!(actual.user, 66.7);
        assert_eq!(actual.kernel, 0.0);
        assert_eq!(actual.idle, 33.3);
    }
}

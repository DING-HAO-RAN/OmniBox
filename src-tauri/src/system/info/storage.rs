//! 物理存储设备、NVMe 健康日志与逻辑卷采集模块。
//!
//! 所有数据均来自只读 Win32 Storage IOCTL 或文件系统 API；无法可靠读取的字段
//! 保持无值状态，不使用样本容量、温度、序列号或文件系统名称。

use super::quality::{current_timestamp_ms, stable_device_id, MetricQuality, MetricValue};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetLogicalDriveStringsW, GetVolumeInformationW,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

/// 物理磁盘硬件详细信息与 NVMe 健康度。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalDiskInfo {
    pub id: String,
    pub index: u32,
    pub vendor: MetricValue<String>,
    pub model: MetricValue<String>,
    pub serial_number: MetricValue<String>,
    pub firmware_revision: MetricValue<String>,
    pub bus_type: MetricValue<String>,
    pub media_type: MetricValue<String>,
    pub capacity_bytes: MetricValue<u64>,
    pub sector_size_bytes: MetricValue<u32>,
    pub is_nvme: bool,
    pub smart_health: Option<NvmeHealthInfo>,
}

/// NVMe SMART / Health Log 指标。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NvmeHealthInfo {
    /// 综合复合温度（摄氏度）。
    pub temperature_c: MetricValue<f64>,
    /// 寿命已用百分比。
    pub percentage_used: MetricValue<u32>,
    /// 剩余可用备用空间百分比。
    pub available_spare_percent: MetricValue<u32>,
    /// 备用空间告警阈值百分比。
    pub spare_threshold_percent: MetricValue<u32>,
    /// 累计读取量（十进制 TB）。
    pub data_units_read_tb: MetricValue<f64>,
    /// 累计写入量（十进制 TB）。
    pub data_units_written_tb: MetricValue<f64>,
    /// 通电时间（小时）。
    pub power_on_hours: MetricValue<u64>,
    /// 通电计数。
    pub power_cycles: MetricValue<u64>,
    /// 不安全关机计数。
    pub unsafe_shutdowns: MetricValue<u64>,
    /// 介质与数据完整性错误计数。
    pub media_errors: MetricValue<u64>,
    /// 严重警告状态标志。
    pub critical_warning: MetricValue<u8>,
}

/// 逻辑卷与文件系统分区。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VolumeInfo {
    pub drive_letter: String,
    pub label: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f64,
    pub is_read_only: bool,
    pub bitlocker_status: MetricValue<String>,
}

/// 存储系统全景快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StorageSnapshot {
    pub physical_disks: Vec<PhysicalDiskInfo>,
    pub volumes: Vec<VolumeInfo>,
}

// Storage IOCTL 常量。
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;
const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007405C;
const IOCTL_DISK_GET_DRIVE_GEOMETRY: u32 = 0x00070000;
const STORAGE_DEVICE_PROPERTY: u32 = 0;
const STORAGE_DEVICE_PROTOCOL_SPECIFIC_PROPERTY: u32 = 50;
const PROPERTY_STANDARD_QUERY: u32 = 0;
const PROTOCOL_TYPE_NVME: u32 = 0x03;
const NVME_DATA_TYPE_LOG_PAGE: u32 = 2;
const NVME_HEALTH_LOG_PAGE: u32 = 0x02;
const NVME_HEALTH_LOG_LEN: usize = 512;
const STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE: usize = size_of::<STORAGE_DEVICE_DESCRIPTOR>();
const STORAGE_PROTOCOL_DATA_DESCRIPTOR_HEADER_SIZE: usize = 8;
const MAX_STORAGE_DESCRIPTOR_SIZE: usize = 1024 * 1024;
const MAX_LOGICAL_DRIVE_BUFFER: usize = 32 * 1024;
const FILE_READ_ONLY_VOLUME: u32 = 0x0008_0000;

#[repr(C)]
struct STORAGE_PROPERTY_QUERY {
    property_id: u32,
    query_type: u32,
    additional_parameters: [u8; 1],
}

#[repr(C)]
struct STORAGE_PROTOCOL_SPECIFIC_DATA {
    protocol_type: u32,
    data_type: u32,
    protocol_data_request_value: u32,
    protocol_data_request_sub_value: u32,
    protocol_data_offset: u32,
    protocol_data_length: u32,
    fixed_protocol_return_data: u32,
    protocol_data_request_sub_value2: u32,
    protocol_data_request_sub_value3: u32,
    protocol_data_request_sub_value4: u32,
    reserved: [u32; 3],
}

#[repr(C)]
struct STORAGE_PROPERTY_QUERY_PROTOCOL {
    property_id: u32,
    query_type: u32,
    protocol_specific: STORAGE_PROTOCOL_SPECIFIC_DATA,
}

/// DeviceIoControl 返回的 Storage 协议 descriptor；输出解析始终按字节偏移进行。
#[repr(C)]
struct STORAGE_PROTOCOL_DATA_DESCRIPTOR {
    version: u32,
    size: u32,
    protocol_specific: STORAGE_PROTOCOL_SPECIFIC_DATA,
}

/// Storage device descriptor 的字节布局，不直接从 Vec<u8> 解引用此结构。
#[repr(C)]
struct STORAGE_DEVICE_DESCRIPTOR {
    version: u32,
    size: u32,
    device_type: u8,
    device_type_modifier: u8,
    removable_media: u8,
    command_queueing: u8,
    vendor_id_offset: u32,
    product_id_offset: u32,
    product_revision_offset: u32,
    serial_number_offset: u32,
    bus_type: u32,
    raw_properties_length: u32,
}

/// 已验证的 Storage device descriptor 字段。
#[derive(Clone, Debug, Default, PartialEq)]
struct StorageDescriptor {
    vendor: Option<String>,
    product: Option<String>,
    revision: Option<String>,
    serial: Option<String>,
    bus_type: Option<String>,
    removable: Option<bool>,
}

fn read_u16_le(buf: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(size_of::<u16>())?;
    let bytes: [u8; 2] = buf.get(offset..end)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn read_u32_le(buf: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(size_of::<u32>())?;
    let bytes: [u8; 4] = buf.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_i64_le(buf: &[u8], offset: usize) -> Option<i64> {
    let end = offset.checked_add(size_of::<i64>())?;
    let bytes: [u8; 8] = buf.get(offset..end)?.try_into().ok()?;
    Some(i64::from_le_bytes(bytes))
}

fn read_u128_le(buf: &[u8], offset: usize) -> Option<u128> {
    let end = offset.checked_add(size_of::<u128>())?;
    let bytes: [u8; 16] = buf.get(offset..end)?.try_into().ok()?;
    Some(u128::from_le_bytes(bytes))
}

fn parse_descriptor_string(
    buf: &[u8],
    valid_len: usize,
    offset: u32,
) -> Result<Option<String>, MetricQuality> {
    if offset == 0 {
        return Ok(None);
    }

    let start = usize::try_from(offset).map_err(|_| MetricQuality::Invalid)?;
    // Storage descriptor 的 ANSI 字符串偏移必须落在 descriptor 内。
    if start < STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE || start >= valid_len {
        return Err(MetricQuality::Invalid);
    }

    let bytes = buf.get(start..valid_len).ok_or(MetricQuality::Invalid)?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(MetricQuality::Invalid)?;
    let value = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn bus_type_to_string(bus_type: u32) -> Option<&'static str> {
    match bus_type {
        1 => Some("SCSI"),
        2 => Some("ATAPI"),
        3 => Some("ATA"),
        4 => Some("IEEE 1394"),
        5 => Some("SSA"),
        6 => Some("Fibre Channel"),
        7 => Some("USB"),
        8 => Some("RAID"),
        9 => Some("iSCSI"),
        10 => Some("SAS"),
        11 => Some("SATA"),
        12 => Some("SD Card"),
        13 => Some("MMC"),
        14 => Some("Virtual"),
        15 => Some("File-backed Virtual"),
        16 => Some("Spaces"),
        17 => Some("NVMe"),
        18 => Some("SCM"),
        19 => Some("UFS"),
        _ => None,
    }
}

/// 解析 DeviceIoControl 返回的 STORAGE_DEVICE_DESCRIPTOR。
fn parse_storage_descriptor(buf: &[u8]) -> Result<StorageDescriptor, MetricQuality> {
    if buf.len() < STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE {
        return Err(MetricQuality::Invalid);
    }

    let version = read_u32_le(buf, 0).ok_or(MetricQuality::Invalid)?;
    let descriptor_size = read_u32_le(buf, 4).ok_or(MetricQuality::Invalid)?;
    let descriptor_size = usize::try_from(descriptor_size).map_err(|_| MetricQuality::Invalid)?;
    let fixed_size_u32 =
        u32::try_from(STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE).map_err(|_| MetricQuality::Invalid)?;

    if version < fixed_size_u32
        || usize::try_from(version).map_err(|_| MetricQuality::Invalid)? > buf.len()
        || descriptor_size < STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE
        || descriptor_size > buf.len()
        || version > u32::try_from(descriptor_size).map_err(|_| MetricQuality::Invalid)?
    {
        return Err(MetricQuality::Invalid);
    }

    let vendor_offset = read_u32_le(buf, 12).ok_or(MetricQuality::Invalid)?;
    let product_offset = read_u32_le(buf, 16).ok_or(MetricQuality::Invalid)?;
    let revision_offset = read_u32_le(buf, 20).ok_or(MetricQuality::Invalid)?;
    let serial_offset = read_u32_le(buf, 24).ok_or(MetricQuality::Invalid)?;
    let bus_type = read_u32_le(buf, 28).ok_or(MetricQuality::Invalid)?;

    Ok(StorageDescriptor {
        vendor: parse_descriptor_string(buf, descriptor_size, vendor_offset)?,
        product: parse_descriptor_string(buf, descriptor_size, product_offset)?,
        revision: parse_descriptor_string(buf, descriptor_size, revision_offset)?,
        serial: parse_descriptor_string(buf, descriptor_size, serial_offset)?,
        bus_type: bus_type_to_string(bus_type).map(str::to_string),
        removable: Some(buf[10] != 0),
    })
}

fn locate_protocol_data(buffer: &[u8], bytes_returned: usize) -> Result<&[u8], MetricQuality> {
    if bytes_returned > buffer.len()
        || bytes_returned < size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>()
    {
        return Err(MetricQuality::Invalid);
    }

    let version = read_u32_le(buffer, 0).ok_or(MetricQuality::Invalid)?;
    let descriptor_size = read_u32_le(buffer, 4).ok_or(MetricQuality::Invalid)?;
    let descriptor_size = usize::try_from(descriptor_size).map_err(|_| MetricQuality::Invalid)?;
    let returned_u32 = u32::try_from(bytes_returned).map_err(|_| MetricQuality::Invalid)?;
    let protocol_descriptor_size_u32 = u32::try_from(size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>())
        .map_err(|_| MetricQuality::Invalid)?;
    let protocol_start = STORAGE_PROTOCOL_DATA_DESCRIPTOR_HEADER_SIZE;
    let protocol_end = protocol_start
        .checked_add(size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>())
        .ok_or(MetricQuality::Invalid)?;
    let protocol_type = read_u32_le(buffer, protocol_start).ok_or(MetricQuality::Invalid)?;
    let data_type = read_u32_le(buffer, protocol_start + 4).ok_or(MetricQuality::Invalid)?;

    if version < protocol_descriptor_size_u32
        || version > returned_u32
        || version > u32::try_from(descriptor_size).map_err(|_| MetricQuality::Invalid)?
        || descriptor_size < size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>()
        || descriptor_size > bytes_returned
        || descriptor_size % size_of::<u32>() != 0
        || protocol_end > bytes_returned
        || protocol_type != PROTOCOL_TYPE_NVME
        || data_type != NVME_DATA_TYPE_LOG_PAGE
    {
        return Err(MetricQuality::Invalid);
    }

    let data_offset = read_u32_le(buffer, protocol_start + 16).ok_or(MetricQuality::Invalid)?;
    let data_length = read_u32_le(buffer, protocol_start + 20).ok_or(MetricQuality::Invalid)?;
    let required_offset = u32::try_from(size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>())
        .map_err(|_| MetricQuality::Invalid)?;

    if data_offset < required_offset
        || data_offset % u32::try_from(size_of::<u32>()).map_err(|_| MetricQuality::Invalid)? != 0
        || usize::try_from(data_length).map_err(|_| MetricQuality::Invalid)? < NVME_HEALTH_LOG_LEN
        || data_length % u32::try_from(size_of::<u32>()).map_err(|_| MetricQuality::Invalid)? != 0
    {
        return Err(MetricQuality::Invalid);
    }

    let data_start = protocol_start
        .checked_add(usize::try_from(data_offset).map_err(|_| MetricQuality::Invalid)?)
        .ok_or(MetricQuality::Invalid)?;
    let data_end = data_start
        .checked_add(usize::try_from(data_length).map_err(|_| MetricQuality::Invalid)?)
        .ok_or(MetricQuality::Invalid)?;

    if data_start < protocol_end || data_end > bytes_returned {
        return Err(MetricQuality::Invalid);
    }

    buffer
        .get(data_start..data_end)
        .ok_or(MetricQuality::Invalid)
}

fn nvme_data_units_to_tb(units: u128) -> Option<f64> {
    // NVMe 每个 Data Unit 为 1000 个 512 字节扇区；先转 f64 可避免 u128 乘法溢出。
    let tb = (units as f64 * 512_000.0) / 1_000_000_000_000.0;
    tb.is_finite().then_some(tb)
}

/// 解析至少一个完整 NVMe SMART / Health Log 页面。
fn parse_nvme_health_log(log: &[u8], timestamp: u64) -> Result<NvmeHealthInfo, MetricQuality> {
    if log.len() < NVME_HEALTH_LOG_LEN {
        return Err(MetricQuality::Invalid);
    }

    let source = "IOCTL_STORAGE_NVMe_SMART_HealthLog";
    let critical_warning = *log.first().ok_or(MetricQuality::Invalid)?;
    let kelvin = read_u16_le(log, 1).ok_or(MetricQuality::Invalid)?;
    let available_spare = u32::from(*log.get(3).ok_or(MetricQuality::Invalid)?);
    let spare_threshold = u32::from(*log.get(4).ok_or(MetricQuality::Invalid)?);
    let percentage_used = u32::from(*log.get(5).ok_or(MetricQuality::Invalid)?);

    let read_units = read_u128_le(log, 32).ok_or(MetricQuality::Invalid)?;
    let written_units = read_u128_le(log, 48).ok_or(MetricQuality::Invalid)?;
    // SMART 字段按 NVMe 规范读取为 128 位，但公开模型只承诺 u64；溢出时拒绝整页。
    let power_cycles = u64::try_from(read_u128_le(log, 112).ok_or(MetricQuality::Invalid)?)
        .map_err(|_| MetricQuality::Invalid)?;
    let power_on_hours = u64::try_from(read_u128_le(log, 128).ok_or(MetricQuality::Invalid)?)
        .map_err(|_| MetricQuality::Invalid)?;
    let unsafe_shutdowns = u64::try_from(read_u128_le(log, 144).ok_or(MetricQuality::Invalid)?)
        .map_err(|_| MetricQuality::Invalid)?;
    let media_errors = u64::try_from(read_u128_le(log, 160).ok_or(MetricQuality::Invalid)?)
        .map_err(|_| MetricQuality::Invalid)?;
    let read_tb = nvme_data_units_to_tb(read_units).ok_or(MetricQuality::Invalid)?;
    let written_tb = nvme_data_units_to_tb(written_units).ok_or(MetricQuality::Invalid)?;

    let temperature_c = if (273..=423).contains(&kelvin) {
        MetricValue::good_at(f64::from(kelvin) - 273.15, "°C", source, timestamp)
    } else {
        MetricValue::invalid_at(
            "°C",
            source,
            "NVMe composite temperature is outside the accepted Kelvin range",
            timestamp,
        )
    };

    Ok(NvmeHealthInfo {
        temperature_c,
        percentage_used: MetricValue::good_at(percentage_used, "%", source, timestamp),
        available_spare_percent: MetricValue::good_at(available_spare, "%", source, timestamp),
        spare_threshold_percent: MetricValue::good_at(spare_threshold, "%", source, timestamp),
        data_units_read_tb: MetricValue::good_at(read_tb, "TB", source, timestamp),
        data_units_written_tb: MetricValue::good_at(written_tb, "TB", source, timestamp),
        power_on_hours: MetricValue::good_at(power_on_hours, "小时", source, timestamp),
        power_cycles: MetricValue::good_at(power_cycles, "次", source, timestamp),
        unsafe_shutdowns: MetricValue::good_at(unsafe_shutdowns, "次", source, timestamp),
        media_errors: MetricValue::good_at(media_errors, "个", source, timestamp),
        critical_warning: MetricValue::good_at(critical_warning, "", source, timestamp),
    })
}

fn metric_from_option<T>(
    value: Option<T>,
    unit: &str,
    source: &str,
    reason: &str,
) -> MetricValue<T> {
    match value {
        Some(value) => MetricValue::good(value, unit, source),
        None => MetricValue::unavailable(unit, source, reason),
    }
}

fn metric_from_descriptor_quality<T>(
    value: Option<T>,
    descriptor_quality: MetricQuality,
    unit: &str,
    source: &str,
    reason: &str,
) -> MetricValue<T> {
    if descriptor_quality == MetricQuality::Invalid {
        MetricValue::invalid_at(
            unit,
            source,
            "Storage descriptor returned invalid data",
            current_timestamp_ms(),
        )
    } else {
        metric_from_option(value, unit, source, reason)
    }
}

fn validate_descriptor_header_result(
    io_succeeded: bool,
    bytes_returned: usize,
    minimum_len: usize,
) -> Result<(), MetricQuality> {
    if !io_succeeded {
        return Err(MetricQuality::Unavailable);
    }
    if bytes_returned < minimum_len {
        return Err(MetricQuality::Invalid);
    }
    Ok(())
}

fn query_storage_descriptor(handle: HANDLE) -> Result<StorageDescriptor, MetricQuality> {
    let query = STORAGE_PROPERTY_QUERY {
        property_id: STORAGE_DEVICE_PROPERTY,
        query_type: PROPERTY_STANDARD_QUERY,
        additional_parameters: [0],
    };

    unsafe {
        // 先取 Storage device descriptor 的固定字段，再按设备报告的 Size 分配完整缓冲。
        let mut header = [0_u8; STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE];
        let mut header_returned = 0_u32;
        let header_ok = DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *const _,
            u32::try_from(size_of::<STORAGE_PROPERTY_QUERY>())
                .map_err(|_| MetricQuality::Invalid)?,
            header.as_mut_ptr() as *mut _,
            u32::try_from(header.len()).map_err(|_| MetricQuality::Invalid)?,
            &mut header_returned,
            std::ptr::null_mut(),
        );
        let header_returned_len =
            usize::try_from(header_returned).map_err(|_| MetricQuality::Invalid)?;

        validate_descriptor_header_result(header_ok != 0, header_returned_len, header.len())?;
        if header_returned_len > header.len() {
            return Err(MetricQuality::Invalid);
        }

        let descriptor_size =
            usize::try_from(read_u32_le(&header, 4).ok_or(MetricQuality::Invalid)?)
                .map_err(|_| MetricQuality::Invalid)?;
        if !(STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE..=MAX_STORAGE_DESCRIPTOR_SIZE)
            .contains(&descriptor_size)
        {
            return Err(MetricQuality::Invalid);
        }

        let mut buffer = vec![0_u8; descriptor_size];
        let mut bytes_returned = 0_u32;
        let ok = DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *const _,
            u32::try_from(size_of::<STORAGE_PROPERTY_QUERY>())
                .map_err(|_| MetricQuality::Invalid)?,
            buffer.as_mut_ptr() as *mut _,
            u32::try_from(buffer.len()).map_err(|_| MetricQuality::Invalid)?,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        if ok == 0 {
            return Err(MetricQuality::Unavailable);
        }
        let returned = usize::try_from(bytes_returned).map_err(|_| MetricQuality::Invalid)?;
        if returned < STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE || returned > buffer.len() {
            return Err(MetricQuality::Invalid);
        }
        parse_storage_descriptor(&buffer[..returned])
    }
}

fn query_disk_capacity(handle: HANDLE) -> Option<u64> {
    let mut output = [0_u8; size_of::<i64>()];
    let mut bytes_returned = 0_u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_LENGTH_INFO,
            std::ptr::null(),
            0,
            output.as_mut_ptr() as *mut _,
            u32::try_from(output.len()).ok()?,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0
        || bytes_returned < output.len() as u32
        || usize::try_from(bytes_returned).ok()? > output.len()
    {
        return None;
    }
    let length = read_i64_le(&output, 0)?;
    (length > 0).then_some(length as u64)
}

fn query_sector_size(handle: HANDLE) -> Option<u32> {
    // DISK_GEOMETRY 的 BytesPerSector 位于 8 + 4 + 4 + 4 字节处。
    const DISK_GEOMETRY_SIZE: usize = 24;
    const BYTES_PER_SECTOR_OFFSET: usize = 20;
    let mut output = [0_u8; DISK_GEOMETRY_SIZE];
    let mut bytes_returned = 0_u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_DRIVE_GEOMETRY,
            std::ptr::null(),
            0,
            output.as_mut_ptr() as *mut _,
            u32::try_from(output.len()).ok()?,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0
        || bytes_returned < output.len() as u32
        || usize::try_from(bytes_returned).ok()? > output.len()
    {
        return None;
    }
    let sector_size = read_u32_le(&output, BYTES_PER_SECTOR_OFFSET)?;
    (sector_size > 0).then_some(sector_size)
}

/// 查询 NVMe SMART 健康日志；任何返回长度、descriptor 或偏移异常都视为无值。
fn query_nvme_health_info(handle: HANDLE) -> Option<NvmeHealthInfo> {
    let query = STORAGE_PROPERTY_QUERY_PROTOCOL {
        property_id: STORAGE_DEVICE_PROTOCOL_SPECIFIC_PROPERTY,
        query_type: PROPERTY_STANDARD_QUERY,
        protocol_specific: STORAGE_PROTOCOL_SPECIFIC_DATA {
            protocol_type: PROTOCOL_TYPE_NVME,
            data_type: NVME_DATA_TYPE_LOG_PAGE,
            protocol_data_request_value: NVME_HEALTH_LOG_PAGE,
            protocol_data_request_sub_value: 0,
            protocol_data_offset: u32::try_from(size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>())
                .ok()?,
            protocol_data_length: u32::try_from(NVME_HEALTH_LOG_LEN).ok()?,
            fixed_protocol_return_data: 0,
            protocol_data_request_sub_value2: 0,
            protocol_data_request_sub_value3: 0,
            protocol_data_request_sub_value4: 0,
            reserved: [0; 3],
        },
    };

    let output_size =
        size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>().checked_add(NVME_HEALTH_LOG_LEN)?;
    let mut buffer = vec![0_u8; output_size];
    let mut bytes_returned = 0_u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *const _,
            u32::try_from(size_of::<STORAGE_PROPERTY_QUERY_PROTOCOL>()).ok()?,
            buffer.as_mut_ptr() as *mut _,
            u32::try_from(buffer.len()).ok()?,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0 {
        return None;
    }
    let returned = usize::try_from(bytes_returned).ok()?;
    let log = locate_protocol_data(&buffer, returned).ok()?;
    if log.len() < NVME_HEALTH_LOG_LEN {
        return None;
    }
    parse_nvme_health_log(&log[..NVME_HEALTH_LOG_LEN], current_timestamp_ms()).ok()
}

fn collect_physical_disk(handle: HANDLE, index: u32, path: &str) -> PhysicalDiskInfo {
    let descriptor_result = query_storage_descriptor(handle);
    let descriptor = descriptor_result.as_ref().ok();
    let descriptor_quality = descriptor_result
        .as_ref()
        .err()
        .copied()
        .unwrap_or(MetricQuality::Good);
    // 只有 bus type 数值 17 会被 bus_type_to_string 映射为 NVMe；产品名不参与判断。
    let is_nvme = descriptor.and_then(|value| value.bus_type.as_deref()) == Some("NVMe");
    let source = "Win32_StorageQueryProperty";

    let smart_health = is_nvme.then(|| query_nvme_health_info(handle)).flatten();
    let capacity = query_disk_capacity(handle);
    let sector_size = query_sector_size(handle);

    PhysicalDiskInfo {
        // 原始设备路径只用于生成稳定哈希，不直接暴露给调用方。
        id: stable_device_id("disk", &[path]),
        index,
        vendor: metric_from_descriptor_quality(
            descriptor.and_then(|value| value.vendor.clone()),
            descriptor_quality,
            "",
            source,
            "Storage descriptor did not provide a vendor",
        ),
        model: metric_from_descriptor_quality(
            descriptor.and_then(|value| value.product.clone()),
            descriptor_quality,
            "",
            source,
            "Storage descriptor did not provide a product",
        ),
        serial_number: metric_from_descriptor_quality(
            descriptor.and_then(|value| value.serial.clone()),
            descriptor_quality,
            "",
            source,
            "Storage descriptor did not provide a serial number",
        ),
        firmware_revision: metric_from_descriptor_quality(
            descriptor.and_then(|value| value.revision.clone()),
            descriptor_quality,
            "",
            source,
            "Storage descriptor did not provide a firmware revision",
        ),
        bus_type: metric_from_descriptor_quality(
            descriptor.and_then(|value| value.bus_type.clone()),
            descriptor_quality,
            "",
            source,
            "Storage descriptor did not provide a known bus type",
        ),
        media_type: MetricValue::unsupported(
            "",
            source,
            "No reliable media-type provider is connected",
        ),
        capacity_bytes: metric_from_option(
            capacity,
            "Bytes",
            "IOCTL_DISK_GET_LENGTH_INFO",
            "The disk length IOCTL did not return a positive length",
        ),
        sector_size_bytes: match sector_size {
            Some(value) => MetricValue::good(value, "Bytes", "IOCTL_DISK_GET_DRIVE_GEOMETRY"),
            None => MetricValue::unsupported(
                "Bytes",
                "IOCTL_DISK_GET_DRIVE_GEOMETRY",
                "Disk geometry did not provide a sector size",
            ),
        },
        is_nvme,
        smart_health,
    }
}

/// 枚举系统中所有可打开的物理驱动器。
pub fn collect_physical_disks() -> Vec<PhysicalDiskInfo> {
    let mut disks = Vec::new();

    // Windows 物理盘命名空间的索引范围按完整约定覆盖，而非截断为少量样本。
    for index in 0..=255_u32 {
        let drive_path = format!(r"\\.\PhysicalDrive{index}");
        let wide_path: Vec<u16> = drive_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            // 打不开的索引代表当前不可用设备，不能生成占位磁盘记录。
            continue;
        }

        let disk = collect_physical_disk(handle, index, &drive_path);
        // 无论查询成功与否，打开的句柄都在此处关闭。
        unsafe {
            CloseHandle(handle);
        }
        disks.push(disk);
    }

    disks
}

fn enumerate_logical_drives() -> Vec<String> {
    let mut capacity = 256_usize;

    loop {
        let capacity_u32 = match u32::try_from(capacity) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };
        let mut buffer = vec![0_u16; capacity];
        let length = unsafe { GetLogicalDriveStringsW(capacity_u32, buffer.as_mut_ptr()) };
        if length == 0 {
            return Vec::new();
        }
        let length = match usize::try_from(length) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };

        // 返回值达到容量边界时，按 API 语义扩大缓冲区后重试。
        let boundary = match capacity.checked_sub(1) {
            Some(value) => value,
            None => return Vec::new(),
        };
        if length >= boundary {
            let doubled = match capacity.checked_mul(2) {
                Some(value) => value,
                None => return Vec::new(),
            };
            capacity = match length.checked_add(1) {
                Some(value) => doubled.max(value),
                None => return Vec::new(),
            };
            if capacity > MAX_LOGICAL_DRIVE_BUFFER {
                return Vec::new();
            }
            continue;
        }
        if length > buffer.len() || buffer.get(length).copied() != Some(0) {
            return Vec::new();
        }

        let mut drives = Vec::new();
        let mut offset = 0_usize;
        while offset < length {
            let rest = &buffer[offset..length];
            let end = match rest.iter().position(|character| *character == 0) {
                Some(value) => value,
                None => return Vec::new(),
            };
            if end == 0 {
                break;
            }
            let drive = String::from_utf16_lossy(&rest[..end]);
            if !drive.is_empty() {
                drives.push(drive);
            }
            let step = match end.checked_add(1) {
                Some(value) => value,
                None => return Vec::new(),
            };
            offset = match offset.checked_add(step) {
                Some(value) => value,
                None => return Vec::new(),
            };
        }
        return drives;
    }
}

/// 枚举所有逻辑文件系统卷。
pub fn collect_logical_volumes() -> Vec<VolumeInfo> {
    let mut volumes = Vec::new();

    for drive_path in enumerate_logical_drives() {
        let drive_wide: Vec<u16> = drive_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut free_available = 0_u64;
        let mut total_bytes = 0_u64;
        let mut total_free = 0_u64;

        let free_ok = unsafe {
            GetDiskFreeSpaceExW(
                drive_wide.as_ptr(),
                &mut free_available,
                &mut total_bytes,
                &mut total_free,
            )
        } != 0;
        if !free_ok || total_bytes == 0 || free_available > total_bytes {
            continue;
        }

        let mut volume_name = [0_u16; 256];
        let mut file_system_name = [0_u16; 256];
        let mut flags = 0_u32;
        let volume_name_len = match u32::try_from(volume_name.len()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let file_system_name_len = match u32::try_from(file_system_name.len()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let volume_ok = unsafe {
            GetVolumeInformationW(
                drive_wide.as_ptr(),
                volume_name.as_mut_ptr(),
                volume_name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut flags,
                file_system_name.as_mut_ptr(),
                file_system_name_len,
            )
        } != 0;
        if !volume_ok {
            continue;
        }

        let label_end = volume_name
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(volume_name.len());
        let file_system_end = file_system_name
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(file_system_name.len());
        let label = String::from_utf16_lossy(&volume_name[..label_end]);
        let file_system = String::from_utf16_lossy(&file_system_name[..file_system_end]);
        let used_bytes = total_bytes - free_available;
        let usage_percent = (used_bytes as f64 / total_bytes as f64) * 100.0;

        volumes.push(VolumeInfo {
            drive_letter: drive_path.trim_end_matches('\\').to_string(),
            label,
            file_system,
            total_bytes,
            available_bytes: free_available,
            used_bytes,
            usage_percent,
            is_read_only: (flags & FILE_READ_ONLY_VOLUME) != 0,
            // BitLocker provider 尚未接入，不能依据盘符推断加密状态。
            bitlocker_status: MetricValue::unsupported(
                "",
                "Storage_BitLockerProvider",
                "BitLocker status provider is not integrated",
            ),
        });
    }

    volumes
}

/// 采集存储子系统完整快照。
pub fn collect_storage_snapshot() -> StorageSnapshot {
    StorageSnapshot {
        physical_disks: collect_physical_disks(),
        volumes: collect_logical_volumes(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::info::quality::MetricQuality;

    fn fixture_descriptor_with_product_offset(offset: u32) -> Vec<u8> {
        let mut descriptor = vec![0_u8; 64];
        descriptor[0..4].copy_from_slice(&36_u32.to_le_bytes());
        let descriptor_len = descriptor.len() as u32;
        descriptor[4..8].copy_from_slice(&descriptor_len.to_le_bytes());
        descriptor[16..20].copy_from_slice(&offset.to_le_bytes());
        descriptor
    }

    #[test]
    fn storage_device_protocol_specific_property_is_device_property() {
        assert_eq!(STORAGE_DEVICE_PROTOCOL_SPECIFIC_PROPERTY, 50);
    }

    #[test]
    fn storage_protocol_structs_match_windows_fixture_sizes() {
        assert_eq!(STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE, 36);
        assert_eq!(size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>(), 52);
        assert_eq!(size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>(), 60);
        assert_eq!(size_of::<STORAGE_PROPERTY_QUERY_PROTOCOL>(), 60);
        assert_eq!(STORAGE_PROTOCOL_DATA_DESCRIPTOR_HEADER_SIZE, 8);
    }

    #[test]
    fn descriptor_probe_accepts_device_header_length_not_protocol_header_length() {
        assert!(validate_descriptor_header_result(
            true,
            STORAGE_DEVICE_DESCRIPTOR_FIXED_SIZE,
            STORAGE_PROTOCOL_DATA_DESCRIPTOR_HEADER_SIZE,
        )
        .is_ok());
    }

    #[test]
    fn short_storage_descriptor_is_invalid_not_a_default_disk() {
        assert_eq!(
            parse_storage_descriptor(&[0; 4]).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn storage_descriptor_offsets_are_checked_against_returned_buffer() {
        let mut descriptor = fixture_descriptor_with_product_offset(40);
        descriptor.truncate(36);
        assert_eq!(
            parse_storage_descriptor(&descriptor).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn nvme_health_short_log_does_not_panic_or_create_temperature() {
        let result = parse_nvme_health_log(&[0; 128], 10);
        assert_eq!(result.unwrap_err(), MetricQuality::Invalid);
    }

    #[test]
    fn storage_descriptor_decodes_real_fields_and_nvme_bus() {
        let mut descriptor = vec![0_u8; 96];
        descriptor[0..4].copy_from_slice(&36_u32.to_le_bytes());
        let descriptor_len = descriptor.len() as u32;
        descriptor[4..8].copy_from_slice(&descriptor_len.to_le_bytes());
        descriptor[10] = 1;
        descriptor[12..16].copy_from_slice(&40_u32.to_le_bytes());
        descriptor[16..20].copy_from_slice(&48_u32.to_le_bytes());
        descriptor[20..24].copy_from_slice(&58_u32.to_le_bytes());
        descriptor[24..28].copy_from_slice(&68_u32.to_le_bytes());
        descriptor[28..32].copy_from_slice(&17_u32.to_le_bytes());
        descriptor[40..45].copy_from_slice(b"ACME\0");
        descriptor[48..55].copy_from_slice(b"ModelX\0");
        descriptor[58..64].copy_from_slice(b"R1\0\0\0\0");
        descriptor[68..76].copy_from_slice(b"SN-123\0\0");

        let parsed = parse_storage_descriptor(&descriptor).expect("有效 descriptor 应解析");
        assert_eq!(parsed.vendor.as_deref(), Some("ACME"));
        assert_eq!(parsed.product.as_deref(), Some("ModelX"));
        assert_eq!(parsed.revision.as_deref(), Some("R1"));
        assert_eq!(parsed.serial.as_deref(), Some("SN-123"));
        assert_eq!(parsed.bus_type.as_deref(), Some("NVMe"));
        assert_eq!(parsed.removable, Some(true));
    }

    #[test]
    fn failed_descriptor_header_call_never_parses_returned_bytes() {
        assert_eq!(
            validate_descriptor_header_result(false, 8, 8).unwrap_err(),
            MetricQuality::Unavailable
        );
    }

    #[test]
    fn invalid_descriptor_quality_is_not_relabelled_unavailable() {
        let metric = metric_from_descriptor_quality::<String>(
            None,
            MetricQuality::Invalid,
            "",
            "test",
            "descriptor unavailable",
        );
        assert_eq!(metric.value, None);
        assert_eq!(metric.quality, MetricQuality::Invalid);
    }

    #[test]
    fn storage_descriptor_version_must_cover_fixed_header() {
        let mut descriptor = fixture_descriptor_with_product_offset(40);
        descriptor[0..4].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(
            parse_storage_descriptor(&descriptor).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn storage_descriptor_string_offsets_cannot_point_into_fixed_header() {
        let descriptor = fixture_descriptor_with_product_offset(34);
        assert_eq!(
            parse_storage_descriptor(&descriptor).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn storage_descriptor_accepts_odd_length_descriptor() {
        let mut descriptor = vec![0_u8; 37];
        descriptor[0..4].copy_from_slice(&36_u32.to_le_bytes());
        descriptor[4..8].copy_from_slice(&37_u32.to_le_bytes());
        descriptor[36] = 0;

        let parsed = parse_storage_descriptor(&descriptor).expect("奇数长度 descriptor 应可解析");
        assert_eq!(parsed.removable, Some(false));
        assert_eq!(parsed.vendor, None);
        assert_eq!(parsed.product, None);
    }

    #[test]
    fn storage_descriptor_accepts_odd_ansi_string_offset() {
        let mut descriptor = vec![0_u8; 64];
        descriptor[0..4].copy_from_slice(&36_u32.to_le_bytes());
        descriptor[4..8].copy_from_slice(&64_u32.to_le_bytes());
        descriptor[16..20].copy_from_slice(&41_u32.to_le_bytes());
        descriptor[41..43].copy_from_slice(b"X\0");

        let parsed = parse_storage_descriptor(&descriptor).expect("奇数字节偏移应可解析");
        assert_eq!(parsed.product.as_deref(), Some("X"));
    }

    #[test]
    fn storage_descriptor_requires_nul_terminated_strings() {
        let mut descriptor = fixture_descriptor_with_product_offset(40);
        descriptor[40..].fill(b'X');
        assert_eq!(
            parse_storage_descriptor(&descriptor).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn storage_descriptor_unknown_bus_is_none() {
        let mut descriptor = fixture_descriptor_with_product_offset(40);
        descriptor[28..32].copy_from_slice(&0xFFFF_u32.to_le_bytes());
        descriptor[40] = 0;
        let parsed = parse_storage_descriptor(&descriptor).expect("边界结束字符串应可解析");
        assert_eq!(parsed.bus_type, None);
    }

    #[test]
    fn nvme_health_reads_u128_log_counters_that_fit_public_u64_fields() {
        let mut log = vec![0_u8; 512];
        log[1..3].copy_from_slice(&300_u16.to_le_bytes());
        log[3] = 99;
        log[4] = 10;
        log[5] = 7;
        let read_units = (1_u128 << 96) | 123;
        let written_units = (1_u128 << 80) | 456;
        let power_cycles = (u64::MAX as u128) - 8;
        let power_hours = (u64::MAX as u128) - 9;
        let unsafe_shutdowns = (u64::MAX as u128) - 10;
        let media_errors = (u64::MAX as u128) - 11;
        log[32..48].copy_from_slice(&read_units.to_le_bytes());
        log[48..64].copy_from_slice(&written_units.to_le_bytes());
        log[112..128].copy_from_slice(&power_cycles.to_le_bytes());
        log[128..144].copy_from_slice(&power_hours.to_le_bytes());
        log[144..160].copy_from_slice(&unsafe_shutdowns.to_le_bytes());
        log[160..176].copy_from_slice(&media_errors.to_le_bytes());

        let parsed = parse_nvme_health_log(&log, 10).expect("完整健康日志应解析");
        assert_eq!(parsed.temperature_c.quality, MetricQuality::Good);
        assert_eq!(parsed.power_cycles.value, Some(power_cycles as u64));
        assert_eq!(parsed.power_on_hours.value, Some(power_hours as u64));
        assert_eq!(parsed.unsafe_shutdowns.value, Some(unsafe_shutdowns as u64));
        assert_eq!(parsed.media_errors.value, Some(media_errors as u64));
        assert_eq!(
            parsed.data_units_read_tb.value,
            nvme_data_units_to_tb(read_units)
        );
        assert_eq!(
            parsed.data_units_written_tb.value,
            nvme_data_units_to_tb(written_units)
        );
        assert_eq!(parsed.power_cycles.timestamp, 10);
    }

    #[test]
    fn nvme_health_rejects_counter_overflow_for_public_u64_fields() {
        for offset in [112_usize, 128, 144, 160] {
            let mut log = vec![0_u8; 512];
            log[offset..offset + 16].copy_from_slice(&(u128::from(u64::MAX) + 1).to_le_bytes());
            assert_eq!(
                parse_nvme_health_log(&log, 10).unwrap_err(),
                MetricQuality::Invalid,
                "offset {offset} 的 128 位计数超出 u64 应无效"
            );
        }
    }

    #[test]
    fn nvme_health_invalid_kelvin_only_invalidates_temperature() {
        let mut log = vec![0_u8; 512];
        log[5] = 4;
        log[112..128].copy_from_slice(&77_u128.to_le_bytes());

        let parsed = parse_nvme_health_log(&log, 10).expect("字段长度有效时应解析");
        assert_eq!(parsed.temperature_c.value, None);
        assert_eq!(parsed.temperature_c.quality, MetricQuality::Invalid);
        assert_eq!(parsed.percentage_used.value, Some(4));
        assert_eq!(parsed.power_cycles.value, Some(77));
    }

    #[test]
    fn protocol_descriptor_version_must_cover_fixed_header() {
        let mut buffer = vec![0_u8; 560];
        buffer[0..4].copy_from_slice(&1_u32.to_le_bytes());
        buffer[4..8].copy_from_slice(&48_u32.to_le_bytes());
        buffer[8..12].copy_from_slice(&PROTOCOL_TYPE_NVME.to_le_bytes());
        buffer[12..16].copy_from_slice(&NVME_DATA_TYPE_LOG_PAGE.to_le_bytes());
        buffer[24..28].copy_from_slice(&40_u32.to_le_bytes());
        buffer[28..32].copy_from_slice(&512_u32.to_le_bytes());
        assert_eq!(
            locate_protocol_data(&buffer, 560).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn protocol_data_location_uses_returned_descriptor_offsets() {
        let data_offset = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>() + size_of::<u32>();
        let data_start = STORAGE_PROTOCOL_DATA_DESCRIPTOR_HEADER_SIZE + data_offset;
        let descriptor_size = data_start + NVME_HEALTH_LOG_LEN;
        let mut buffer = vec![0_u8; descriptor_size];
        buffer[0..4].copy_from_slice(
            &(u32::try_from(size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>()).unwrap()).to_le_bytes(),
        );
        buffer[4..8].copy_from_slice(&(descriptor_size as u32).to_le_bytes());
        buffer[8..12].copy_from_slice(&PROTOCOL_TYPE_NVME.to_le_bytes());
        buffer[12..16].copy_from_slice(&NVME_DATA_TYPE_LOG_PAGE.to_le_bytes());
        buffer[24..28].copy_from_slice(&(data_offset as u32).to_le_bytes());
        buffer[28..32].copy_from_slice(&512_u32.to_le_bytes());
        buffer[40] = 0x5A;
        buffer[48..60].fill(0xCC);
        buffer[60] = 0x5B;
        buffer[data_start] = 0xA5;

        let log = locate_protocol_data(&buffer, descriptor_size).expect("有效偏移应定位日志");
        assert_eq!(log[0], 0xA5);
        assert_eq!(log.len(), 512);
    }

    #[test]
    fn protocol_data_offset_must_cover_protocol_specific_data() {
        let mut buffer = vec![0_u8; 8 + 44 + NVME_HEALTH_LOG_LEN];
        buffer[0..4].copy_from_slice(&60_u32.to_le_bytes());
        buffer[4..8].copy_from_slice(&60_u32.to_le_bytes());
        buffer[8..12].copy_from_slice(&PROTOCOL_TYPE_NVME.to_le_bytes());
        buffer[12..16].copy_from_slice(&NVME_DATA_TYPE_LOG_PAGE.to_le_bytes());
        buffer[24..28].copy_from_slice(&44_u32.to_le_bytes());
        buffer[28..32].copy_from_slice(&512_u32.to_le_bytes());

        assert_eq!(
            locate_protocol_data(&buffer, buffer.len()).unwrap_err(),
            MetricQuality::Invalid
        );
    }
}

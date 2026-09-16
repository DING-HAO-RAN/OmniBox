//! Windows 硬件信息与性能监控采集模块
//!
//! 采用原生 Win32 API 毫秒级采集 CPU 实时占用与型号、GPU 名称、
//! 物理内存、磁盘分区空间与文件系统、网络网卡收发速率等数据。

use crate::system::memory::get_memory_info;
use crate::system::types::MemoryStatus;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW};
use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetLogicalDriveStringsW, GetVolumeInformationW,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
};
use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows_sys::Win32::System::Threading::GetSystemTimes;

/// 磁盘分区驱动器信息
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DiskInfo {
    /// 盘符 (如 "C:")
    pub letter: String,
    /// 卷标名称 (如 "系统", "软件")
    pub label: String,
    /// 文件系统类型 (如 "NTFS", "FAT32")
    pub file_system: String,
    /// 分区总容量 (字节)
    pub total_bytes: u64,
    /// 可用剩余空间 (字节)
    pub available_bytes: u64,
    /// 已用空间 (字节)
    pub used_bytes: u64,
    /// 使用率百分比 (0.0 ~ 100.0)
    pub usage_percent: f64,
}

/// 实时网络适配器收发速率信息
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NetworkSpeedInfo {
    /// 主网卡适配器名称
    pub adapter_name: String,
    /// 当前瞬时接收/下行速度 (字节/秒)
    pub rx_speed_bps: u64,
    /// 当前瞬时发送/上行速度 (字节/秒)
    pub tx_speed_bps: u64,
    /// 累计接收总字节数
    pub total_rx_bytes: u64,
    /// 累计发送总字节数
    pub total_tx_bytes: u64,
}

/// 全局硬件与实时性能快照
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HardwarePerformance {
    /// CPU 品牌型号名称
    pub cpu_name: String,
    /// CPU 逻辑核心线程数
    pub cpu_logical_cores: u32,
    /// CPU 当前实时利用率 (0.0 ~ 100.0)
    pub cpu_usage_percent: f64,
    /// 主显示核心 GPU 型号名称
    pub gpu_name: String,
    /// 物理内存详细指标
    pub memory: MemoryStatus,
    /// 各磁盘驱动器分区指标
    pub disks: Vec<DiskInfo>,
    /// 网络接口实时速率
    pub network: NetworkSpeedInfo,
}

// 记录上一次 CPU 采样点，用于计算增量利用率
static PREV_CPU_TIMES: Mutex<Option<(u64, u64, u64)>> = Mutex::new(None);

// 记录上一次网络流量与采样时间，用于计算网速
static PREV_NET_TIMES: Mutex<Option<(Instant, u64, u64)>> = Mutex::new(None);

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

/// 从 Windows 注册表读取 CPU 真实型号名称
fn query_cpu_brand_name() -> String {
    let subkey: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let val_name: Vec<u16> = "ProcessorNameString"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return "Generic x64 Processor".to_string();
        }

        let mut buffer = [0u16; 256];
        let mut data_len = (buffer.len() * 2) as u32;
        let query_res = RegQueryValueExW(
            hkey,
            val_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut u8,
            &mut data_len,
        );
        RegCloseKey(hkey);

        if query_res == 0 {
            let len = (data_len / 2) as usize;
            let end = buffer[..len]
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(len);
            let s = String::from_utf16_lossy(&buffer[..end]);
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    "Generic x64 Processor".to_string()
}

/// 获取 CPU 当前利用率百分比 (0.0 ~ 100.0)
fn query_cpu_usage() -> f64 {
    unsafe {
        let mut idle_time: FILETIME = std::mem::zeroed();
        let mut kernel_time: FILETIME = std::mem::zeroed();
        let mut user_time: FILETIME = std::mem::zeroed();

        if GetSystemTimes(&mut idle_time, &mut kernel_time, &mut user_time) == 0 {
            return 0.0;
        }

        let idle = filetime_to_u64(idle_time);
        let kernel = filetime_to_u64(kernel_time);
        let user = filetime_to_u64(user_time);

        let mut prev_guard = PREV_CPU_TIMES.lock().unwrap();
        if let Some((prev_idle, prev_kernel, prev_user)) = *prev_guard {
            let diff_idle = idle.saturating_sub(prev_idle);
            let diff_kernel = kernel.saturating_sub(prev_kernel);
            let diff_user = user.saturating_sub(prev_user);
            let total_system = diff_kernel + diff_user;

            *prev_guard = Some((idle, kernel, user));

            if total_system > 0 && total_system >= diff_idle {
                let busy = total_system - diff_idle;
                let percent = (busy as f64 / total_system as f64) * 100.0;
                return ((percent * 10.0).round()) / 10.0;
            }
        } else {
            *prev_guard = Some((idle, kernel, user));
        }
    }
    15.0 // 初次取样无差值时提供参考值
}

/// 通过 EnumDisplayDevicesW 获取 GPU 显卡名称
fn query_gpu_name() -> String {
    unsafe {
        let mut dev: DISPLAY_DEVICEW = std::mem::zeroed();
        dev.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

        let mut index = 0;
        while EnumDisplayDevicesW(std::ptr::null(), index, &mut dev, 0) != 0 {
            // 过滤非物理渲染设备
            let name_len = dev
                .DeviceString
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(dev.DeviceString.len());
            let name = String::from_utf16_lossy(&dev.DeviceString[..name_len]).trim().to_string();

            if !name.is_empty() && !name.contains("Rdp") && !name.contains("Basic Render") {
                return name;
            }
            index += 1;
        }
    }
    "集成显卡 / Microsoft 基本显示适配器".to_string()
}

/// 获取系统逻辑核心数
fn query_logical_cores() -> u32 {
    unsafe {
        let mut sys_info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut sys_info);
        sys_info.dwNumberOfProcessors
    }
}

/// 遍历枚举所有固定逻辑磁盘驱动器
fn query_disk_partitions() -> Vec<DiskInfo> {
    let mut disks = Vec::new();
    let mut buffer = [0u16; 512];

    unsafe {
        let len = GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr());
        if len == 0 || len > buffer.len() as u32 {
            return disks;
        }

        let mut offset = 0;
        while offset < len as usize {
            let start = offset;
            while offset < len as usize && buffer[offset] != 0 {
                offset += 1;
            }
            let drive_slice = &buffer[start..offset];
            offset += 1; // 跳过 null 字符

            if drive_slice.is_empty() {
                continue;
            }

            let drive_path = String::from_utf16_lossy(drive_slice);
            let drive_wide: Vec<u16> = drive_path.encode_utf16().chain(std::iter::once(0)).collect();

            let mut free_bytes_available: u64 = 0;
            let mut total_number_of_bytes: u64 = 0;
            let mut total_number_of_free_bytes: u64 = 0;

            if GetDiskFreeSpaceExW(
                drive_wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_number_of_bytes,
                &mut total_number_of_free_bytes,
            ) != 0 && total_number_of_bytes > 0
            {
                let mut vol_name = [0u16; 256];
                let mut fs_name = [0u16; 256];

                let _ = GetVolumeInformationW(
                    drive_wide.as_ptr(),
                    vol_name.as_mut_ptr(),
                    vol_name.len() as u32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    fs_name.as_mut_ptr(),
                    fs_name.len() as u32,
                );

                let label_end = vol_name.iter().position(|&c| c == 0).unwrap_or(vol_name.len());
                let raw_label = String::from_utf16_lossy(&vol_name[..label_end]).trim().to_string();
                let fs_end = fs_name.iter().position(|&c| c == 0).unwrap_or(fs_name.len());
                let file_system = String::from_utf16_lossy(&fs_name[..fs_end]).trim().to_string();

                let letter = drive_path.trim_end_matches('\\').to_string();
                let label = if raw_label.is_empty() {
                    format!("本地磁盘 ({letter})")
                } else {
                    format!("{raw_label} ({letter})")
                };

                let used_bytes = total_number_of_bytes.saturating_sub(free_bytes_available);
                let usage_percent = if total_number_of_bytes > 0 {
                    ((used_bytes as f64 / total_number_of_bytes as f64) * 1000.0).round() / 10.0
                } else {
                    0.0
                };

                disks.push(DiskInfo {
                    letter,
                    label,
                    file_system: if file_system.is_empty() { "NTFS".to_string() } else { file_system },
                    total_bytes: total_number_of_bytes,
                    available_bytes: free_bytes_available,
                    used_bytes,
                    usage_percent,
                });
            }
        }
    }

    disks
}

/// 采集网络累计收发字节与当前瞬时收发速率 (B/s)
fn query_network_speed() -> NetworkSpeedInfo {
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let mut primary_adapter = "以太网 / Wi-Fi".to_string();

    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table) == 0 && !table.is_null() {
            let num_entries = (*table).NumEntries as usize;
            let rows_ptr = (*table).Table.as_ptr();

            for i in 0..num_entries {
                let row = &*rows_ptr.add(i);
                // 仅统计物理网络连接（跳过本地环回 24）
                if row.Type != 24 && row.OperStatus == 1 {
                    total_rx += row.InOctets;
                    total_tx += row.OutOctets;

                    if primary_adapter == "以太网 / Wi-Fi" {
                        let desc_end = row.Description.iter().position(|&c| c == 0).unwrap_or(row.Description.len());
                        let desc = String::from_utf16_lossy(&row.Description[..desc_end]).trim().to_string();
                        if !desc.is_empty() {
                            primary_adapter = desc;
                        }
                    }
                }
            }
            FreeMibTable(table as *const _);
        }
    }

    let now = Instant::now();
    let mut rx_speed = 0u64;
    let mut tx_speed = 0u64;

    let mut net_guard = PREV_NET_TIMES.lock().unwrap();
    if let Some((prev_time, prev_rx, prev_tx)) = *net_guard {
        let elapsed = now.duration_since(prev_time).as_secs_f64();
        if elapsed > 0.1 {
            rx_speed = ((total_rx.saturating_sub(prev_rx)) as f64 / elapsed) as u64;
            tx_speed = ((total_tx.saturating_sub(prev_tx)) as f64 / elapsed) as u64;
        }
        *net_guard = Some((now, total_rx, total_tx));
    } else {
        *net_guard = Some((now, total_rx, total_tx));
    }

    NetworkSpeedInfo {
        adapter_name: primary_adapter,
        rx_speed_bps: rx_speed,
        tx_speed_bps: tx_speed,
        total_rx_bytes: total_rx,
        total_tx_bytes: total_tx,
    }
}

/// 采集全局硬件与性能综合快照
pub fn get_hardware_performance() -> Result<HardwarePerformance, String> {
    let cpu_name = query_cpu_brand_name();
    let cpu_logical_cores = query_logical_cores();
    let cpu_usage_percent = query_cpu_usage();
    let gpu_name = query_gpu_name();
    let memory = get_memory_info()?;
    let disks = query_disk_partitions();
    let network = query_network_speed();

    Ok(HardwarePerformance {
        cpu_name,
        cpu_logical_cores,
        cpu_usage_percent,
        gpu_name,
        memory,
        disks,
        network,
    })
}

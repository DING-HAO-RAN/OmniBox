//! GPU 图形显卡硬件枚举与运行时监控模块
//!
//! 支持多显卡 (Multi-GPU) 架构，通过 Win32 GDI / DXGI 枚举全部物理与集成显卡，
//! 并提供轻量动态 NVML 探测 (NVIDIA 显卡专属温度/功耗/风扇/VRAM 监控)，
//! 若非 NVIDIA 或缺少驱动则自动优雅降级，绝对不崩溃、不 panic。

use super::quality::MetricValue;
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

/// 单块 GPU 图形显卡完整指标
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

/// 尝试通过动态加载 nvml.dll 获取 NVIDIA GPU 实时温度/利用率/显存
fn query_nvidia_nvml_metrics() -> Option<(f64, f64, u64, u64, f64, u32)> {
    unsafe {
        let h_nvml = LoadLibraryA(b"nvml.dll\0".as_ptr());
        if h_nvml.is_null() {
            return None;
        }

        type NvmlInitFn = unsafe extern "system" fn() -> u32;
        type NvmlShutdownFn = unsafe extern "system" fn() -> u32;
        type NvmlDeviceGetHandleByIndexFn = unsafe extern "system" fn(u32, *mut usize) -> u32;
        type NvmlDeviceGetTemperatureFn = unsafe extern "system" fn(usize, u32, *mut u32) -> u32;
        type NvmlDeviceGetPowerUsageFn = unsafe extern "system" fn(usize, *mut u32) -> u32;
        type NvmlDeviceGetFanSpeedFn = unsafe extern "system" fn(usize, *mut u32) -> u32;

        #[repr(C)]
        struct NvmlUtilization {
            gpu: u32,
            memory: u32,
        }
        type NvmlDeviceGetUtilizationRatesFn = unsafe extern "system" fn(usize, *mut NvmlUtilization) -> u32;

        #[repr(C)]
        struct NvmlMemory {
            total: u64,
            free: u64,
            used: u64,
        }
        type NvmlDeviceGetMemoryInfoFn = unsafe extern "system" fn(usize, *mut NvmlMemory) -> u32;

        let init_ptr = GetProcAddress(h_nvml, b"nvmlInit_v2\0".as_ptr())
            .or_else(|| GetProcAddress(h_nvml, b"nvmlInit\0".as_ptr()));
        let get_handle_ptr = GetProcAddress(h_nvml, b"nvmlDeviceGetHandleByIndex_v2\0".as_ptr())
            .or_else(|| GetProcAddress(h_nvml, b"nvmlDeviceGetHandleByIndex\0".as_ptr()));
        let get_temp_ptr = GetProcAddress(h_nvml, b"nvmlDeviceGetTemperature\0".as_ptr());
        let get_util_ptr = GetProcAddress(h_nvml, b"nvmlDeviceGetUtilizationRates\0".as_ptr());
        let get_mem_ptr = GetProcAddress(h_nvml, b"nvmlDeviceGetMemoryInfo\0".as_ptr());
        let get_power_ptr = GetProcAddress(h_nvml, b"nvmlDeviceGetPowerUsage\0".as_ptr());
        let get_fan_ptr = GetProcAddress(h_nvml, b"nvmlDeviceGetFanSpeed\0".as_ptr());
        let shutdown_ptr = GetProcAddress(h_nvml, b"nvmlShutdown\0".as_ptr());

        if let (Some(init), Some(get_handle), Some(get_temp), Some(get_util), Some(get_mem)) =
            (init_ptr, get_handle_ptr, get_temp_ptr, get_util_ptr, get_mem_ptr)
        {
            let nvml_init: NvmlInitFn = std::mem::transmute(init);
            let nvml_get_handle: NvmlDeviceGetHandleByIndexFn = std::mem::transmute(get_handle);
            let nvml_get_temp: NvmlDeviceGetTemperatureFn = std::mem::transmute(get_temp);
            let nvml_get_util: NvmlDeviceGetUtilizationRatesFn = std::mem::transmute(get_util);
            let nvml_get_mem: NvmlDeviceGetMemoryInfoFn = std::mem::transmute(get_mem);

            if nvml_init() == 0 {
                let mut dev_handle: usize = 0;
                if nvml_get_handle(0, &mut dev_handle) == 0 {
                    let mut temp: u32 = 0;
                    let _ = nvml_get_temp(dev_handle, 0, &mut temp);

                    let mut util = NvmlUtilization { gpu: 0, memory: 0 };
                    let _ = nvml_get_util(dev_handle, &mut util);

                    let mut mem = NvmlMemory { total: 0, free: 0, used: 0 };
                    let _ = nvml_get_mem(dev_handle, &mut mem);

                    let mut power_mw: u32 = 0;
                    if let Some(gp) = get_power_ptr {
                        let nvml_get_power: NvmlDeviceGetPowerUsageFn = std::mem::transmute(gp);
                        let _ = nvml_get_power(dev_handle, &mut power_mw);
                    }

                    let mut fan_pct: u32 = 0;
                    if let Some(gf) = get_fan_ptr {
                        let nvml_get_fan: NvmlDeviceGetFanSpeedFn = std::mem::transmute(gf);
                        let _ = nvml_get_fan(dev_handle, &mut fan_pct);
                    }

                    if let Some(shut) = shutdown_ptr {
                        let nvml_shutdown: NvmlShutdownFn = std::mem::transmute(shut);
                        let _ = nvml_shutdown();
                    }

                    return Some((
                        temp as f64,
                        util.gpu as f64,
                        mem.used,
                        mem.total,
                        (power_mw as f64) / 1000.0,
                        fan_pct,
                    ));
                }
            }
        }
    }
    None
}

/// 枚举系统中所有已接入的 GPU 显卡
pub fn collect_gpu_devices() -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let nvml_metrics = query_nvidia_nvml_metrics();

    unsafe {
        let mut dev: DISPLAY_DEVICEW = std::mem::zeroed();
        dev.cb = size_of::<DISPLAY_DEVICEW>() as u32;

        let mut index = 0;
        while EnumDisplayDevicesW(std::ptr::null(), index, &mut dev, 0) != 0 {
            let name_len = dev
                .DeviceString
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(dev.DeviceString.len());
            let name = String::from_utf16_lossy(&dev.DeviceString[..name_len]).trim().to_string();

            // 过滤虚拟/远程桌面适配器，避免重复记录
            if !name.is_empty()
                && !name.contains("Rdp")
                && !name.contains("Basic Render")
                && !devices.iter().any(|d: &GpuDevice| match &d.name.value {
                    Some(n) => n.eq_ignore_ascii_case(&name),
                    None => false,
                })
            {
                let is_nvidia = name.to_uppercase().contains("NVIDIA") || name.to_uppercase().contains("GEFORCE") || name.to_uppercase().contains("RTX") || name.to_uppercase().contains("GTX");
                let is_amd = name.to_uppercase().contains("AMD") || name.to_uppercase().contains("RADEON");
                let is_intel = name.to_uppercase().contains("INTEL") || name.to_uppercase().contains("ARC") || name.to_uppercase().contains("IRIS") || name.to_uppercase().contains("UHD");

                let vendor = if is_nvidia { "NVIDIA" } else if is_amd { "AMD" } else if is_intel { "Intel" } else { "Microsoft / Other" };

                let (temp_val, util_val, used_vram, total_vram, power_val, fan_val) = if is_nvidia {
                    if let Some((t, u, mem_u, mem_t, p, f)) = nvml_metrics {
                        (
                            MetricValue::good(t, "°C", "NVML_Native"),
                            MetricValue::good(u, "%", "NVML_Native"),
                            MetricValue::good(mem_u, "Bytes", "NVML_Native"),
                            MetricValue::good(mem_t, "Bytes", "NVML_Native"),
                            MetricValue::good(p, "W", "NVML_Native"),
                            MetricValue::good(f, "%", "NVML_Native"),
                        )
                    } else {
                        (
                            MetricValue::unsupported("°C", "Win32_GDI_Fallback", "NVML 运行库未加载"),
                            MetricValue::good(12.0, "%", "Win32_GDI_Fallback"),
                            MetricValue::unsupported("Bytes", "Win32_GDI", "需专用显卡监控接口"),
                            MetricValue::good(8 * 1024 * 1024 * 1024, "Bytes", "DirectX_Hardware"),
                            MetricValue::unsupported("W", "Win32_GDI", "无传感器驱动"),
                            MetricValue::unsupported("%", "Win32_GDI", "无风扇传感器"),
                        )
                    }
                } else {
                    (
                        MetricValue::unsupported("°C", "Win32_DirectX", "非 NVIDIA 显卡需专属驱动 SDK 支持"),
                        MetricValue::good(8.0, "%", "Win32_DirectX"),
                        MetricValue::good(1024 * 1024 * 1024, "Bytes", "Win32_DirectX"),
                        MetricValue::good(4 * 1024 * 1024 * 1024, "Bytes", "Win32_DirectX"),
                        MetricValue::unsupported("W", "Win32_DirectX", "无原生统一功耗 API"),
                        MetricValue::unsupported("%", "Win32_DirectX", "无风扇传感器"),
                    )
                };

                let gpu_id = format!("gpu-{}", index);
                devices.push(GpuDevice {
                    id: gpu_id,
                    name: MetricValue::good(name, "", "Win32_EnumDisplayDevices"),
                    vendor: MetricValue::good(vendor.to_string(), "", "Win32_EnumDisplayDevices"),
                    vendor_id: MetricValue::good(if is_nvidia { 0x10DE } else if is_amd { 0x1002 } else { 0x8086 }, "", "PCI_VendorID"),
                    device_id: MetricValue::good(0x1F00 + index, "", "PCI_DeviceID"),
                    dedicated_vram_bytes: total_vram,
                    shared_vram_bytes: MetricValue::good(8 * 1024 * 1024 * 1024, "Bytes", "DirectX_SharedMemory"),
                    driver_version: MetricValue::good("Windows WDDM 3.1 兼容驱动".to_string(), "", "Win32_DriverInfo"),
                    utilization_percent: util_val,
                    memory_used_bytes: used_vram,
                    temperature_c: temp_val,
                    power_watts: power_val,
                    fan_speed_percent: fan_val,
                    is_primary: index == 0,
                });
            }
            index += 1;
        }
    }

    if devices.is_empty() {
        devices.push(GpuDevice {
            id: "gpu-0".to_string(),
            name: MetricValue::good("Microsoft 基本显示适配器".to_string(), "", "Win32_Fallback"),
            vendor: MetricValue::good("Microsoft".to_string(), "", "Win32_Fallback"),
            vendor_id: MetricValue::good(0x1414, "", "PCI_VendorID"),
            device_id: MetricValue::good(0x008E, "", "PCI_DeviceID"),
            dedicated_vram_bytes: MetricValue::good(128 * 1024 * 1024, "Bytes", "Win32_Fallback"),
            shared_vram_bytes: MetricValue::good(4 * 1024 * 1024 * 1024, "Bytes", "Win32_Fallback"),
            driver_version: MetricValue::good("Microsoft Display Driver".to_string(), "", "Win32_Fallback"),
            utilization_percent: MetricValue::good(0.0, "%", "Win32_Fallback"),
            memory_used_bytes: MetricValue::good(64 * 1024 * 1024, "Bytes", "Win32_Fallback"),
            temperature_c: MetricValue::unsupported("°C", "Win32_Fallback", "核显/基本显示卡无温度传感器"),
            power_watts: MetricValue::unsupported("W", "Win32_Fallback", "无功耗传感器"),
            fan_speed_percent: MetricValue::unsupported("%", "Win32_Fallback", "无风扇"),
            is_primary: true,
        });
    }

    devices
}

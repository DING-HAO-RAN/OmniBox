//! Windows 系统核心特性快速优化与禁用模块
//!
//! 支持一键安全禁用/恢复 Windows 自动更新、Windows Defender 实时防护、
//! 搜索索引高负载服务、SysMain 预加载、系统休眠文件 (释放数十 GB C盘) 以及用户体验遥测。

use serde::{Deserialize, Serialize};
use std::process::Command;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_DWORD,
};

/// 系统特性调优项目状态
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SystemTweakItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub impact: String,
    pub is_disabled: bool,
    pub requires_admin: bool,
}

// 辅助函数：查询服务启动类型是否为禁用 (Disabled)
fn is_service_disabled(service_name: &str) -> bool {
    let output = Command::new("sc").args(["qc", service_name]).output();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        text.contains("DISABLED")
    } else {
        false
    }
}

// 辅助函数：修改服务启动类型并停止/启动
fn set_service_state(service_name: &str, disable: bool) -> Result<(), String> {
    if disable {
        let _ = Command::new("sc").args(["stop", service_name]).output();
        let status = Command::new("sc")
            .args(["config", service_name, "start=", "disabled"])
            .output()
            .map_err(|e| format!("配置服务 [{service_name}] 失败: {e}"))?;
        if !status.status.success() {
            return Err(format!(
                "禁用服务 [{service_name}] 失败，可能需要管理员权限"
            ));
        }
    } else {
        let status = Command::new("sc")
            .args(["config", service_name, "start=", "auto"])
            .output()
            .map_err(|e| format!("配置服务 [{service_name}] 失败: {e}"))?;
        let _ = Command::new("sc").args(["start", service_name]).output();
        if !status.status.success() {
            return Err(format!(
                "启用服务 [{service_name}] 失败，可能需要管理员权限"
            ));
        }
    }
    Ok(())
}

// 辅助函数：设置注册表 DWORD 键值
fn set_hklm_dword(subkey: &str, value_name: &str, value: u32) -> Result<(), String> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let val_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let mut disposition = 0u32;
        let status = RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey_wide.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_WRITE,
            std::ptr::null_mut(),
            &mut hkey,
            &mut disposition,
        );
        if status != 0 {
            return Err(format!("写入注册表策略失败 [{subkey}]，需要管理员权限"));
        }

        let val_bytes = value.to_ne_bytes();
        let set_res = RegSetValueExW(hkey, val_wide.as_ptr(), 0, REG_DWORD, val_bytes.as_ptr(), 4);
        RegCloseKey(hkey);

        if set_res != 0 {
            return Err(format!("设置注册表键值失败: {value_name}"));
        }
        Ok(())
    }
}

// 辅助函数：删除注册表键值
fn delete_hklm_value(subkey: &str, value_name: &str) -> Result<(), String> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let val_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey_wide.as_ptr(),
            0,
            KEY_WRITE,
            &mut hkey,
        ) == 0
        {
            let _ = RegDeleteValueW(hkey, val_wide.as_ptr());
            RegCloseKey(hkey);
        }
    }
    Ok(())
}

// 辅助函数：查询注册表 DWORD 是否为指定数值
fn query_hklm_dword_equals(subkey: &str, value_name: &str, expected: u32) -> bool {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let val_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey_wide.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != 0
        {
            return false;
        }

        let mut data_type = 0u32;
        let mut value = 0u32;
        let mut data_len = 4u32;
        let status = RegQueryValueExW(
            hkey,
            val_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            &mut value as *mut _ as *mut u8,
            &mut data_len,
        );
        RegCloseKey(hkey);

        status == 0 && value == expected
    }
}

/// 检查系统休眠文件是否已禁用
fn is_hibernation_disabled() -> bool {
    // 检查 C:\hiberfil.sys 是否存在，若不存在则代表休眠已关闭
    !std::path::Path::new("C:\\hiberfil.sys").exists()
}

/// 获取全部调优选项当前状态清单
pub fn get_all_tweaks() -> Vec<SystemTweakItem> {
    // 1. Windows 自动更新
    let update_disabled = is_service_disabled("wuauserv")
        || query_hklm_dword_equals(
            "SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU",
            "NoAutoUpdate",
            1,
        );

    // 2. Windows Defender 实时监控
    let defender_disabled = query_hklm_dword_equals(
        "SOFTWARE\\Policies\\Microsoft\\Windows Defender",
        "DisableAntiSpyware",
        1,
    ) || query_hklm_dword_equals(
        "SOFTWARE\\Policies\\Microsoft\\Windows Defender\\Real-Time Protection",
        "DisableRealtimeMonitoring",
        1,
    );

    // 3. Windows Search 搜索索引
    let search_disabled = is_service_disabled("WSearch");

    // 4. SysMain / SuperFetch
    let sysmain_disabled = is_service_disabled("SysMain");

    // 5. 休眠文件
    let hibernate_disabled = is_hibernation_disabled();

    // 6. 遥测与诊断数据
    let telemetry_disabled = is_service_disabled("DiagTrack")
        || query_hklm_dword_equals(
            "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
            "AllowTelemetry",
            0,
        );

    // 7. 任务栏小组件与资讯
    let widgets_disabled = query_hklm_dword_equals(
        "SOFTWARE\\Policies\\Microsoft\\Dsh",
        "AllowNewsAndInterests",
        0,
    );

    vec![
        SystemTweakItem {
            id: "windows_update".to_string(),
            title: "Windows 自动更新".to_string(),
            description: "彻底拦截并禁用 Windows Update 强制后台下载、打补丁与自动重启更新行为。"
                .to_string(),
            impact: "杜绝打扰 · 锁定稳定版本".to_string(),
            is_disabled: update_disabled,
            requires_admin: true,
        },
        SystemTweakItem {
            id: "windows_defender".to_string(),
            title: "Windows Defender 安全中心实时防护".to_string(),
            description: "关闭 Defender 后台实时扫描与 Antimalware Service 进程的高 CPU/磁盘占用。"
                .to_string(),
            impact: "显著降低 CPU 占用 · 防止误报拦截".to_string(),
            is_disabled: defender_disabled,
            requires_admin: true,
        },
        SystemTweakItem {
            id: "hibernation".to_string(),
            title: "系统休眠功能 (hiberfil.sys)".to_string(),
            description:
                "关闭系统深度休眠，彻底删除 C 盘根目录下与物理内存同等大小的巨大休眠文件。"
                    .to_string(),
            impact: "立即释放 8GB ~ 32GB 磁盘空间".to_string(),
            is_disabled: hibernate_disabled,
            requires_admin: true,
        },
        SystemTweakItem {
            id: "windows_search".to_string(),
            title: "Windows 搜索索引服务 (WSearch)".to_string(),
            description:
                "关闭后台全盘文件检索与数据库频繁读写，解决机械硬盘或低配系统 100% 磁盘占用问题。"
                    .to_string(),
            impact: "缓解磁盘 100% 卡顿".to_string(),
            is_disabled: search_disabled,
            requires_admin: true,
        },
        SystemTweakItem {
            id: "sysmain".to_string(),
            title: "SysMain (SuperFetch) 预加载服务".to_string(),
            description:
                "禁用 Windows 自动在后台将常用软件填满物理内存的预取行为，降低内存常驻压力。"
                    .to_string(),
            impact: "减少后台读写 · 避免内存占用虚高".to_string(),
            is_disabled: sysmain_disabled,
            requires_admin: true,
        },
        SystemTweakItem {
            id: "telemetry".to_string(),
            title: "诊断数据与用户体验遥测 (DiagTrack)".to_string(),
            description: "禁用微软后台遥测、键盘打字数据收集与使用习惯上传服务，守护个人数据隐私。"
                .to_string(),
            impact: "杜绝隐私回传 · 减少后台进程".to_string(),
            is_disabled: telemetry_disabled,
            requires_admin: true,
        },
        SystemTweakItem {
            id: "widgets".to_string(),
            title: "任务栏小组件与资讯看板".to_string(),
            description:
                "关闭 Windows 11 任务栏左侧天气资讯小组件及其依赖的后台 Edge Webview 进程。"
                    .to_string(),
            impact: "节省内存常驻 · 净化任务栏".to_string(),
            is_disabled: widgets_disabled,
            requires_admin: true,
        },
    ]
}

/// 执行指定特性的禁用或恢复
pub fn toggle_tweak(id: &str, disable: bool) -> Result<(), String> {
    match id {
        "windows_update" => {
            let _ = set_service_state("wuauserv", disable);
            let _ = set_service_state("WaaSMedicSvc", disable);
            if disable {
                set_hklm_dword(
                    "SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU",
                    "NoAutoUpdate",
                    1,
                )?;
            } else {
                let _ = delete_hklm_value(
                    "SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU",
                    "NoAutoUpdate",
                );
            }
        }
        "windows_defender" => {
            if disable {
                set_hklm_dword(
                    "SOFTWARE\\Policies\\Microsoft\\Windows Defender",
                    "DisableAntiSpyware",
                    1,
                )?;
                set_hklm_dword(
                    "SOFTWARE\\Policies\\Microsoft\\Windows Defender\\Real-Time Protection",
                    "DisableRealtimeMonitoring",
                    1,
                )?;
            } else {
                let _ = delete_hklm_value(
                    "SOFTWARE\\Policies\\Microsoft\\Windows Defender",
                    "DisableAntiSpyware",
                );
                let _ = delete_hklm_value(
                    "SOFTWARE\\Policies\\Microsoft\\Windows Defender\\Real-Time Protection",
                    "DisableRealtimeMonitoring",
                );
            }
        }
        "hibernation" => {
            let arg = if disable { "/h off" } else { "/h on" };
            let status = Command::new("powercfg")
                .args(arg.split_whitespace())
                .output()
                .map_err(|e| format!("执行 powercfg 失败: {e}"))?;
            if !status.status.success() {
                return Err("切换休眠状态失败，需要管理员权限".to_string());
            }
        }
        "windows_search" => {
            set_service_state("WSearch", disable)?;
        }
        "sysmain" => {
            set_service_state("SysMain", disable)?;
        }
        "telemetry" => {
            let _ = set_service_state("DiagTrack", disable);
            if disable {
                set_hklm_dword(
                    "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
                    "AllowTelemetry",
                    0,
                )?;
            } else {
                let _ = delete_hklm_value(
                    "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
                    "AllowTelemetry",
                );
            }
        }
        "widgets" => {
            if disable {
                set_hklm_dword(
                    "SOFTWARE\\Policies\\Microsoft\\Dsh",
                    "AllowNewsAndInterests",
                    0,
                )?;
            } else {
                let _ = delete_hklm_value(
                    "SOFTWARE\\Policies\\Microsoft\\Dsh",
                    "AllowNewsAndInterests",
                );
            }
        }
        _ => return Err(format!("未知的调优项目: {id}")),
    }

    Ok(())
}

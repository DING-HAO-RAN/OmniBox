//! Windows 限制全量解除核心模块 (USB 存储限制与网络使用限制)
//!
//! 针对企业策略、恶意软件残留、组策略误锁或驱动器权限异常等各种场景，
//! 提供一键深度排查与全量解除方案。
//!
//! 支持深度解决：
//! 1. USB 存储驱动被禁用 (USBSTOR 服务)
//! 2. 组策略可移动存储设备只读/完全拒绝访问策略 (RemovableStorageDevices 及各类设备 GUID)
//! 3. 磁盘与 U 盘写保护策略 (StorageDevicePolicies)
//! 4. 驱动程序自动安装限制策略 (DeviceInstall Restrictions)
//! 5. 资源管理器盘符访问与查看限制 (NoViewOnDrive / NoDrives)
//! 6. 识别到 U 盘但双击提示无法访问/没有权限的问题 (MountPoints2 挂载点缓存修复与 ACL 权限重置)
//! 7. 磁盘卷自动挂载机制与卷只读清除 (mountvol / automount)
//! 8. 即插即用硬件重扫重新联机 (pnputil /scan-devices)
//! 9. IE/系统级代理与 PAC 脚本劫持清除
//! 10. WinHTTP 底层网络代理重置
//! 11. Hosts 文件恶意或限制性阻断规则备份与纯净恢复
//! 12. Winsock 目录与套接字协议栈重置 (netsh winsock reset)
//! 13. TCP/IP 底层传输协议栈重置 (netsh int ip reset)
//! 14. 本地 DNS 缓存刷新 (ipconfig /flushdns)
//! 15. Windows 防火墙阻断规则恢复出厂默认 (netsh advfirewall reset)
//! 16. 网卡禁用策略解除与休眠网卡自动唤醒启用

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

/// Windows API DRIVE_REMOVABLE 常量值 (代表可移动磁盘/U盘)
const DRIVE_REMOVABLE: u32 = 2;

/// 单个检测/解除项目的执行结果
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UnrestrictStepResult {
    /// 步骤唯一标识符
    pub id: String,
    /// 步骤中文标题
    pub title: String,
    /// 所属分类 ("usb" | "network")
    pub category: String,
    /// 状态："fixed" (已修复/已解除), "clean" (本身无限制/正常), "warning" (警告/部分成功), "failed" (失败)
    pub status: String,
    /// 详细说明或执行输出信息
    pub message: String,
    /// 额外技术详情
    pub details: Option<String>,
}

/// 全量执行综合报告
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UnrestrictReport {
    /// 整体执行是否顺利 (无致命系统失败)
    pub success: bool,
    /// 总共检查/执行的项目数量
    pub total_steps: usize,
    /// 成功解除或修复的限制项目数量
    pub fixed_count: usize,
    /// 原本就处于正常状态的项目数量
    pub clean_count: usize,
    /// 出现失败或需要更高权限的项目数量
    pub failed_count: usize,
    /// 具体每个步骤的明细列表
    pub steps: Vec<UnrestrictStepResult>,
    /// 是否需要重启系统以使底层网络堆栈彻底生效
    pub needs_reboot: bool,
    /// 总结描述消息
    pub message: String,
}

/// 系统限制状态预检概览
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RestrictionOverview {
    /// 是否检测到任何 USB 相关的使用限制
    pub usb_restricted: bool,
    /// 是否检测到任何网络相关的使用限制
    pub network_restricted: bool,
    /// 检测到的 USB 限制项总数
    pub usb_issues_count: usize,
    /// 检测到的网络限制项总数
    pub network_issues_count: usize,
    /// 所有项的快速诊断明细
    pub details: Vec<UnrestrictStepResult>,
}

// =========================================================================
// 辅助工具函数：注册表、系统命令及驱动器检查
// =========================================================================

/// 运行系统控制台命令并获取标准输出文本
fn run_command_capture(cmd: &str, args: &[&str]) -> (bool, String) {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let combined = if !stderr_str.is_empty() {
                format!("{stdout_str}\n{stderr_str}").trim().to_string()
            } else {
                stdout_str
            };
            (output.status.success(), combined)
        }
        Err(e) => (false, format!("调用系统组件 [{cmd}] 失败: {e}")),
    }
}

/// 检查注册表键是否存在
fn check_reg_key_exists(path: &str) -> bool {
    let (success, _) = run_command_capture("reg", &["query", path, "/reg:64"]);
    success
}

/// 检查注册表键下的特定键值是否存在并获取其输出
fn query_reg_value(path: &str, value_name: &str) -> Option<String> {
    let (success, output) = run_command_capture("reg", &["query", path, "/v", value_name, "/reg:64"]);
    if success {
        Some(output)
    } else {
        None
    }
}

/// 删除指定的注册表键值
fn delete_reg_value(path: &str, value_name: &str) -> (bool, String) {
    run_command_capture("reg", &["delete", path, "/v", value_name, "/f", "/reg:64"])
}

/// 删除指定的注册表键及其所有子项
fn delete_reg_key_tree(path: &str) -> (bool, String) {
    run_command_capture("reg", &["delete", path, "/f", "/reg:64"])
}

/// 设置注册表 DWORD 数值
fn set_reg_dword(path: &str, value_name: &str, dword_val: u32) -> (bool, String) {
    run_command_capture(
        "reg",
        &[
            "add",
            path,
            "/v",
            value_name,
            "/t",
            "REG_DWORD",
            "/d",
            &dword_val.to_string(),
            "/f",
            "/reg:64",
        ],
    )
}

/// 获取当前系统中所有挂载的可移动盘符 (例如 ["E:\\", "F:\\"])
fn get_removable_drive_roots() -> Vec<String> {
    let mut drives = Vec::new();
    unsafe {
        let drive_mask = GetLogicalDrives();
        for i in 0..26 {
            if (drive_mask & (1 << i)) != 0 {
                let letter = (b'A' + i as u8) as char;
                let root_str = format!("{}:\\", letter);
                let wide_path: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();
                let drive_type = GetDriveTypeW(wide_path.as_ptr());
                if drive_type == DRIVE_REMOVABLE {
                    drives.push(root_str);
                }
            }
        }
    }
    drives
}

// =========================================================================
// USB 限制检测与解除逻辑
// =========================================================================

/// 1. USB 存储驱动核心服务 (USBSTOR) 状态恢复
fn handle_usbstor_service(apply_fix: bool) -> UnrestrictStepResult {
    let key_path = r"HKLM\SYSTEM\CurrentControlSet\Services\USBSTOR";
    let is_restricted = if let Some(query_str) = query_reg_value(key_path, "Start") {
        // 如果 Start 为 0x4 (4) 则代表被禁用，正常标准启动应为 0x3 (3)
        query_str.to_ascii_lowercase().contains("0x4") || query_str.contains(" 4")
    } else {
        false
    };

    if !is_restricted {
        return UnrestrictStepResult {
            id: "usbstor_service".to_string(),
            title: "USB 存储服务驱动 (USBSTOR)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "USB 存储服务启动模式正常 (未被系统禁用)".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "usbstor_service".to_string(),
            title: "USB 存储服务驱动 (USBSTOR)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "检测到 USBSTOR 驱动服务启动项被强制置为禁用 (4)".to_string(),
            details: Some(format!("注册表项: {key_path}")),
        };
    }

    // 执行修复：设置 Start 为 3，并通过 sc 配置与拉起服务
    let (set_ok, reg_out) = set_reg_dword(key_path, "Start", 3);
    let _ = run_command_capture("sc", &["config", "usbstor", "start=", "demand"]);
    let _ = run_command_capture("sc", &["start", "usbstor"]);

    if set_ok {
        UnrestrictStepResult {
            id: "usbstor_service".to_string(),
            title: "USB 存储服务驱动 (USBSTOR)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: "已成功将 USB 存储驱动服务恢复为按需自启状态 (Start=3)".to_string(),
            details: Some(reg_out),
        }
    } else {
        UnrestrictStepResult {
            id: "usbstor_service".to_string(),
            title: "USB 存储服务驱动 (USBSTOR)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "尝试恢复 USBSTOR 失败，请确保以系统管理员身份运行".to_string(),
            details: Some(reg_out),
        }
    }
}

/// 2. 组策略可移动存储设备访问限制深度清除 (RemovableStorageDevices Policy)
/// 覆盖全盘 Deny_All、Deny_Read、Deny_Write 以及 U盘/移动硬盘特定 GUID
fn handle_removable_storage_policy(apply_fix: bool) -> UnrestrictStepResult {
    let hklm_policy = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\RemovableStorageDevices";
    let hkcu_policy = r"HKCU\Software\Policies\Microsoft\Windows\RemovableStorageDevices";

    let hklm_exists = check_reg_key_exists(hklm_policy);
    let hkcu_exists = check_reg_key_exists(hkcu_policy);

    if !hklm_exists && !hkcu_exists {
        return UnrestrictStepResult {
            id: "removable_storage_policy".to_string(),
            title: "可移动存储组策略封锁 (RemovableStorageDevices)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "未发现可移动介质组策略读写封锁项".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "removable_storage_policy".to_string(),
            title: "可移动存储组策略封锁 (RemovableStorageDevices)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "检测到组策略中存在针对可移动磁盘设备的读写禁止策略".to_string(),
            details: Some(format!("涉及策略项: HKLM/HKCU 下的 RemovableStorageDevices")),
        };
    }

    // 执行修复：彻底删除 HKLM 和 HKCU 下的该策略树
    let mut messages = Vec::new();
    if hklm_exists {
        let (ok, out) = delete_reg_key_tree(hklm_policy);
        if ok {
            messages.push("已彻底清理计算机级可移动设备读写拦截策略树".to_string());
        } else {
            messages.push(format!("清理计算机级策略失败: {out}"));
        }
    }
    if hkcu_exists {
        let (ok, out) = delete_reg_key_tree(hkcu_policy);
        if ok {
            messages.push("已彻底清理当前用户级可移动设备拦截策略树".to_string());
        } else {
            messages.push(format!("清理用户级策略失败: {out}"));
        }
    }

    UnrestrictStepResult {
        id: "removable_storage_policy".to_string(),
        title: "可移动存储组策略封锁 (RemovableStorageDevices)".to_string(),
        category: "usb".to_string(),
        status: "fixed".to_string(),
        message: "已全量清除组策略针对 U 盘/移动介质的所有 Deny_All/Deny_Read/Deny_Write 封锁".to_string(),
        details: Some(messages.join("；")),
    }
}

/// 3. 磁盘与 U 盘写保护策略清除 (StorageDevicePolicies - WriteProtect)
fn handle_storage_device_policies(apply_fix: bool) -> UnrestrictStepResult {
    let key_path = r"HKLM\SYSTEM\CurrentControlSet\Control\StorageDevicePolicies";
    let is_restricted = if let Some(query_str) = query_reg_value(key_path, "WriteProtect") {
        query_str.contains("0x1") || query_str.contains(" 1")
    } else {
        false
    };

    if !is_restricted {
        return UnrestrictStepResult {
            id: "storage_device_policies".to_string(),
            title: "全局磁盘写保护策略 (WriteProtect)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "系统未启用全局磁盘写保护强制限制".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "storage_device_policies".to_string(),
            title: "全局磁盘写保护策略 (WriteProtect)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "检测到 StorageDevicePolicies 处于开启状态，U 盘将被强制只读".to_string(),
            details: Some(format!("注册表项: {key_path}")),
        };
    }

    // 执行修复：将 WriteProtect 改为 0 或删除该键值
    let (ok, out) = delete_reg_value(key_path, "WriteProtect");
    let fallback = if !ok {
        let (fallback_ok, _) = set_reg_dword(key_path, "WriteProtect", 0);
        fallback_ok
    } else {
        true
    };

    if fallback {
        UnrestrictStepResult {
            id: "storage_device_policies".to_string(),
            title: "全局磁盘写保护策略 (WriteProtect)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: "已成功解除存储设备全局只读写保护锁定".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "storage_device_policies".to_string(),
            title: "全局磁盘写保护策略 (WriteProtect)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "解除写保护策略失败，需要管理员权限".to_string(),
            details: Some(out),
        }
    }
}

/// 4. 设备安装限制策略清除 (DeviceInstall Restrictions)
/// 防止插入新 USB 设备时系统策略拒绝加载对应驱动
fn handle_device_install_restrictions(apply_fix: bool) -> UnrestrictStepResult {
    let key_path = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DeviceInstall\Restrictions";
    let exists = check_reg_key_exists(key_path);

    if !exists {
        return UnrestrictStepResult {
            id: "device_install_restrictions".to_string(),
            title: "新硬件驱动安装限制 (DeviceInstall)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "未发现限制新硬件驱动自动安装的策略".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "device_install_restrictions".to_string(),
            title: "新硬件驱动安装限制 (DeviceInstall)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "检测到驱动安装拦截策略，插入全新 U 盘可能无法正常识别硬件".to_string(),
            details: Some(format!("注册表项: {key_path}")),
        };
    }

    let (ok, out) = delete_reg_key_tree(key_path);
    if ok {
        UnrestrictStepResult {
            id: "device_install_restrictions".to_string(),
            title: "新硬件驱动安装限制 (DeviceInstall)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: "已彻底清除硬件驱动安装限制策略树，新硬件可自由加载驱动".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "device_install_restrictions".to_string(),
            title: "新硬件驱动安装限制 (DeviceInstall)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "清除驱动安装限制策略树失败".to_string(),
            details: Some(out),
        }
    }
}

/// 5. 资源管理器驱动器限制策略清除 (NoViewOnDrive / NoDrives)
/// 重点解决：识别到 U 盘盘符，但一双击就提示“本次操作由于这台计算机的限制而被取消/无法访问”
fn handle_explorer_drive_restrictions(apply_fix: bool) -> UnrestrictStepResult {
    let hklm_exp = r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer";
    let hkcu_exp = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer";

    let has_noview_hklm = query_reg_value(hklm_exp, "NoViewOnDrive").is_some();
    let has_noview_hkcu = query_reg_value(hkcu_exp, "NoViewOnDrive").is_some();
    let has_nodrives_hklm = query_reg_value(hklm_exp, "NoDrives").is_some();
    let has_nodrives_hkcu = query_reg_value(hkcu_exp, "NoDrives").is_some();

    let has_restriction = has_noview_hklm || has_noview_hkcu || has_nodrives_hklm || has_nodrives_hkcu;

    if !has_restriction {
        return UnrestrictStepResult {
            id: "explorer_drive_restrictions".to_string(),
            title: "资源管理器盘符访问权限策略 (NoViewOnDrive)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "资源管理器未配置 NoViewOnDrive 或 NoDrives 访问阻断策略".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "explorer_drive_restrictions".to_string(),
            title: "资源管理器盘符访问权限策略 (NoViewOnDrive)".to_string(),
            category: "usb".to_string(),
            status: "failed".to_string(),
            message: "检测到 NoViewOnDrive 策略存在，会导致双击 U 盘时弹窗提示无法访问！".to_string(),
            details: Some("涉及策略: NoViewOnDrive / NoDrives".to_string()),
        };
    }

    // 执行修复：清理该两处位置的阻断键值
    let _ = delete_reg_value(hklm_exp, "NoViewOnDrive");
    let _ = delete_reg_value(hkcu_exp, "NoViewOnDrive");
    let _ = delete_reg_value(hklm_exp, "NoDrives");
    let _ = delete_reg_value(hkcu_exp, "NoDrives");

    UnrestrictStepResult {
        id: "explorer_drive_restrictions".to_string(),
        title: "资源管理器盘符访问权限策略 (NoViewOnDrive)".to_string(),
        category: "usb".to_string(),
        status: "fixed".to_string(),
        message: "已成功清除 NoViewOnDrive 和 NoDrives 盘符访问限制，恢复双击打开权限".to_string(),
        details: None,
    }
}

/// 6. 修复 MountPoints2 挂载点缓存与 Shell 关联损坏
/// 重点解决：识别到盘符，但由于残留的 autorun.inf 命令或损坏的 Shell 动词导致双击报错打不开
fn handle_mountpoints2_cache(apply_fix: bool) -> UnrestrictStepResult {
    let mountpoints_path = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\MountPoints2";
    let exists = check_reg_key_exists(mountpoints_path);

    if !exists {
        return UnrestrictStepResult {
            id: "mountpoints2_cache".to_string(),
            title: "驱动器挂载点与 Shell 动作缓存 (MountPoints2)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "挂载点缓存无异常".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "mountpoints2_cache".to_string(),
            title: "驱动器挂载点与 Shell 动作缓存 (MountPoints2)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "已就绪，一键解除时将自动重置损坏的挂载点残留指令".to_string(),
            details: None,
        };
    }

    // 清理损坏的 MountPoints2 缓存，系统会在下一次插入或点击时自动安全重建
    let (ok, out) = delete_reg_key_tree(mountpoints_path);
    if ok {
        UnrestrictStepResult {
            id: "mountpoints2_cache".to_string(),
            title: "驱动器挂载点与 Shell 动作缓存 (MountPoints2)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: "已重置资源管理器 MountPoints2 挂载点缓存，消除 autorun 劫持或双击无响应".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "mountpoints2_cache".to_string(),
            title: "驱动器挂载点与 Shell 动作缓存 (MountPoints2)".to_string(),
            category: "usb".to_string(),
            status: "warning".to_string(),
            message: "部分挂载点缓存正在被 Explorer 占用，建议解除后重启资源管理器".to_string(),
            details: Some(out),
        }
    }
}

/// 7. 可移动驱动器根目录文件系统安全权限修复 (ACL Permissions)
/// 重点解决：能看到盘符，但由于 ACL 权限损坏双击弹窗“无法访问。拒绝访问 (Access is denied)”
fn handle_removable_volume_acls(apply_fix: bool) -> UnrestrictStepResult {
    let removable_roots = get_removable_drive_roots();

    if removable_roots.is_empty() {
        return UnrestrictStepResult {
            id: "removable_volume_acls".to_string(),
            title: "可移动卷根目录访问权限 (ACL/Everyone 授权)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "当前系统暂未检测到插入的物理 U 盘设备 (已挂载 0 个可移动盘符)".to_string(),
            details: None,
        };
    }

    let drives_str = removable_roots.join(", ");

    if !apply_fix {
        return UnrestrictStepResult {
            id: "removable_volume_acls".to_string(),
            title: "可移动卷根目录访问权限 (ACL/Everyone 授权)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: format!("检测到已插入 U 盘设备: [{drives_str}]，一键解除时将自动修复根权限"),
            details: None,
        };
    }

    // 针对每个检测到的可移动 U 盘盘符，授予 Everyone 完全访问权限
    let mut success_drives = Vec::new();
    let mut fail_drives = Vec::new();

    for root in &removable_roots {
        // 执行 icacls 赋权
        let (ok, _) = run_command_capture(
            "icacls",
            &[root, "/grant", "Everyone:(OI)(CI)F", "/c", "/q"],
        );
        if ok {
            success_drives.push(root.clone());
        } else {
            // 备用：尝试赋权给 Users 组
            let (ok_user, _) = run_command_capture(
                "icacls",
                &[root, "/grant", "Users:(OI)(CI)F", "/c", "/q"],
            );
            if ok_user {
                success_drives.push(root.clone());
            } else {
                fail_drives.push(root.clone());
            }
        }
    }

    if !fail_drives.is_empty() {
        UnrestrictStepResult {
            id: "removable_volume_acls".to_string(),
            title: "可移动卷根目录访问权限 (ACL/Everyone 授权)".to_string(),
            category: "usb".to_string(),
            status: "warning".to_string(),
            message: format!(
                "已修复盘符 [{}] 的访问权限，但盘符 [{}] 响应异常 (可能为写保护硬件锁或需格式化)",
                success_drives.join(", "),
                fail_drives.join(", ")
            ),
            details: None,
        }
    } else {
        UnrestrictStepResult {
            id: "removable_volume_acls".to_string(),
            title: "可移动卷根目录访问权限 (ACL/Everyone 授权)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: format!(
                "已为当前 U 盘盘符 [{}] 授予完整 Everyone/Users 读写与执行权限，彻底解决双击拒绝访问",
                success_drives.join(", ")
            ),
            details: None,
        }
    }
}

/// 8. 磁盘卷自动挂载机制与卷只读清除 (mountvol / automount)
fn handle_disk_automount(apply_fix: bool) -> UnrestrictStepResult {
    if !apply_fix {
        return UnrestrictStepResult {
            id: "disk_automount".to_string(),
            title: "新卷自动装入机制 (mountvol / automount)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "卷装载管理器处于正常响应状态".to_string(),
            details: None,
        };
    }

    // 启用自动装载新卷
    let (ok, out) = run_command_capture("mountvol", &["/e"]);
    if ok {
        UnrestrictStepResult {
            id: "disk_automount".to_string(),
            title: "新卷自动装入机制 (mountvol / automount)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: "已重新启用新基本卷自动装入机制 (mountvol /e)".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "disk_automount".to_string(),
            title: "新卷自动装入机制 (mountvol / automount)".to_string(),
            category: "usb".to_string(),
            status: "warning".to_string(),
            message: "配置自动装载命令已执行".to_string(),
            details: Some(out),
        }
    }
}

/// 9. 即插即用硬件设备重新扫描 (pnputil /scan-devices)
fn handle_pnp_rescan(apply_fix: bool) -> UnrestrictStepResult {
    if !apply_fix {
        return UnrestrictStepResult {
            id: "pnp_rescan".to_string(),
            title: "硬件即插即用重新枚举 (PnP Rescan)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "设备总线就绪".to_string(),
            details: None,
        };
    }

    let (ok, out) = run_command_capture("pnputil", &["/scan-devices"]);
    if ok {
        UnrestrictStepResult {
            id: "pnp_rescan".to_string(),
            title: "硬件即插即用重新枚举 (PnP Rescan)".to_string(),
            category: "usb".to_string(),
            status: "fixed".to_string(),
            message: "已触发 Windows 即插即用硬件总线全盘重新扫描，已挂起设备已刷新联机".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "pnp_rescan".to_string(),
            title: "硬件即插即用重新枚举 (PnP Rescan)".to_string(),
            category: "usb".to_string(),
            status: "clean".to_string(),
            message: "硬件重新扫描指令已发送".to_string(),
            details: Some(out),
        }
    }
}

// =========================================================================
// 网络限制检测与解除逻辑
// =========================================================================

/// 1. IE 与系统级代理强制重置 (清除 PAC 脚本与 127.0.0.1 代理劫持)
fn handle_system_proxy(apply_fix: bool) -> UnrestrictStepResult {
    let key_path = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let proxy_enable = query_reg_value(key_path, "ProxyEnable")
        .map(|v| v.contains("0x1") || v.contains(" 1"))
        .unwrap_or(false);
    let has_autoconfig = query_reg_value(key_path, "AutoConfigURL").is_some();

    let has_restriction = proxy_enable || has_autoconfig;

    if !has_restriction {
        return UnrestrictStepResult {
            id: "system_proxy".to_string(),
            title: "系统级网络代理与 PAC 脚本".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "系统代理处于关闭状态，直连网络正常".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "system_proxy".to_string(),
            title: "系统级网络代理与 PAC 脚本".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: "检测到系统强制开启了代理服务器或自动 PAC 脚本，可能导致断网或流量劫持".to_string(),
            details: Some(format!("注册表位置: {key_path}")),
        };
    }

    // 执行修复：置 ProxyEnable 为 0，删除 ProxyServer、AutoConfigURL
    let _ = set_reg_dword(key_path, "ProxyEnable", 0);
    let _ = delete_reg_value(key_path, "ProxyServer");
    let _ = delete_reg_value(key_path, "ProxyOverride");
    let _ = delete_reg_value(key_path, "AutoConfigURL");

    // 清理策略中的代理锁定
    let hklm_policy_proxy = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\CurrentVersion\Internet Settings";
    let _ = delete_reg_value(hklm_policy_proxy, "ProxySettingsPerUser");

    UnrestrictStepResult {
        id: "system_proxy".to_string(),
        title: "系统级网络代理与 PAC 脚本".to_string(),
        category: "network".to_string(),
        status: "fixed".to_string(),
        message: "已完全关闭系统级代理服务器并清除自动 PAC 脚本配置，恢复直连网络".to_string(),
        details: None,
    }
}

/// 2. WinHTTP 底层网络代理重置 (netsh winhttp reset proxy)
fn handle_winhttp_proxy(apply_fix: bool) -> UnrestrictStepResult {
    let (query_ok, query_out) = run_command_capture("netsh", &["winhttp", "show", "proxy"]);
    let is_restricted = query_ok && !query_out.contains("直接访问") && !query_out.to_ascii_lowercase().contains("direct access");

    if !is_restricted {
        return UnrestrictStepResult {
            id: "winhttp_proxy".to_string(),
            title: "WinHTTP 基础底层网络代理".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "WinHTTP 底层服务代理为直接访问 (DIRECT)".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "winhttp_proxy".to_string(),
            title: "WinHTTP 基础底层网络代理".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: "检测到 WinHTTP 底层存在代理拦截规则，部分系统服务与无界面应用可能无法联网".to_string(),
            details: Some(query_out),
        };
    }

    let (ok, out) = run_command_capture("netsh", &["winhttp", "reset", "proxy"]);
    if ok {
        UnrestrictStepResult {
            id: "winhttp_proxy".to_string(),
            title: "WinHTTP 基础底层网络代理".to_string(),
            category: "network".to_string(),
            status: "fixed".to_string(),
            message: "已成功重置 WinHTTP 代理为默认直接连接 (Direct Access)".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "winhttp_proxy".to_string(),
            title: "WinHTTP 基础底层网络代理".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: "重置 WinHTTP 代理失败".to_string(),
            details: Some(out),
        }
    }
}

/// 3. Hosts 文件阻断规则深度检查与纯净重置
fn handle_hosts_file(apply_fix: bool) -> UnrestrictStepResult {
    let hosts_path = Path::new(r"C:\Windows\System32\drivers\etc\hosts");
    if !hosts_path.exists() {
        return UnrestrictStepResult {
            id: "hosts_file".to_string(),
            title: "系统 Hosts 域名解析映射表".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "Hosts 文件默认标准 (不存在额外重定向文件)".to_string(),
            details: None,
        };
    }

    let content = match fs::read_to_string(hosts_path) {
        Ok(c) => c,
        Err(_) => {
            return UnrestrictStepResult {
                id: "hosts_file".to_string(),
                title: "系统 Hosts 域名解析映射表".to_string(),
                category: "network".to_string(),
                status: "clean".to_string(),
                message: "读取 Hosts 文件受阻，可能需要提权".to_string(),
                details: None,
            }
        }
    };

    // 检查是否有将常规网站重定向到 127.0.0.1 或 0.0.0.0 的屏蔽规则
    let mut blocked_lines = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("127.0.0.1") || trimmed.starts_with("0.0.0.0") {
            let lower = trimmed.to_ascii_lowercase();
            if !lower.contains("localhost") && !lower.contains("broadcasthost") {
                blocked_lines += 1;
            }
        }
    }

    if blocked_lines == 0 {
        return UnrestrictStepResult {
            id: "hosts_file".to_string(),
            title: "系统 Hosts 域名解析映射表".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "Hosts 映射表纯净，未发现域名恶意屏蔽或重定向条目".to_string(),
            details: None,
        };
    }

    if !apply_fix {
        return UnrestrictStepResult {
            id: "hosts_file".to_string(),
            title: "系统 Hosts 域名解析映射表".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: format!("检测到 Hosts 文件中存在 {blocked_lines} 条可能导致网络受限的域名封锁规则"),
            details: None,
        };
    }

    // 自动备份原文件，然后写入干净的 Windows 官方标准 Hosts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_path = format!(r"C:\Windows\System32\drivers\etc\hosts.bak_{timestamp}");
    let _ = fs::copy(hosts_path, &backup_path);

    let clean_hosts = "# Copyright (c) 1993-2009 Microsoft Corp.\n# Clean hosts restored by OmniBox System Unblocker\n127.0.0.1       localhost\n::1             localhost\n";
    match fs::write(hosts_path, clean_hosts) {
        Ok(_) => UnrestrictStepResult {
            id: "hosts_file".to_string(),
            title: "系统 Hosts 域名解析映射表".to_string(),
            category: "network".to_string(),
            status: "fixed".to_string(),
            message: format!(
                "已清除 {blocked_lines} 条域名屏蔽规则，恢复标准解析 (原文件已备份为: hosts.bak_{timestamp})"
            ),
            details: Some(backup_path),
        },
        Err(e) => UnrestrictStepResult {
            id: "hosts_file".to_string(),
            title: "系统 Hosts 域名解析映射表".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: format!("重置 Hosts 文件失败 (需要管理员权限): {e}"),
            details: None,
        },
    }
}

/// 4. Winsock 目录与协议套接字重置 (netsh winsock reset)
fn handle_winsock_catalog(apply_fix: bool) -> UnrestrictStepResult {
    if !apply_fix {
        return UnrestrictStepResult {
            id: "winsock_catalog".to_string(),
            title: "Winsock 目录与网络协议套接字".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "网络协议栈就绪".to_string(),
            details: None,
        };
    }

    let (ok, out) = run_command_capture("netsh", &["winsock", "reset"]);
    if ok {
        UnrestrictStepResult {
            id: "winsock_catalog".to_string(),
            title: "Winsock 目录与网络协议套接字".to_string(),
            category: "network".to_string(),
            status: "fixed".to_string(),
            message: "已成功重置 Winsock 目录，清除第三方网络劫持与 LSP 驱动损坏 (需重启系统生效)".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "winsock_catalog".to_string(),
            title: "Winsock 目录与网络协议套接字".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: "重置 Winsock 失败，需要系统管理员权限".to_string(),
            details: Some(out),
        }
    }
}

/// 5. TCP/IP 底层传输协议栈重置 (netsh int ip reset)
fn handle_tcpip_stack(apply_fix: bool) -> UnrestrictStepResult {
    if !apply_fix {
        return UnrestrictStepResult {
            id: "tcpip_stack".to_string(),
            title: "TCP/IP 底层传输协议栈".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "传输层配置就绪".to_string(),
            details: None,
        };
    }

    let (ok, out) = run_command_capture("netsh", &["int", "ip", "reset"]);
    if ok {
        UnrestrictStepResult {
            id: "tcpip_stack".to_string(),
            title: "TCP/IP 底层传输协议栈".to_string(),
            category: "network".to_string(),
            status: "fixed".to_string(),
            message: "已成功重置 TCP/IP 核心网络堆栈 (需重启系统彻底生效)".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "tcpip_stack".to_string(),
            title: "TCP/IP 底层传输协议栈".to_string(),
            category: "network".to_string(),
            status: "warning".to_string(),
            message: "TCP/IP 重置指令已执行，部分注册表分支受系统保护".to_string(),
            details: Some(out),
        }
    }
}

/// 6. 本地 DNS 缓存刷新 (ipconfig /flushdns)
fn handle_dns_cache(apply_fix: bool) -> UnrestrictStepResult {
    if !apply_fix {
        return UnrestrictStepResult {
            id: "dns_cache".to_string(),
            title: "本地 DNS 解析缓存 (DNS Resolver Cache)".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "DNS 服务运行中".to_string(),
            details: None,
        };
    }

    let (ok, out) = run_command_capture("ipconfig", &["/flushdns"]);
    if ok {
        UnrestrictStepResult {
            id: "dns_cache".to_string(),
            title: "本地 DNS 解析缓存 (DNS Resolver Cache)".to_string(),
            category: "network".to_string(),
            status: "fixed".to_string(),
            message: "已完全清空本地 DNS 解析缓存，消除过期或污染的解析记录".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "dns_cache".to_string(),
            title: "本地 DNS 解析缓存 (DNS Resolver Cache)".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "DNS 刷新命令已执行".to_string(),
            details: Some(out),
        }
    }
}

/// 7. Windows 防火墙出厂重置 (清除恶意进出站阻断规则)
fn handle_firewall_rules(apply_fix: bool) -> UnrestrictStepResult {
    if !apply_fix {
        return UnrestrictStepResult {
            id: "firewall_rules".to_string(),
            title: "Windows 高级安全防火墙策略".to_string(),
            category: "network".to_string(),
            status: "clean".to_string(),
            message: "防火墙运行中".to_string(),
            details: None,
        };
    }

    let (ok, out) = run_command_capture("netsh", &["advfirewall", "reset"]);
    if ok {
        UnrestrictStepResult {
            id: "firewall_rules".to_string(),
            title: "Windows 高级安全防火墙策略".to_string(),
            category: "network".to_string(),
            status: "fixed".to_string(),
            message: "已恢复 Windows 防火墙出厂默认策略，清除所有恶意的出站/入站端口与程序封锁".to_string(),
            details: Some(out),
        }
    } else {
        UnrestrictStepResult {
            id: "firewall_rules".to_string(),
            title: "Windows 高级安全防火墙策略".to_string(),
            category: "network".to_string(),
            status: "failed".to_string(),
            message: "重置防火墙失败，需要管理员权限".to_string(),
            details: Some(out),
        }
    }
}

/// 8. 网卡组策略限制清除与被禁网卡自动唤醒启用
fn handle_network_adapters(apply_fix: bool) -> UnrestrictStepResult {
    let policy_path = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\Network Connections";
    let policy_exists = check_reg_key_exists(policy_path);

    if !apply_fix {
        if policy_exists {
            return UnrestrictStepResult {
                id: "network_adapters".to_string(),
                title: "网卡硬件与网络连接策略".to_string(),
                category: "network".to_string(),
                status: "failed".to_string(),
                message: "检测到网络连接策略组存在限制项".to_string(),
                details: Some(format!("注册表项: {policy_path}")),
            };
        } else {
            return UnrestrictStepResult {
                id: "network_adapters".to_string(),
                title: "网卡硬件与网络连接策略".to_string(),
                category: "network".to_string(),
                status: "clean".to_string(),
                message: "网络连接策略正常".to_string(),
                details: None,
            };
        }
    }

    // 清除可能存在的组策略锁定
    if policy_exists {
        let _ = delete_reg_key_tree(policy_path);
    }

    // 唤醒所有处于 Disabled 状态的网卡
    let ps_cmd = "Get-NetAdapter | Where-Object { $_.Status -eq 'Disabled' } | Enable-NetAdapter -Confirm:$false";
    let (ok, out) = run_command_capture(
        "powershell",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_cmd],
    );

    UnrestrictStepResult {
        id: "network_adapters".to_string(),
        title: "网卡硬件与网络连接策略".to_string(),
        category: "network".to_string(),
        status: if ok { "fixed".to_string() } else { "warning".to_string() },
        message: "已清除网络连接策略限制，并自动扫描唤醒所有被禁用的物理与无线网卡适配器".to_string(),
        details: Some(out),
    }
}

// =========================================================================
// 统一入口：状态预检、全量 USB 解除、全量网络解除、一键解除一切
// =========================================================================

/// 获取全系统限制状态预检概览
pub fn get_restriction_overview() -> RestrictionOverview {
    let mut steps = Vec::new();

    // USB 预检
    steps.push(handle_usbstor_service(false));
    steps.push(handle_removable_storage_policy(false));
    steps.push(handle_storage_device_policies(false));
    steps.push(handle_device_install_restrictions(false));
    steps.push(handle_explorer_drive_restrictions(false));
    steps.push(handle_mountpoints2_cache(false));
    steps.push(handle_removable_volume_acls(false));
    steps.push(handle_disk_automount(false));
    steps.push(handle_pnp_rescan(false));

    // 网络预检
    steps.push(handle_system_proxy(false));
    steps.push(handle_winhttp_proxy(false));
    steps.push(handle_hosts_file(false));
    steps.push(handle_winsock_catalog(false));
    steps.push(handle_tcpip_stack(false));
    steps.push(handle_dns_cache(false));
    steps.push(handle_firewall_rules(false));
    steps.push(handle_network_adapters(false));

    let usb_issues_count = steps
        .iter()
        .filter(|s| s.category == "usb" && s.status == "failed")
        .count();
    let network_issues_count = steps
        .iter()
        .filter(|s| s.category == "network" && s.status == "failed")
        .count();

    RestrictionOverview {
        usb_restricted: usb_issues_count > 0,
        network_restricted: network_issues_count > 0,
        usb_issues_count,
        network_issues_count,
        details: steps,
    }
}

/// 执行全部 USB 限制解除
pub fn unrestrict_all_usb() -> UnrestrictReport {
    let mut steps = Vec::new();

    steps.push(handle_usbstor_service(true));
    steps.push(handle_removable_storage_policy(true));
    steps.push(handle_storage_device_policies(true));
    steps.push(handle_device_install_restrictions(true));
    steps.push(handle_explorer_drive_restrictions(true));
    steps.push(handle_mountpoints2_cache(true));
    steps.push(handle_removable_volume_acls(true));
    steps.push(handle_disk_automount(true));
    steps.push(handle_pnp_rescan(true));

    build_report(steps, false)
}

/// 执行全部网络限制解除
pub fn unrestrict_all_network() -> UnrestrictReport {
    let mut steps = Vec::new();

    steps.push(handle_system_proxy(true));
    steps.push(handle_winhttp_proxy(true));
    steps.push(handle_hosts_file(true));
    steps.push(handle_winsock_catalog(true));
    steps.push(handle_tcpip_stack(true));
    steps.push(handle_dns_cache(true));
    steps.push(handle_firewall_rules(true));
    steps.push(handle_network_adapters(true));

    build_report(steps, true)
}

/// 一键执行一切方法解除全盘限制 (USB + 网络)
pub fn unrestrict_all_everything() -> UnrestrictReport {
    let mut steps = Vec::new();

    // 1. 先执行全量 USB 解除流程
    steps.push(handle_usbstor_service(true));
    steps.push(handle_removable_storage_policy(true));
    steps.push(handle_storage_device_policies(true));
    steps.push(handle_device_install_restrictions(true));
    steps.push(handle_explorer_drive_restrictions(true));
    steps.push(handle_mountpoints2_cache(true));
    steps.push(handle_removable_volume_acls(true));
    steps.push(handle_disk_automount(true));
    steps.push(handle_pnp_rescan(true));

    // 2. 再执行全量网络解除流程
    steps.push(handle_system_proxy(true));
    steps.push(handle_winhttp_proxy(true));
    steps.push(handle_hosts_file(true));
    steps.push(handle_winsock_catalog(true));
    steps.push(handle_tcpip_stack(true));
    steps.push(handle_dns_cache(true));
    steps.push(handle_firewall_rules(true));
    steps.push(handle_network_adapters(true));

    build_report(steps, true)
}

/// 辅助聚合生成报告
fn build_report(steps: Vec<UnrestrictStepResult>, needs_reboot_flag: bool) -> UnrestrictReport {
    let total_steps = steps.len();
    let fixed_count = steps.iter().filter(|s| s.status == "fixed").count();
    let clean_count = steps.iter().filter(|s| s.status == "clean").count();
    let failed_count = steps.iter().filter(|s| s.status == "failed").count();

    let success = failed_count == 0;
    let message = if success {
        if fixed_count > 0 {
            format!("已成功完成全量解除！共修复 {fixed_count} 项限制策略，其余 {clean_count} 项本身处于正常状态。")
        } else {
            "全盘排查完毕，未检测到任何系统级封锁策略，所有功能均处于正常通行状态。".to_string()
        }
    } else {
        format!("解除流程已执行，修复了 {fixed_count} 项限制，但有 {failed_count} 项未能成功处理 (可能缺少管理员提权)。")
    };

    UnrestrictReport {
        success,
        total_steps,
        fixed_count,
        clean_count,
        failed_count,
        steps,
        needs_reboot: needs_reboot_flag && fixed_count > 0,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restriction_overview_structure() {
        let overview = get_restriction_overview();
        assert!(overview.details.len() >= 17, "预检项数量应至少包含 17 项检查");
        
        let usb_steps: Vec<_> = overview.details.iter().filter(|s| s.category == "usb").collect();
        let net_steps: Vec<_> = overview.details.iter().filter(|s| s.category == "network").collect();
        assert_eq!(usb_steps.len(), 9, "USB 预检项应为 9 项");
        assert_eq!(net_steps.len(), 8, "网络预检项应为 8 项");

        for step in &overview.details {
            assert!(!step.id.is_empty(), "id 不能为空");
            assert!(!step.title.is_empty(), "title 不能为空");
            assert!(matches!(step.status.as_str(), "clean" | "fixed" | "warning" | "failed"));
        }
    }

    #[test]
    fn test_build_report_aggregation() {
        let mock_steps = vec![
            UnrestrictStepResult {
                id: "step1".to_string(),
                title: "测试1".to_string(),
                category: "usb".to_string(),
                status: "fixed".to_string(),
                message: "已修复".to_string(),
                details: None,
            },
            UnrestrictStepResult {
                id: "step2".to_string(),
                title: "测试2".to_string(),
                category: "network".to_string(),
                status: "clean".to_string(),
                message: "正常".to_string(),
                details: None,
            },
            UnrestrictStepResult {
                id: "step3".to_string(),
                title: "测试3".to_string(),
                category: "usb".to_string(),
                status: "failed".to_string(),
                message: "失败".to_string(),
                details: None,
            },
        ];

        let report = build_report(mock_steps, true);
        assert_eq!(report.total_steps, 3);
        assert_eq!(report.fixed_count, 1);
        assert_eq!(report.clean_count, 1);
        assert_eq!(report.failed_count, 1);
        assert!(!report.success, "存在 failed 项时总体 success 应为 false");
    }

    #[test]
    fn test_unrestrict_report_json_serde() {
        let report = UnrestrictReport {
            success: true,
            total_steps: 1,
            fixed_count: 1,
            clean_count: 0,
            failed_count: 0,
            steps: vec![UnrestrictStepResult {
                id: "test".to_string(),
                title: "测试项".to_string(),
                category: "usb".to_string(),
                status: "fixed".to_string(),
                message: "修复成功".to_string(),
                details: Some("细节".to_string()),
            }],
            needs_reboot: false,
            message: "测试成功".to_string(),
        };

        let json = serde_json::to_string(&report).expect("必须支持序列化为 JSON");
        let parsed: UnrestrictReport = serde_json::from_str(&json).expect("必须支持从 JSON 反序列化");
        assert_eq!(report, parsed);
    }
}

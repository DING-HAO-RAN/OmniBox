//! Windows 原生管理与诊断工具快捷唤起模块
//!
//! 提供系统维护、诊断配置、硬件检测等几十种原生实用工具的秒级唤起能力。

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// 系统内置工具项元数据
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SystemToolItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String, // "admin" (管理维护), "diag" (性能诊断), "config" (系统配置), "network" (网络安全)
    pub command: String,
    pub icon_name: String,
}

/// 获取全套 Windows 原生系统工具清单
pub fn get_system_tools_list() -> Vec<SystemToolItem> {
    vec![
        // ================= 管理与维护 =================
        SystemToolItem {
            id: "gpedit".to_string(),
            name: "组策略编辑器".to_string(),
            description: "配置 Windows 用户权限、安全策略与系统核心行为 (gpedit.msc)".to_string(),
            category: "admin".to_string(),
            command: "gpedit.msc".to_string(),
            icon_name: "ShieldCheck".to_string(),
        },
        SystemToolItem {
            id: "regedit".to_string(),
            name: "注册表编辑器".to_string(),
            description: "查看与修改 Windows 核心配置键值 (regedit.exe)".to_string(),
            category: "admin".to_string(),
            command: "regedit.exe".to_string(),
            icon_name: "Database".to_string(),
        },
        SystemToolItem {
            id: "compmgmt".to_string(),
            name: "计算机管理".to_string(),
            description: "集成管理事件、共享文件夹、本地用户和磁盘 (compmgmt.msc)".to_string(),
            category: "admin".to_string(),
            command: "compmgmt.msc".to_string(),
            icon_name: "MonitorCheck".to_string(),
        },
        SystemToolItem {
            id: "services".to_string(),
            name: "系统服务管理".to_string(),
            description: "管理 Windows 后台服务的启动类型与运行状态 (services.msc)".to_string(),
            category: "admin".to_string(),
            command: "services.msc".to_string(),
            icon_name: "Cog".to_string(),
        },
        SystemToolItem {
            id: "devmgmt".to_string(),
            name: "设备管理器".to_string(),
            description: "查看和更新显卡、声卡、主板等硬件设备驱动 (devmgmt.msc)".to_string(),
            category: "admin".to_string(),
            command: "devmgmt.msc".to_string(),
            icon_name: "HardDrive".to_string(),
        },
        SystemToolItem {
            id: "diskmgmt".to_string(),
            name: "磁盘管理".to_string(),
            description: "格式化分区、调整驱动器卷大小与盘符 (diskmgmt.msc)".to_string(),
            category: "admin".to_string(),
            command: "diskmgmt.msc".to_string(),
            icon_name: "PieChart".to_string(),
        },
        SystemToolItem {
            id: "taskmgr".to_string(),
            name: "任务管理器".to_string(),
            description: "查看实时进程、启动项与详细性能 (taskmgr.exe)".to_string(),
            category: "admin".to_string(),
            command: "taskmgr.exe".to_string(),
            icon_name: "Activity".to_string(),
        },
        SystemToolItem {
            id: "fsmgmt".to_string(),
            name: "共享文件夹管理".to_string(),
            description: "查看和管理当前局域网共享资源与会话 (fsmgmt.msc)".to_string(),
            category: "admin".to_string(),
            command: "fsmgmt.msc".to_string(),
            icon_name: "FolderSymlink".to_string(),
        },
        // ================= 性能与诊断 =================
        SystemToolItem {
            id: "resmon".to_string(),
            name: "资源监视器".to_string(),
            description: "深度排查哪个进程占用了 CPU、内存、磁盘与网络 (resmon.exe)".to_string(),
            category: "diag".to_string(),
            command: "resmon.exe".to_string(),
            icon_name: "Gauge".to_string(),
        },
        SystemToolItem {
            id: "perfmon".to_string(),
            name: "性能监视器".to_string(),
            description: "分析系统日志与高级计数器报表 (perfmon.msc)".to_string(),
            category: "diag".to_string(),
            command: "perfmon.msc".to_string(),
            icon_name: "LineChart".to_string(),
        },
        SystemToolItem {
            id: "eventvwr".to_string(),
            name: "事件查看器".to_string(),
            description: "排查系统崩溃、应用程序报错与安全审计日志 (eventvwr.msc)".to_string(),
            category: "diag".to_string(),
            command: "eventvwr.msc".to_string(),
            icon_name: "FileText".to_string(),
        },
        SystemToolItem {
            id: "dxdiag".to_string(),
            name: "DirectX 诊断工具".to_string(),
            description: "全面检测显卡驱动、显存、声卡与图形接口状态 (dxdiag.exe)".to_string(),
            category: "diag".to_string(),
            command: "dxdiag.exe".to_string(),
            icon_name: "Cpu".to_string(),
        },
        SystemToolItem {
            id: "msinfo32".to_string(),
            name: "系统信息摘要".to_string(),
            description: "查看 BIOS 版本、主板芯片、硬件资源分配详表 (msinfo32.exe)".to_string(),
            category: "diag".to_string(),
            command: "msinfo32.exe".to_string(),
            icon_name: "Info".to_string(),
        },
        // ================= 系统配置与优化 =================
        SystemToolItem {
            id: "msconfig".to_string(),
            name: "系统配置 (msconfig)".to_string(),
            description: "配置引导启动选项、安全模式与系统服务引导 (msconfig.exe)".to_string(),
            category: "config".to_string(),
            command: "msconfig.exe".to_string(),
            icon_name: "Sliders".to_string(),
        },
        SystemToolItem {
            id: "sysprop".to_string(),
            name: "高级系统属性 & 环境变量".to_string(),
            description:
                "快速编辑系统 PATH 环境变量、虚拟内存与系统保护 (SystemPropertiesAdvanced)"
                    .to_string(),
            category: "config".to_string(),
            command: "SystemPropertiesAdvanced.exe".to_string(),
            icon_name: "Variable".to_string(),
        },
        SystemToolItem {
            id: "control".to_string(),
            name: "经典控制面板".to_string(),
            description: "开启 Windows 经典控制中心所有功能 (control.exe)".to_string(),
            category: "config".to_string(),
            command: "control.exe".to_string(),
            icon_name: "LayoutGrid".to_string(),
        },
        SystemToolItem {
            id: "cleanmgr".to_string(),
            name: "磁盘清理工具".to_string(),
            description: "清理 Windows 旧更新备份、临时文件与回收站 (cleanmgr.exe)".to_string(),
            category: "config".to_string(),
            command: "cleanmgr.exe".to_string(),
            icon_name: "Sparkles".to_string(),
        },
        // ================= 网络与安全终端 =================
        SystemToolItem {
            id: "wf".to_string(),
            name: "高级安全 Windows 防火墙".to_string(),
            description: "设置入站/出站端口拦截策略与自定义网络规则 (wf.msc)".to_string(),
            category: "network".to_string(),
            command: "wf.msc".to_string(),
            icon_name: "Shield".to_string(),
        },
        SystemToolItem {
            id: "ncpa".to_string(),
            name: "网络连接适配器面板".to_string(),
            description: "快速修改本地以太网、WLAN 的 IPv4/IPv6 与 DNS 属性 (ncpa.cpl)".to_string(),
            category: "network".to_string(),
            command: "ncpa.cpl".to_string(),
            icon_name: "Network".to_string(),
        },
        SystemToolItem {
            id: "secpol".to_string(),
            name: "本地安全策略".to_string(),
            description: "配置密码安全度、账户锁定与审核策略 (secpol.msc)".to_string(),
            category: "network".to_string(),
            command: "secpol.msc".to_string(),
            icon_name: "Key".to_string(),
        },
        SystemToolItem {
            id: "mstsc".to_string(),
            name: "远程桌面连接 (RDP)".to_string(),
            description: "连接远程 Windows 服务器与桌面主机 (mstsc.exe)".to_string(),
            category: "network".to_string(),
            command: "mstsc.exe".to_string(),
            icon_name: "ScreenShare".to_string(),
        },
        SystemToolItem {
            id: "cmd_admin".to_string(),
            name: "命令提示符 (CMD)".to_string(),
            description: "启动 Windows 命令解释器环境 (cmd.exe)".to_string(),
            category: "network".to_string(),
            command: "cmd.exe".to_string(),
            icon_name: "Terminal".to_string(),
        },
        SystemToolItem {
            id: "powershell".to_string(),
            name: "Windows PowerShell".to_string(),
            description: "强大的自动化与脚本终端环境 (powershell.exe)".to_string(),
            category: "network".to_string(),
            command: "powershell.exe".to_string(),
            icon_name: "TerminalSquare".to_string(),
        },
    ]
}

/// 唤起指定的 Windows 原生系统工具
pub fn launch_tool_command(command: &str) -> Result<(), String> {
    let wide_cmd: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let res = ShellExecuteW(
            0 as HWND,
            std::ptr::null(),
            wide_cmd.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL as i32,
        );

        // ShellExecute 返回值大于 32 表示成功唤起
        if (res as usize) > 32 {
            Ok(())
        } else {
            // 降级使用 std::process::Command 启动
            std::process::Command::new("cmd")
                .args(["/c", "start", "", command])
                .spawn()
                .map_err(|e| format!("启动系统工具 [{command}] 失败: {e}"))?;
            Ok(())
        }
    }
}

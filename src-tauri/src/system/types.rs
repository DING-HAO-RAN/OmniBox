//! 系统管理模块的核心数据结构定义
//!
//! 包含内存状态、清理结果以及启动项配置的数据结构。

use serde::{Deserialize, Serialize};

/// 物理内存运行状态
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MemoryStatus {
    /// 物理总内存 (字节)
    pub total_ram: u64,
    /// 可用物理内存 (字节)
    pub available_ram: u64,
    /// 已用物理内存 (字节)
    pub used_ram: u64,
    /// 内存占用百分比 (0.0 - 100.0)
    pub usage_percent: f64,
}

/// 内存工作集清理执行结果
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CleanResult {
    /// 释放的字节数 (若前后未释放则保底为 0)
    pub freed_bytes: u64,
    /// 释放的 MB 数
    pub freed_mb: f64,
    /// 成功修剪工作集的进程数量
    pub processes_trimmed: u32,
    /// 清理前内存占用百分比
    pub before_usage_percent: f64,
    /// 清理后内存占用百分比
    pub after_usage_percent: f64,
    /// 释放的备用列表/待机缓存（Standby Cache）字节数
    pub standby_freed_bytes: u64,
    /// 清理模式（如 "PCL2 深度内核优化 (待机缓存+工作集)" 或 "标准工作集优化"）
    pub clean_mode: String,
    /// 执行时是否具备管理员权限
    pub is_admin: bool,
}

/// 自定义启动项配置
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LaunchItem {
    /// 启动项唯一标识 ID
    pub id: String,
    /// 启动项显示名称
    pub name: String,
    /// 目标程序或脚本文件路径
    pub path: String,
    /// 附加命令行参数
    pub args: String,
    /// 指定执行工作目录 (可选)
    pub work_dir: Option<String>,
    /// 是否以无窗口静默模式运行
    pub silent: bool,
    /// 是否处于启用状态
    pub enabled: bool,
}

/// 应用程序通用设置
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppSettings {
    /// 是否以管理员权限自启/首选运行
    pub run_as_admin_default: bool,
    /// 界面语言 ("zh-CN" | "en-US")
    pub language: String,
    /// 是否开机自动启动
    pub auto_start: bool,
    /// 开机自启时是否默认静默最小化
    pub start_minimized: bool,
    /// 关闭窗口时最小化到托盘还是退出
    pub close_to_tray: bool,
    /// 是否启用内存智能自动清理
    pub auto_clean_memory: bool,
    /// 自动清理内存的占用率阈值 (例如 80 代表 80%)
    pub auto_clean_threshold: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            run_as_admin_default: false,
            language: "zh-CN".to_string(),
            auto_start: false,
            start_minimized: false,
            close_to_tray: false,
            auto_clean_memory: false,
            auto_clean_threshold: 80,
        }
    }
}

/// 深度隐藏文件/文件夹条目
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CloakedItem {
    /// 唯一标识 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 文件或文件夹完整绝对路径
    pub path: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 添加时间戳 (毫秒)
    pub added_at: u64,
    /// 当前是否处于超级隐藏状态
    pub is_cloaked: bool,
    /// 备注说明
    pub note: String,
}

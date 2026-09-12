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

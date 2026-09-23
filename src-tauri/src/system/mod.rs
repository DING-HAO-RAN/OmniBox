//! Windows 底层系统核心接口模块
//!
//! 包含内存监测与工作集修剪、多模式进程启动及相关数据结构。

pub mod cloaker;
pub mod hardware;
pub mod info;
pub mod memory;
pub mod process;
pub mod settings;
pub mod sys_tools;
pub mod tweaks;
pub mod types;
pub mod unrestrict;

pub use hardware::{
    get_hardware_performance, HardwarePerformance, RuntimeDiskInfo, RuntimeNetworkInfo,
};
pub use info::{
    collect_full_system_report, collect_memory_info, export_system_report_json, SystemFullReport,
};
pub use memory::{clean_process_working_sets, get_memory_info, is_running_as_admin};
pub use process::launch_process;
pub use sys_tools::{get_system_tools_list, launch_tool_command, SystemToolItem};
pub use tweaks::{get_all_tweaks, toggle_tweak, SystemTweakItem};
pub use types::{AppSettings, CleanResult, CloakedItem, LaunchItem, MemoryStatus};
pub use unrestrict::{
    get_restriction_overview, unrestrict_all_everything, unrestrict_all_network,
    unrestrict_all_usb, RestrictionOverview, UnrestrictReport, UnrestrictStepResult,
};

#[cfg(test)]
mod tests;

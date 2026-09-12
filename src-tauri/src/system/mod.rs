//! Windows 底层系统核心接口模块
//!
//! 包含内存监测与工作集修剪、多模式进程启动及相关数据结构。

pub mod memory;
pub mod process;
pub mod types;

pub use memory::{clean_process_working_sets, get_memory_info};
pub use process::launch_process;
pub use types::{CleanResult, LaunchItem, MemoryStatus};

#[cfg(test)]
mod tests;

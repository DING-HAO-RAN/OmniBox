// Windows 高速多线程下载器底层模块
// 包含 WinHTTP 客户端封装、分块切分算法与核心数据模型、多线程并发 Seek 写入引擎与任务管理器

pub mod client;
pub mod commands;
pub mod engine;
pub mod types;

pub use engine::DownloadManager;
pub use types::{DownloadChunk, DownloadTask, NewTaskParams, TaskStatus, UrlMeta};

#[cfg(test)]
mod tests;

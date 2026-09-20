// Windows 高速多线程下载器底层模块
// 包含 WinHTTP 客户端封装、分块切分算法与核心数据模型

pub mod client;
pub mod types;

#[cfg(test)]
mod tests;

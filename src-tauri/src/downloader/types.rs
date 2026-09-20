// 核心下载任务数据模型与状态定义
// 遵循 Windows 11 Fluent 架构标准，提供前后端序列化对齐的数据契约

use serde::{Deserialize, Serialize};

/// 单个分块下载进度与区间状态
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DownloadChunk {
    /// 分片序号 (从 0 开始)
    pub id: usize,
    /// 分片在目标文件中的起始字节偏移 (闭区间)
    pub start: u64,
    /// 分片在目标文件中的结束字节偏移 (闭区间)
    pub end: u64,
    /// 该分片已下载的字节数
    pub downloaded: u64,
    /// 该分片是否已全部下载完成
    pub is_finished: bool,
}

/// 任务生命周期状态
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    /// 等待开始或初始化探测中
    Pending,
    /// 正在下载传输
    Downloading,
    /// 已被用户暂停
    Paused,
    /// 全部数据传输完成且文件校验成功
    Completed,
    /// 下载遇到不可恢复的错误失败
    Failed,
    /// 任务已被取消
    Cancelled,
}

/// 完整的下载任务定义
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DownloadTask {
    /// 任务唯一标识 UUID
    pub id: String,
    /// 远程下载目标 URL
    pub url: String,
    /// 保存的文件名
    pub file_name: String,
    /// 保存的目标文件完整路径
    pub save_path: String,
    /// 文件总大小 (字节，0 表示未知/不支持预获取)
    pub total_bytes: u64,
    /// 已累计下载完成的字节总数
    pub downloaded_bytes: u64,
    /// 整体下载进度百分比 (0.0 ~ 100.0)
    pub progress_percent: f64,
    /// 当前瞬时下载速率 (字节/秒)
    pub speed_bps: u64,
    /// 预估剩余下载时间 (秒，0 表示无法预估或已完成)
    pub eta_seconds: u64,
    /// 当前任务生命周期状态
    pub status: TaskStatus,
    /// 分块并发线程数 (1 ~ 32)
    pub thread_count: usize,
    /// 服务端是否支持 HTTP Range 分块断点续传
    pub supports_range: bool,
    /// 失败时的错误信息
    pub error_message: Option<String>,
    /// 任务创建的时间戳 (毫秒或秒)
    pub created_at: u64,
    /// 各分片当前进度详细列表 (用于前端热力图绘制)
    pub chunks: Vec<DownloadChunk>,
}

/// URL 预探测元数据结果
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UrlMeta {
    /// 文件总大小 (字节，0 表示未知)
    pub total_bytes: u64,
    /// 服务端是否支持 Range 分块下载
    pub supports_range: bool,
    /// 服务端建议文件名或根据 URL 提取的默认文件名
    pub suggested_filename: String,
}

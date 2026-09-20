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

/// 创建新下载任务的参数
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NewTaskParams {
    /// 下载目标链接
    pub url: String,
    /// 保存目录 (若为 None 则使用默认下载目录)
    pub save_dir: Option<String>,
    /// 自定义保存文件名 (若为 None 则自动解析)
    pub file_name: Option<String>,
    /// 并发分块线程数 (1 ~ 32，若为 None 则默认 4)
    pub threads: Option<usize>,
}

impl DownloadTask {
    /// 获取未完成的分片列表引用
    pub fn get_unfinished_chunks(&self) -> Vec<DownloadChunk> {
        self.chunks
            .iter()
            .filter(|c| !c.is_finished && c.downloaded < (c.end.saturating_sub(c.start) + 1))
            .cloned()
            .collect()
    }

    /// 伴生临时文件路径 (.downloading)
    pub fn downloading_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}.downloading", self.save_path))
    }

    /// 伴生断点续传元数据路径 (.part.json)
    pub fn part_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}.part.json", self.save_path))
    }

    /// 根据各分片游标重新汇总已下载总量与进度百分比
    pub fn refresh_progress_from_chunks(&mut self) {
        if self.chunks.is_empty() {
            return;
        }

        let mut sum_downloaded = 0u64;
        let mut all_finished = true;

        for chunk in &self.chunks {
            sum_downloaded = sum_downloaded.saturating_add(chunk.downloaded);
            if !chunk.is_finished {
                all_finished = false;
            }
        }

        self.downloaded_bytes = sum_downloaded;
        if self.total_bytes > 0 {
            let percent = (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0;
            self.progress_percent = (percent * 100.0).round() / 100.0;
            if self.downloaded_bytes >= self.total_bytes && all_finished {
                self.progress_percent = 100.0;
            }
        }
    }
}

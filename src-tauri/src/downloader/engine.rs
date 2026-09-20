// Windows 高速零碎片多线程并发下载引擎与任务管理器
// 核心原则：
// 1. 文件句柄直接 Seek 写入预分配 .downloading 临时文件，免去事后碎片合并开销
// 2. 伴生 .part.json 记录分块游标，实现断点续传
// 3. 滑动窗口测速与 ETA 估算
// 4. 全局 DownloadManager 任务生命周期与持久化管理

use super::client::{download_range_stream, probe_url_meta, split_into_chunks};
use super::types::{DownloadTask, NewTaskParams, TaskStatus};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// 滑动窗口测速与 ETA 预估器
pub struct SpeedTracker {
    /// 历史采样队列: (采样时刻, 累计下载字节数)
    samples: VecDeque<(Instant, u64)>,
    /// 统计滑动窗口时长 (通常为 2.0 秒)
    window_duration: Duration,
}

impl SpeedTracker {
    /// 创建测速器，默认滑动窗口时长为 2 秒
    pub fn new(window_duration: Duration) -> Self {
        Self {
            samples: VecDeque::new(),
            window_duration,
        }
    }

    /// 记录当前累计字节数并计算瞬时速率 (字节/秒) 与剩余时间 (秒)
    pub fn update(&mut self, current_bytes: u64, total_bytes: u64) -> (u64, u64) {
        let now = Instant::now();
        self.samples.push_back((now, current_bytes));

        // 剔除超过滑动窗口时长的旧采样点，但保留至少 2 个点以计算差值
        while self.samples.len() > 2 {
            if let Some(&(first_time, _)) = self.samples.front() {
                if now.duration_since(first_time) > self.window_duration {
                    self.samples.pop_front();
                } else {
                    break;
                }
            }
        }

        if self.samples.len() < 2 {
            return (0, 0);
        }

        let (oldest_time, oldest_bytes) = *self.samples.front().unwrap();
        let duration = now.duration_since(oldest_time).as_secs_f64();
        if duration <= 0.05 {
            return (0, 0);
        }

        let delta_bytes = current_bytes.saturating_sub(oldest_bytes);
        let speed_bps = (delta_bytes as f64 / duration) as u64;

        let eta_seconds = if speed_bps > 0 && total_bytes > current_bytes {
            let remaining = total_bytes - current_bytes;
            let secs = (remaining as f64 / speed_bps as f64).ceil() as u64;
            secs.max(1)
        } else {
            0
        };

        (speed_bps, eta_seconds)
    }

    /// 重置测速采样窗口
    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

/// 获取 Windows 平台默认下载任务数据库存储路径 (%APPDATA%/OmniBox/download_tasks.json)
pub fn get_default_db_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("LOCALAPPDATA"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("OmniBox").join("download_tasks.json")
}

/// 获取 Windows 平台用户默认下载目录 ({USERPROFILE}\Downloads)
pub fn get_default_download_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let downloads = PathBuf::from(profile).join("Downloads");
        if downloads.exists() {
            return downloads;
        }
    }
    std::env::temp_dir()
}

/// 基于 Windows COM API 生成标准 GUID，若不可用则以时间戳兜底
pub fn generate_task_uuid() -> String {
    let mut guid = unsafe { std::mem::zeroed() };
    if unsafe { windows_sys::Win32::System::Com::CoCreateGuid(&mut guid) } == 0 {
        format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            guid.data1,
            guid.data2,
            guid.data3,
            guid.data4[0],
            guid.data4[1],
            guid.data4[2],
            guid.data4[3],
            guid.data4[4],
            guid.data4[5],
            guid.data4[6],
            guid.data4[7],
        )
    } else {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("task-{}", ts)
    }
}

/// 全局多线程下载管理器
pub struct DownloadManager {
    /// 任务列表集合 (线程安全)
    tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
    /// 任务取消/暂停中断信号映射表
    cancel_tokens: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// 任务清单持久化 JSON 文件路径
    db_path: PathBuf,
}

impl DownloadManager {
    /// 创建下载管理器，使用默认 %APPDATA%/OmniBox/download_tasks.json 持久化路径
    pub fn new() -> Result<Self, String> {
        Self::new_with_db_path(get_default_db_path())
    }

    /// 创建指定持久化数据库路径的下载管理器 (便于测试和隔离)
    pub fn new_with_db_path(db_path: PathBuf) -> Result<Self, String> {
        let manager = Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            db_path,
        };

        // 尝试从磁盘加载历史任务
        let _ = manager.load_tasks_from_disk();
        Ok(manager)
    }

    /// 获取任务持久化路径
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// 持久化保存当前所有任务到 JSON 文件
    pub fn save_tasks_to_disk(&self) -> Result<(), String> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建任务存储目录失败 [{}]: {}", parent.display(), e))?;
        }

        let tasks_guard = self.tasks.lock().map_err(|e| e.to_string())?;
        let task_list: Vec<DownloadTask> = tasks_guard.values().cloned().collect();
        drop(tasks_guard);

        let json_str = serde_json::to_string_pretty(&task_list)
            .map_err(|e| format!("序列化下载任务列表失败: {}", e))?;

        fs::write(&self.db_path, json_str)
            .map_err(|e| format!("写入下载任务数据库失败 [{}]: {}", self.db_path.display(), e))?;

        Ok(())
    }

    /// 从磁盘加载历史任务列表
    pub fn load_tasks_from_disk(&self) -> Result<(), String> {
        if !self.db_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.db_path)
            .map_err(|e| format!("读取下载任务数据库失败: {}", e))?;

        if content.trim().is_empty() {
            return Ok(());
        }

        let task_list: Vec<DownloadTask> = serde_json::from_str(&content)
            .map_err(|e| format!("反序列化下载任务数据库失败: {}", e))?;

        let mut tasks_guard = self.tasks.lock().map_err(|e| e.to_string())?;
        for mut task in task_list {
            // 如果历史任务在非正常退出前处于 Downloading，启动加载时自动重置为 Paused
            if task.status == TaskStatus::Downloading {
                task.status = TaskStatus::Paused;
                task.speed_bps = 0;
                task.eta_seconds = 0;
            }
            tasks_guard.insert(task.id.clone(), task);
        }

        Ok(())
    }

    /// 获取所有下载任务的快照列表
    pub fn get_tasks(&self) -> Vec<DownloadTask> {
        let tasks_guard = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let mut list: Vec<DownloadTask> = tasks_guard.values().cloned().collect();
        // 按创建时间升序排列
        list.sort_by_key(|t| t.created_at);
        list
    }

    /// 获取单个指定任务的快照
    pub fn get_task(&self, id: &str) -> Option<DownloadTask> {
        let tasks_guard = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks_guard.get(id).cloned()
    }

    /// 创建并启动新的下载任务
    pub fn create_task(&self, params: NewTaskParams) -> Result<DownloadTask, String> {
        // 1. 预探测 URL 获取文件总大小、建议文件名与 Range 支持
        let meta = probe_url_meta(&params.url, None)?;

        // 2. 确定保存目录与文件名
        let save_dir = match params.save_dir {
            Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()),
            _ => get_default_download_dir(),
        };

        let file_name = match params.file_name {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => meta.suggested_filename,
        };

        let save_path = save_dir.join(&file_name);
        let save_path_str = save_path.to_string_lossy().to_string();

        // 3. 确定线程数并切分分片
        let thread_count = params.threads.unwrap_or(4).clamp(1, 32);
        let chunks = if meta.supports_range && meta.total_bytes > 0 {
            split_into_chunks(meta.total_bytes, thread_count)
        } else {
            split_into_chunks(meta.total_bytes, 1)
        };

        let now_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let task = DownloadTask {
            id: generate_task_uuid(),
            url: params.url,
            file_name,
            save_path: save_path_str,
            total_bytes: meta.total_bytes,
            downloaded_bytes: 0,
            progress_percent: 0.0,
            speed_bps: 0,
            eta_seconds: 0,
            status: TaskStatus::Downloading,
            thread_count,
            supports_range: meta.supports_range,
            error_message: None,
            created_at: now_ts,
            chunks,
        };

        // 4. 登记任务与中断信号
        let cancel_token = Arc::new(AtomicBool::new(false));
        {
            let mut tokens = self.cancel_tokens.lock().map_err(|e| e.to_string())?;
            tokens.insert(task.id.clone(), Arc::clone(&cancel_token));

            let mut tasks_map = self.tasks.lock().map_err(|e| e.to_string())?;
            tasks_map.insert(task.id.clone(), task.clone());
        }

        // 保存数据库
        let _ = self.save_tasks_to_disk();

        // 5. 启动后台执行下载 Worker
        self.spawn_download_worker(&task.id, cancel_token)?;

        Ok(task)
    }

    /// 暂停正在下载的任务
    pub fn pause_task(&self, id: &str) -> Result<(), String> {
        let token = {
            let tokens = self.cancel_tokens.lock().map_err(|e| e.to_string())?;
            tokens.get(id).cloned()
        };

        if let Some(token) = token {
            token.store(true, Ordering::Relaxed);
        }

        // 更新任务状态为 Paused 并写回持久化与 .part.json
        let mut tasks_guard = self.tasks.lock().map_err(|e| e.to_string())?;
        if let Some(task) = tasks_guard.get_mut(id) {
            if task.status == TaskStatus::Downloading {
                task.status = TaskStatus::Paused;
                task.speed_bps = 0;
                task.eta_seconds = 0;
                let _ = save_part_file(task);
            }
        }
        drop(tasks_guard);

        let _ = self.save_tasks_to_disk();
        Ok(())
    }

    /// 继续暂停或失败的任务
    pub fn resume_task(&self, id: &str) -> Result<(), String> {
        let (task_exists, is_resumable) = {
            let tasks_guard = self.tasks.lock().map_err(|e| e.to_string())?;
            match tasks_guard.get(id) {
                Some(t) => (true, t.status == TaskStatus::Paused || t.status == TaskStatus::Failed),
                None => (false, false),
            }
        };

        if !task_exists {
            return Err(format!("任务不存在: {}", id));
        }

        if !is_resumable {
            return Ok(()); // 已经是下载中或已完成，无需处理
        }

        // 尝试从磁盘加载 .part.json 恢复最新游标
        {
            let mut tasks_guard = self.tasks.lock().map_err(|e| e.to_string())?;
            if let Some(task) = tasks_guard.get_mut(id) {
                let part_path = task.part_path();
                if part_path.exists() {
                    if let Ok(part_task) = load_part_file(&part_path) {
                        if part_task.total_bytes == task.total_bytes && part_task.chunks.len() == task.chunks.len() {
                            task.chunks = part_task.chunks;
                            task.refresh_progress_from_chunks();
                        }
                    }
                }
                task.status = TaskStatus::Downloading;
                task.error_message = None;
            }
        }

        let cancel_token = Arc::new(AtomicBool::new(false));
        {
            let mut tokens = self.cancel_tokens.lock().map_err(|e| e.to_string())?;
            tokens.insert(id.to_string(), Arc::clone(&cancel_token));
        }

        let _ = self.save_tasks_to_disk();
        self.spawn_download_worker(id, cancel_token)?;

        Ok(())
    }

    /// 取消或删除任务
    pub fn cancel_task(&self, id: &str, delete_file: bool) -> Result<(), String> {
        // 先发送中断信号
        let token = {
            let mut tokens = self.cancel_tokens.lock().map_err(|e| e.to_string())?;
            tokens.remove(id)
        };
        if let Some(token) = token {
            token.store(true, Ordering::Relaxed);
        }

        let task_opt = {
            let mut tasks_guard = self.tasks.lock().map_err(|e| e.to_string())?;
            if delete_file {
                tasks_guard.remove(id)
            } else if let Some(task) = tasks_guard.get_mut(id) {
                task.status = TaskStatus::Cancelled;
                task.speed_bps = 0;
                task.eta_seconds = 0;
                Some(task.clone())
            } else {
                None
            }
        };

        if let Some(task) = task_opt {
            if delete_file {
                let downloading = task.downloading_path();
                if downloading.exists() {
                    let _ = fs::remove_file(downloading);
                }
                let part = task.part_path();
                if part.exists() {
                    let _ = fs::remove_file(part);
                }
                let final_path = PathBuf::from(&task.save_path);
                if final_path.exists() {
                    let _ = fs::remove_file(final_path);
                }
            }
        }

        let _ = self.save_tasks_to_disk();
        Ok(())
    }

    /// 后台启动执行任务的多线程 Seek 写入与测速协调主循环
    fn spawn_download_worker(&self, task_id: &str, cancel_token: Arc<AtomicBool>) -> Result<(), String> {
        let task_id = task_id.to_string();
        let tasks_arc = Arc::clone(&self.tasks);
        let db_path = self.db_path.clone();

        thread::spawn(move || {
            execute_download_coordinator(task_id, tasks_arc, cancel_token, db_path);
        });

        Ok(())
    }
}

/// 保存 .part.json 伴生断点续传元数据文件
pub fn save_part_file(task: &DownloadTask) -> Result<(), String> {
    let part_path = task.part_path();
    if let Some(parent) = part_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string(task)
        .map_err(|e| format!("序列化 .part.json 失败: {}", e))?;
    fs::write(&part_path, json)
        .map_err(|e| format!("写入 .part.json 失败 [{}]: {}", part_path.display(), e))?;
    Ok(())
}

/// 从磁盘读取 .part.json 伴生断点续传元数据文件
pub fn load_part_file(part_path: &Path) -> Result<DownloadTask, String> {
    let content = fs::read_to_string(part_path)
        .map_err(|e| format!("读取 .part.json 失败 [{}]: {}", part_path.display(), e))?;
    let task: DownloadTask = serde_json::from_str(&content)
        .map_err(|e| format!("反序列化 .part.json 失败: {}", e))?;
    Ok(task)
}

/// 核心下载协调器逻辑：负责文件预分配、启动并发 Range 下载 Worker、直接 Seek 写入、测速聚合与最终重命名结算
fn execute_download_coordinator(
    task_id: String,
    tasks_arc: Arc<Mutex<HashMap<String, DownloadTask>>>,
    cancel_token: Arc<AtomicBool>,
    db_path: PathBuf,
) {
    // 1. 获取任务初始信息
    let initial_task = {
        let tasks_guard = tasks_arc.lock().unwrap();
        match tasks_guard.get(&task_id) {
            Some(t) => t.clone(),
            None => return,
        }
    };

    let downloading_path = initial_task.downloading_path();
    let final_path = PathBuf::from(&initial_task.save_path);

    // 确保目标目录存在
    if let Some(parent) = downloading_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            fail_task(&task_id, &tasks_arc, &format!("创建下载保存目录失败: {}", e));
            return;
        }
    }

    // 2. 尝试从伴生 .part.json 恢复未完成的进度游标
    let part_path = initial_task.part_path();
    if part_path.exists() && downloading_path.exists() {
        if let Ok(saved_part) = load_part_file(&part_path) {
            if saved_part.total_bytes == initial_task.total_bytes && saved_part.chunks.len() == initial_task.chunks.len() {
                let mut guard = tasks_arc.lock().unwrap();
                if let Some(current) = guard.get_mut(&task_id) {
                    current.chunks = saved_part.chunks;
                    current.refresh_progress_from_chunks();
                }
            }
        }
    }

    // 3. 准备目标零碎片临时文件与预分配空间
    let file = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&downloading_path)
    {
        Ok(f) => f,
        Err(e) => {
            fail_task(&task_id, &tasks_arc, &format!("打开临时下载文件失败: {}", e));
            return;
        }
    };

    // 若已知总大小，直接预分配物理空间以实现零碎片连续存储
    if initial_task.total_bytes > 0 {
        let curr_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if curr_len < initial_task.total_bytes {
            if let Err(e) = file.set_len(initial_task.total_bytes) {
                fail_task(&task_id, &tasks_arc, &format!("预分配文件大小失败: {}", e));
                return;
            }
        }
    }

    // 使用 Arc<Mutex<File>> 互斥文件句柄共享给各 Worker 线程安全 Seek 写入
    let shared_file = Arc::new(Mutex::new(file));

    // 4. 获取当前任务最新分片列表
    let (url, supports_range, total_bytes, chunks) = {
        let guard = tasks_arc.lock().unwrap();
        let current = match guard.get(&task_id) {
            Some(t) => t,
            None => return,
        };
        (current.url.clone(), current.supports_range, current.total_bytes, current.chunks.clone())
    };

    // 5. 分支执行：Range 多线程并发拉取 vs 单线程降级
    let mut worker_handles = Vec::new();
    let worker_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    if supports_range && total_bytes > 0 {
        // 多线程并发 Range 下载
        for chunk in chunks {
            if chunk.is_finished || chunk.downloaded >= (chunk.end.saturating_sub(chunk.start) + 1) {
                continue;
            }

            let c_url = url.clone();
            let c_cancel = Arc::clone(&cancel_token);
            let c_file = Arc::clone(&shared_file);
            let c_tasks = Arc::clone(&tasks_arc);
            let c_task_id = task_id.clone();
            let c_errors = Arc::clone(&worker_errors);

            let handle = thread::spawn(move || {
                let start_offset = chunk.start + chunk.downloaded;
                let end_offset = chunk.end;

                if start_offset > end_offset {
                    return;
                }

                let range = Some((start_offset, end_offset));
                let mut current_offset = start_offset;

                let stream_res = download_range_stream(
                    &c_url,
                    range,
                    None,
                    &c_cancel,
                    |chunk_data| {
                        // 互斥写入目标文件对应区间
                        {
                            let mut f = c_file.lock().map_err(|e| format!("文件锁互斥失败: {}", e))?;
                            f.seek(SeekFrom::Start(current_offset))
                                .map_err(|e| format!("文件 Seek 失败: {}", e))?;
                            f.write_all(chunk_data)
                                .map_err(|e| format!("文件写入失败: {}", e))?;
                        }

                        current_offset += chunk_data.len() as u64;

                        // 更新分片进度
                        let mut guard = c_tasks.lock().unwrap();
                        if let Some(t) = guard.get_mut(&c_task_id) {
                            if let Some(item) = t.chunks.iter_mut().find(|c| c.id == chunk.id) {
                                item.downloaded = current_offset.saturating_sub(item.start);
                                if item.downloaded >= (item.end.saturating_sub(item.start) + 1) {
                                    item.is_finished = true;
                                }
                            }
                            t.refresh_progress_from_chunks();
                        }

                        Ok(())
                    },
                );

                if let Err(err) = stream_res {
                    if err != "download_cancelled" {
                        let mut errs = c_errors.lock().unwrap();
                        errs.push(format!("分片 {} 下载失败: {}", chunk.id, err));
                    }
                }
            });

            worker_handles.push(handle);
        }
    } else {
        // 单线程降级流式拉取
        let c_url = url.clone();
        let c_cancel = Arc::clone(&cancel_token);
        let c_file = Arc::clone(&shared_file);
        let c_tasks = Arc::clone(&tasks_arc);
        let c_task_id = task_id.clone();
        let c_errors = Arc::clone(&worker_errors);

        let initial_downloaded = chunks.first().map(|c| c.downloaded).unwrap_or(0);

        let handle = thread::spawn(move || {
            let mut current_offset = initial_downloaded;
            let range = if initial_downloaded > 0 {
                // 若已有部分下载且尝试单线程续传
                Some((initial_downloaded, total_bytes.saturating_sub(1)))
            } else {
                None
            };

            let stream_res = download_range_stream(
                &c_url,
                range,
                None,
                &c_cancel,
                |chunk_data| {
                    {
                        let mut f = c_file.lock().map_err(|e| format!("文件锁互斥失败: {}", e))?;
                        f.seek(SeekFrom::Start(current_offset))
                            .map_err(|e| format!("文件 Seek 失败: {}", e))?;
                        f.write_all(chunk_data)
                            .map_err(|e| format!("文件写入失败: {}", e))?;
                    }

                    current_offset += chunk_data.len() as u64;

                    let mut guard = c_tasks.lock().unwrap();
                    if let Some(t) = guard.get_mut(&c_task_id) {
                        t.downloaded_bytes = current_offset;
                        if t.total_bytes > 0 {
                            let pct = (current_offset as f64 / t.total_bytes as f64) * 100.0;
                            t.progress_percent = (pct * 100.0).round() / 100.0;
                        }
                        if let Some(item) = t.chunks.first_mut() {
                            item.downloaded = current_offset;
                            if t.total_bytes > 0 && current_offset >= t.total_bytes {
                                item.is_finished = true;
                            }
                        }
                    }

                    Ok(())
                },
            );

            if let Err(err) = stream_res {
                if err != "download_cancelled" {
                    let mut errs = c_errors.lock().unwrap();
                    errs.push(format!("流式下载失败: {}", err));
                }
            }
        });

        worker_handles.push(handle);
    }

    // 6. 主协调监控与测速循环
    let mut speed_tracker = SpeedTracker::new(Duration::from_secs(2));
    let mut last_part_save = Instant::now();

    loop {
        // 检查 Worker 线程是否全部退出
        let mut any_alive = false;
        for h in &worker_handles {
            if !h.is_finished() {
                any_alive = true;
                break;
            }
        }

        // 刷新测速与持久化状态
        {
            let mut guard = tasks_arc.lock().unwrap();
            if let Some(task) = guard.get_mut(&task_id) {
                let (speed, eta) = speed_tracker.update(task.downloaded_bytes, task.total_bytes);
                task.speed_bps = speed;
                task.eta_seconds = eta;

                // 每 500ms 自动保存一次 .part.json 伴生断点文件
                if last_part_save.elapsed() >= Duration::from_millis(500) {
                    let _ = save_part_file(task);
                    last_part_save = Instant::now();
                }
            }
        }

        if !any_alive {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    // 等待所有 Worker 线程完全退出并收集
    for h in worker_handles {
        let _ = h.join();
    }

    // 显式释放文件句柄，保证 Windows 文件系统解除锁定
    drop(shared_file);

    // 7. 结算分析：判断是取消暂停、出错还是成功完成
    if cancel_token.load(Ordering::Relaxed) {
        // 用户暂停或取消
        let mut guard = tasks_arc.lock().unwrap();
        if let Some(task) = guard.get_mut(&task_id) {
            if task.status == TaskStatus::Downloading {
                task.status = TaskStatus::Paused;
            }
            task.speed_bps = 0;
            task.eta_seconds = 0;
            let _ = save_part_file(task);
        }
        let _ = persist_tasks_db(&tasks_arc, &db_path);
        return;
    }

    // 检查是否有 Worker 抛出错误
    let errors = worker_errors.lock().unwrap().clone();
    if !errors.is_empty() {
        let err_msg = errors.join("; ");
        fail_task(&task_id, &tasks_arc, &err_msg);
        let _ = persist_tasks_db(&tasks_arc, &db_path);
        return;
    }

    // 8. 验证所有分片是否全部下载完毕
    let (is_complete, final_downloaded) = {
        let mut guard = tasks_arc.lock().unwrap();
        if let Some(task) = guard.get_mut(&task_id) {
            task.refresh_progress_from_chunks();
            let all_done = if task.supports_range && task.total_bytes > 0 {
                task.chunks.iter().all(|c| c.is_finished)
            } else if task.total_bytes > 0 {
                task.downloaded_bytes >= task.total_bytes
            } else {
                task.downloaded_bytes > 0
            };
            (all_done, task.downloaded_bytes)
        } else {
            (false, 0)
        }
    };

    if is_complete {
        // 重命名临时文件为最终目标文件
        if final_path.exists() {
            let _ = fs::remove_file(&final_path);
        }

        if let Err(e) = fs::rename(&downloading_path, &final_path) {
            fail_task(&task_id, &tasks_arc, &format!("完成重命名文件失败: {}", e));
            let _ = persist_tasks_db(&tasks_arc, &db_path);
            return;
        }

        // 安全删除伴生 .part.json
        if part_path.exists() {
            let _ = fs::remove_file(&part_path);
        }

        // 更新状态为已完成
        {
            let mut guard = tasks_arc.lock().unwrap();
            if let Some(task) = guard.get_mut(&task_id) {
                task.status = TaskStatus::Completed;
                task.progress_percent = 100.0;
                task.downloaded_bytes = if task.total_bytes > 0 { task.total_bytes } else { final_downloaded };
                task.speed_bps = 0;
                task.eta_seconds = 0;
                task.error_message = None;
                for c in &mut task.chunks {
                    c.is_finished = true;
                }
            }
        }

        let _ = persist_tasks_db(&tasks_arc, &db_path);
    } else {
        // 未完全完成且无 cancel 信号，标记为 Failed
        fail_task(&task_id, &tasks_arc, "下载未完全完成，连接提前终止");
        let _ = persist_tasks_db(&tasks_arc, &db_path);
    }
}

/// 标记任务为失败
fn fail_task(task_id: &str, tasks_arc: &Arc<Mutex<HashMap<String, DownloadTask>>>, error_msg: &str) {
    let mut guard = tasks_arc.lock().unwrap();
    if let Some(task) = guard.get_mut(task_id) {
        task.status = TaskStatus::Failed;
        task.speed_bps = 0;
        task.eta_seconds = 0;
        task.error_message = Some(error_msg.to_string());
        let _ = save_part_file(task);
    }
}

/// 持久化任务清单到数据库 JSON 文件
fn persist_tasks_db(tasks_arc: &Arc<Mutex<HashMap<String, DownloadTask>>>, db_path: &Path) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let guard = tasks_arc.lock().unwrap();
    let task_list: Vec<DownloadTask> = guard.values().cloned().collect();
    drop(guard);

    let json_str = serde_json::to_string_pretty(&task_list)
        .map_err(|e| format!("序列化任务列表失败: {}", e))?;
    fs::write(db_path, json_str)
        .map_err(|e| format!("写入任务数据库失败: {}", e))?;
    Ok(())
}

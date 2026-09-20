# Windows 高速多线程下载器 (TurboDownloader) 架构与设计规范

## 1. 目标与定位

为 OmniBox 万能工具箱打造一个对标 **PCL2 启动器** 与 **FDM (Free Download Manager)** 的现代化超高速、低资源占用多线程下载器。

- **核心原则**：
  1. 采用 Windows 原生系统级 `WinHTTP` API，纯 Rust 安全封装，免去第三方庞大 TLS/HTTP 依赖，零额外体积开销；
  2. 智能分块并发拉取（支持 1~32 线程可调节），并直接通过文件句柄 `Seek` 并行随机写入同一个目标文件，**彻底根除下载完成后漫长二次合并碎片的 CPU 与磁盘双倍损耗**；
  3. 支持断点续传（通过同目录 `.part.json` 元数据持久化各分片游标）；
  4. 实时滑动窗口测速、剩余时间预估 (ETA)、分片状态热力图可视化（FDM 经典彩色进度格展示）；
  5. 视觉完全对齐 Windows 11 Fluent 亚克力磨砂设计规范。

---

## 2. 核心架构与数据流

```text
[用户输入 URL / 参数 / 保存路径]
            |
            v
[Tauri Command: create_download_task]
            |
            v
[探测阶段: WinHTTP HEAD / GET (bytes=0-0)]
  -> 检查状态码 200/206
  -> 提取 Content-Length (总大小)
  -> 提取 Accept-Ranges
  -> 解析建议文件名 (Content-Disposition 或 URL 尾部)
            |
    +-------+-------+
    |               |
[支持 Range]    [不支持 Range / 大小未知]
    |               |
    v               v
均匀切分 N 个分块   单线程流式下载
(每个分片记录 start, end, downloaded)
    |
    v
预分配文件大小 (SetFileValidData / SetEndOfFile)
创建预分配临时文件 *.downloading
            |
            v
并发 Worker 线程池拉取各自 Range
每个分块读取到内存 Buffer 后，互斥写入同一个文件句柄 (Seek + Write)
            |
            v
状态管理器每 250ms 聚合进度、计算速度、保存 *.part.json
            |
            v
所有分片完成后：
校验大小 -> 移除 *.part.json -> 重命名 *.downloading 为最终目标文件
```

---

## 3. 数据模型设计 (Rust & TypeScript 对齐)

### 3.1 分块状态 (`DownloadChunk`)
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DownloadChunk {
    pub id: usize,            // 分片序号 (0..N-1)
    pub start: u64,           // 起始字节偏移
    pub end: u64,             // 结束字节偏移 (包含)
    pub downloaded: u64,      // 当前分块已下载字节数
    pub is_finished: bool,    // 当前分块是否已完成
}
```

### 3.2 任务状态 (`DownloadTask`)
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,     // 等待中 / 初始化探测中
    Downloading, // 正在高速下载
    Paused,      // 用户手动暂停
    Completed,   // 下载成功完成
    Failed,      // 下载失败 (包含错误说明)
    Cancelled,   // 已取消
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadTask {
    pub id: String,                 // 任务 UUID
    pub url: String,                // 下载目标链接
    pub file_name: String,          // 保存文件名
    pub save_path: String,          // 完整存储绝对路径
    pub total_bytes: u64,           // 文件总大小 (0 表示未知)
    pub downloaded_bytes: u64,      // 当前已累计下载字节数
    pub progress_percent: f64,      // 下载进度百分比 (0.0 ~ 100.0)
    pub speed_bps: u64,             // 瞬时下载速率 (字节/秒)
    pub eta_seconds: u64,           // 预估剩余时间 (秒)
    pub status: TaskStatus,         // 当前任务生命周期状态
    pub thread_count: usize,        // 分块并发线程数 (1~32)
    pub supports_range: bool,       // 服务端是否支持分块断点续传
    pub error_message: Option<String>,
    pub created_at: u64,            // 创建时间戳
    pub chunks: Vec<DownloadChunk>, // 各分片当前进度状态 (供 FDM 热力图可视化)
}
```

---

## 4. 关键技术细节与边界防护

1. **WinHTTP 安全封装**：
   - 自动处理 HTTP 重定向 (301, 302, 307, 308)；
   - 支持自定义 `User-Agent` 与 `Referer`；
   - 严格设置连接、发送与接收超时（避免网络挂死）；
   - 对非法非回环或异常网络错误安全捕获，抛出结构化错误信息。
2. **零碎片文件写入**：
   - 打开文件使用 Windows 标准读写共享模式 (`FILE_SHARE_READ | FILE_SHARE_WRITE`)；
   - 写入时线程根据 `chunk.start + chunk.downloaded` 直接 `seek` 并执行写入；
   - 不需要分片完成后再读取所有小文件拼接，瞬时完成重命名，极度适合几十 MB 至上百 GB 的大型文件高速下载。
3. **断点续传机制**：
   - 临时文件命名格式为 `[目标文件名].downloading`，同目录伴生 `[目标文件名].part.json`；
   - 任务暂停或断网后，下次启动直接读取 `.part.json`，检查各 chunk 的 `downloaded`，请求 `Range: bytes={start+downloaded}-{end}` 继续拉取。
4. **FDM 经典分片热力图可视化**：
   - 前端接收各分块的 `downloaded / (end - start + 1)` 比例；
   - 渲染包含 N 个细分小格子的进度指示栏，直观展示哪些块已经填满、哪些块正在脉冲传输。

---

## 5. UI 与前端集成设计

- 模块挂载在 `ToolRegistry` 中（分类：`efficiency` 高效工具，ID：`turbo-downloader`，标题：`高速下载器`，图标：`DownloadCloud`）；
- **操作面板**：
  - 顶部统计卡片：当前任务数、累计完成量、全速总吞吐量；
  - 「新建下载」主按钮：呼出 Fluent 模态框，输入 URL 自动提取文件名、自选保存路径、滑块选择 1~32 线程；
  - 任务卡片列表：包含文件图标、名称、大小、瞬时速度、ETA、主进度条、FDM 分片热力图矩阵、暂停/继续/打开文件/打开文件夹/删除按钮。

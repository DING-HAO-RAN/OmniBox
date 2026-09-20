# 高速多线程下载器 (TurboDownloader) 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个对标 PCL2 / FDM 的 Windows 高速多线程下载器，基于 Windows 原生 WinHTTP 实现智能分块、零碎片直接 Seek 并行写入同一个文件、断点续传及经典 FDM 分片热力图可视化。

**Architecture:** 后端利用 WinHTTP 原生网络栈执行分片并发拉取并直接多线程 Seek 写入预分配目标文件，杜绝后续合并碎片开销；同目录持久化 `.part.json` 记录游标断点续传；前端利用 Vue 3 + Tailwind CSS 打造 Windows 11 Fluent 风格界面并提供彩色分片进度矩阵。

**Tech Stack:** Tauri v2, Rust (windows-sys / WinHTTP / Threading / FileSystem), Vue 3, Vite, TypeScript, Tailwind CSS, Lucide Icons.

**Spec:** `docs/superpowers/specs/2026-09-20-multi-threaded-downloader-design.md`

## Global Constraints

- 操作系统支持：Windows 10 / Windows 11 (x64)
- 零额外重量级依赖：优先使用 Windows 原生 WinHTTP API，不引入第三方不可控的外部二进制
- 零碎片损耗：大文件直接 Seek 并发写入目标预分配文件，禁止先下载分片再二次合并
- 风格一致性：遵循 Windows 11 Fluent 磨砂与卡片拟态设计
- 无过度工程化：简单直接，聚焦下载核心功能与稳定性

---

### Task 1: Rust WinHTTP 客户端封装、分块算法与核心数据结构

**Files:**
- Create: `src-tauri/src/downloader/types.rs`
- Create: `src-tauri/src/downloader/client.rs`
- Create: `src-tauri/src/downloader/mod.rs`
- Test: `src-tauri/src/downloader/tests.rs`

**Interfaces:**
- Consumes: Win32 `Win32_Networking_WinHttp` API
- Produces:
  - `pub fn probe_url_meta(url: &str, custom_headers: Option<&[(&str, &str)]>) -> Result<UrlMeta, String>`
  - `pub fn split_into_chunks(total_bytes: u64, thread_count: usize) -> Vec<DownloadChunk>`
  - `pub fn extract_filename_from_url(url: &str, disposition: Option<&str>) -> String`

- [ ] **Step 1: 编写数据结构 `DownloadChunk`, `DownloadTask`, `TaskStatus`, `UrlMeta`**
- [ ] **Step 2: 编写单元测试验证分片切分算法与文件名解析**
- [ ] **Step 3: 运行测试并验证初始失败状态 (TDD)**
- [ ] **Step 4: 实现 WinHTTP 网络请求与 HEAD 探测逻辑**
- [ ] **Step 5: 运行 `cargo test` 确保切分与探测逻辑全绿**
- [ ] **Step 6: 提交基础代码**

---

### Task 2: 零碎片并行写入下载引擎、断点续传与任务管理器

**Files:**
- Create: `src-tauri/src/downloader/engine.rs`
- Modify: `src-tauri/src/downloader/mod.rs`
- Test: `src-tauri/src/downloader/tests.rs`

**Interfaces:**
- Consumes: `downloader::client`, `downloader::types`
- Produces:
  - `pub struct DownloadManager`
  - `DownloadManager::create_task(params: NewTaskParams) -> Result<DownloadTask, String>`
  - `DownloadManager::pause_task(id: &str) -> Result<(), String>`
  - `DownloadManager::resume_task(id: &str) -> Result<(), String>`
  - `DownloadManager::cancel_task(id: &str, delete_file: bool) -> Result<(), String>`
  - `DownloadManager::get_tasks() -> Vec<DownloadTask>`

- [ ] **Step 1: 编写多线程 Range 下载 Worker 与直接文件 Seek 写入逻辑**
- [ ] **Step 2: 编写伴生 `.part.json` 任务游标持久化与断点续传状态恢复**
- [ ] **Step 3: 编写滑动窗口测速器与 ETA 预估器**
- [ ] **Step 4: 编写多线程下载与断点续传单元测试**
- [ ] **Step 5: 运行 `cargo test` 验证通过**
- [ ] **Step 6: 提交引擎代码**

---

### Task 3: Tauri Commands 暴露与应用状态注入

**Files:**
- Create: `src-tauri/src/downloader/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`

**Interfaces:**
- Produces Tauri Commands:
  - `downloader_create_task(url: String, save_dir: Option<String>, file_name: Option<String>, threads: Option<usize>)`
  - `downloader_list_tasks()`
  - `downloader_pause_task(id: String)`
  - `downloader_resume_task(id: String)`
  - `downloader_delete_task(id: String, delete_file: bool)`
  - `downloader_choose_dir()`
  - `downloader_open_file(path: String)`
  - `downloader_open_folder(path: String)`

- [ ] **Step 1: 编写 Tauri Command 接口函数**
- [ ] **Step 2: 在 Tauri App State 中注入全局共享的 `DownloadManager` 实例**
- [ ] **Step 3: 在 `lib.rs` 中完整注册全部命令**
- [ ] **Step 4: 运行 `npm run check:rust` 验证无警告编译**
- [ ] **Step 5: 提交命令层代码**

---

### Task 4: 前端 Vue 3 Fluent UI 界面与 FDM 经典分片热力图

**Files:**
- Create: `src/views/TurboDownloaderView.vue`
- Create: `src/components/downloader/NewDownloadModal.vue`
- Create: `src/components/downloader/ChunkHeatmap.vue`
- Modify: `src/types/module.ts`
- Modify: `src/registry/index.ts`
- Modify: `src/components/layout/Sidebar.vue`

**Interfaces:**
- Consumes: Tauri invoke 命令
- Produces: 现代 Fluent 风格的多线程下载器控制中心与热力图

- [ ] **Step 1: 声明前端 TypeScript 接口 (`DownloadTask`, `DownloadChunk` 等)**
- [ ] **Step 2: 开发 `ChunkHeatmap.vue` (FDM 经典分片矩阵与彩色进度指示)**
- [ ] **Step 3: 开发 `NewDownloadModal.vue` (URL 输入、文件名解析、保存路径与线程数滑块)**
- [ ] **Step 4: 开发 `TurboDownloaderView.vue` (顶部统计、任务卡片列表、控制按钮、定时轮询)**
- [ ] **Step 5: 在 `ToolRegistry` 与侧边栏中注册模块**
- [ ] **Step 6: 运行 `npm run build` 确保类型检查与 Vite 打包通过**
- [ ] **Step 7: 提交前端界面代码**

---

### Task 5: 全量构建、自动化测试、EXE 打包与功能验证

**Files:**
- Test: 全量单元测试与端到端测试
- Build: `OmniBox-万能工具箱.exe`

- [ ] **Step 1: 执行全量 `cargo test` (确保所有现有测试与新增下载器测试 100% 通过)**
- [ ] **Step 2: 执行 `npm run build`**
- [ ] **Step 3: 执行 `npx tauri build --no-bundle` 生成最新绿色版单文件 EXE**
- [ ] **Step 4: 验证启动运行与下载测试**
- [ ] **Step 5: 提交全部最终代码并检查 Git 状态**

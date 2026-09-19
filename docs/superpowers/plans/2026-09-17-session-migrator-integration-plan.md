# 跨设备浏览器会话迁移与免密登录器 (Session Migrator) 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `D:\桌面\项目‘\bilibili登录器` 的完整功能 1:1 整合进 OmniBox 桌面万能工具箱中，保持所有配置项、按钮和操作流程完全一致，且 UI 风格与整体 Windows 11 Fluent Design 深度契合。

**Architecture:** 后端利用 Rust 管理独立 Chrome/Edge 进程的生命周期，通过 CDP 协议进行高可靠 Cookie 提取与注入，结合标准 AES-256-GCM / PBKDF2 密码学引擎处理 `bsm/1` 与 `bsm/2` 加密包；前端基于 Vue 3 + Tailwind CSS 构建 1:1 的 Fluent UI 双标签页控制台。

**Tech Stack:** Tauri v2, Rust (windows-sys / Process / IO / Crypto / HTTP), Vue 3, Vite, TypeScript, Tailwind CSS, Lucide Icons.

**Spec:** `docs/superpowers/specs/2026-09-17-session-migrator-integration-design.md`

## Global Constraints

- 操作系统支持：Windows 10 / Windows 11 (x64)
- 零隐私泄露：绝不收集或保存明文 Cookie、密码、Token；导出的加密文件采用强加密保护
- 保持原项目操作流程不变：浏览器选择、端口、独立配置、启动/停止 CDP、生成脚本、导出标准文件/扩展文件、导入已选文件、打开目录
- 美术与 UI 一致性：Windows 11 Fluent 亚克力磨砂质感、电光蓝/粉紫点缀色、柔和微动效
- 无过度工程化：简单直接，聚焦核心需求

---

### Task 1: Rust 底层浏览器探测、本地 CDP 启动控制与脚本生成 (`session_migrator/cdp.rs`)

**Files:**
- Create: `src-tauri/src/session_migrator/mod.rs`
- Create: `src-tauri/src/session_migrator/launcher.rs`
- Create: `src-tauri/src/session_migrator/types.rs`
- Modify: `src-tauri/src/system/mod.rs` or `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: Windows 系统安装路径探测与子进程管理
- Produces:
  - `pub fn find_browser_executable(browser: &str) -> Result<PathBuf, String>`
  - `pub fn get_default_profile_dir(browser: &str) -> PathBuf`
  - `pub fn launch_local_cdp(browser: &str, port: u16, profile_dir: &str) -> Result<RunningBrowser, String>`
  - `pub fn stop_local_cdp() -> Result<(), String>`
  - `pub fn write_profile_launcher_cmd(browser: &str, port: u16, profile_dir: &str, dest_path: &str) -> Result<(), String>`
  - `pub fn choose_directory_dialog() -> Option<String>`

- [ ] **Step 1: 编写数据模型与浏览器探测逻辑**
- [ ] **Step 2: 实现带独立 profile 与 `--remote-debugging-address=127.0.0.1` 的进程启动与版本探针**
- [ ] **Step 3: 实现专属 `.cmd` 启动脚本生成与进程停止**
- [ ] **Step 4: 编译检查验证**

---

### Task 2: 会话包数据模型、加密解密引擎与 CDP Cookie 提取/注入 (`session_migrator/engine.rs`)

**Files:**
- Create: `src-tauri/src/session_migrator/crypto.rs`
- Create: `src-tauri/src/session_migrator/cookie_engine.rs`

**Interfaces:**
- Consumes: CDP HTTP/WebSocket 协议、AES-GCM / PBKDF2 算法
- Produces:
  - `pub fn extract_cookies_from_cdp(port: u16, domain: &str, site_url: &str, include_subdomains: bool) -> Result<SessionBundle, String>`
  - `pub fn inject_cookies_to_cdp(port: u16, bundle: &SessionBundle, domain: &str, site_url: &str, include_subdomains: bool) -> Result<usize, String>`
  - `pub fn export_encrypted_session_file(bundle: &SessionBundle, password: &str, format: &str, out_path: &str) -> Result<String, String>`
  - `pub fn import_encrypted_session_file(file_path: &str, password: &str) -> Result<SessionBundle, String>`

- [ ] **Step 1: 实现会话包与 Cookie 清洗/规范化逻辑**
- [ ] **Step 2: 实现符合 `bsm/1` 与 `bsm/2` 规范的 PBKDF2/AES-256-GCM 加密与解密**
- [ ] **Step 3: 实现通过 CDP 接口的 Cookie 提取与注入导航**
- [ ] **Step 4: 编写测试用例验证加解密往返与 Cookie 域名过滤**

---

### Task 3: Tauri Commands 暴露与目录快捷管理

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces Tauri Commands:
  - `cdp_launch_browser(browser: String, port: u16, profile_dir: String)`
  - `cdp_stop_browser()`
  - `cdp_create_desktop_launcher(browser: String, port: u16, profile_dir: String)`
  - `cdp_choose_profile_dir()`
  - `cdp_export_session(...)`
  - `cdp_import_session(...)`
  - `cdp_get_sessions_dir()`
  - `cdp_open_sessions_dir()`
  - `cdp_choose_session_file()`

- [ ] **Step 1: 在 `commands.rs` 中编写封装命令**
- [ ] **Step 2: 在 `lib.rs` 中完整注册**
- [ ] **Step 3: 执行 `cargo check` 与 `cargo test` 验证**

---

### Task 4: 前端 Vue 3 Fluent UI 完整视图开发 (`SessionMigratorView.vue`)

**Files:**
- Create: `src/views/SessionMigratorView.vue`
- Create: `src/components/session-migrator/PasswordModal.vue`
- Modify: `src/types/module.ts`
- Modify: `src/registry/index.ts`
- Modify: `src/components/layout/Sidebar.vue`

**Interfaces:**
- Consumes: Tauri invoke 命令
- Produces: 1:1 还原原项目界面的现代 Fluent 视图组件

- [ ] **Step 1: 声明前端 TypeScript 接口**
- [ ] **Step 2: 编写口令输入/确认模态弹窗 `PasswordModal.vue`**
- [ ] **Step 3: 编写 1:1 操作布局的 `SessionMigratorView.vue` (连接配置、导出页、导入页、状态条、安全提示)**
- [ ] **Step 4: 接入 `ToolRegistry` 与侧边栏图标**
- [ ] **Step 5: 验证 `npm run build`**

---

### Task 5: 端到端全量构建、测试与 EXE 打包

**Files:**
- Build: `OmniBox-万能工具箱.exe`
- Modify: `README.md`

- [ ] **Step 1: 执行全量前端与后端类型测试与单元测试**
- [ ] **Step 2: 执行 `npx tauri build --no-bundle` 生成独立绿色版 EXE**
- [ ] **Step 3: 验证启动与无缝功能**

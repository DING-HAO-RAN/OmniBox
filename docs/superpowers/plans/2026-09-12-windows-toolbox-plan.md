# Windows 现代化万能工具箱 (OmniBox) 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个超低资源占用、轻量可扩展、拥有 Windows 11 Fluent 现代化质感的桌面端万能工具箱，集成多进程快速启动器与系统运行内存优化清理两大核心功能。

**Architecture:** 采用 Tauri v2 作为桌面容器（复用系统 WebView2 保证 30MB 内低占用），前端使用 Vue 3 + TypeScript + Tailwind CSS 构建模块化插件式外壳，后端使用 Rust 直接调用 Win32 原生 API 实现精准的进程静默启动与内存工作集修剪。

**Tech Stack:** Tauri v2, Rust (windows-sys / winapi / sysinfo), Vue 3, Vite, TypeScript, Tailwind CSS, Lucide-Vue-Next.

**Spec:** `docs/superpowers/specs/2026-09-12-windows-toolbox-design.md`

## Global Constraints

- 操作系统支持：Windows 10 / Windows 11 (x64)
- 零隐私泄露：不硬编码任何开发者的本地绝对路径，配置文件存放于应用安全存储
- 低资源开销：空载驻留内存严格控制在 30MB 左右
- 风格一致性：遵循 Windows 11 Fluent 磨砂与卡片拟态设计
- 无过度工程化：简单直接，聚焦核心需求

---

### Task 1: 项目脚手架与基础工程初始化

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `index.html`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: Node.js, Cargo, Tauri CLI
- Produces: 完整可编译的 Tauri v2 + Vue 3 基础工程

- [ ] **Step 1: 创建 package.json 与前端工程配置文件**
配置 Vue 3, Vite, Tailwind CSS, lucide-vue-next, @tauri-apps/api, @tauri-apps/plugin-shell 等必要依赖。

- [ ] **Step 2: 创建 src-tauri 配置与 Cargo.toml**
引入 `tauri`, `serde`, `serde_json`, `windows-sys` (包含 Process/Threading/Memory API) 等核心依赖。

- [ ] **Step 3: 安装依赖并验证脚手架配置**
运行 `npm install` 并运行 `cargo check` 确认脚手架结构就绪。

- [ ] **Step 4: 提交基础脚手架代码**
```bash
git add .
git commit -m "chore: initialize Tauri v2 and Vue 3 project scaffold"
```

---

### Task 2: Rust 底层系统 API 模块开发与测试 (进程控制与内存清理)

**Files:**
- Create: `src-tauri/src/system/mod.rs`
- Create: `src-tauri/src/system/process.rs`
- Create: `src-tauri/src/system/memory.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/system/tests.rs`

**Interfaces:**
- Consumes: Windows API (`GlobalMemoryStatusEx`, `EmptyWorkingSet`, `CreateProcessW`)
- Produces: 
  - `pub fn get_memory_info() -> Result<MemoryStatus, String>`
  - `pub fn clean_process_working_sets() -> Result<CleanResult, String>`
  - `pub fn launch_process(item: &LaunchItem) -> Result<u32, String>`

- [ ] **Step 1: 编写单元测试用例**
测试能够正确读取当前系统总物理内存与可用物理内存；测试启动器命令能正确执行常规程序。

- [ ] **Step 2: 运行测试并验证失败状态（TDD）**
运行 `cargo test`，验证模块尚未实现时的编译与测试断言。

- [ ] **Step 3: 实现 memory.rs 内存监测与工作集清理**
利用 Win32 `GlobalMemoryStatusEx` 获取内存，遍历进程快照（`CreateToolhelp32Snapshot`）安全调用 `EmptyWorkingSet`，自动忽略无权限进程。

- [ ] **Step 4: 实现 process.rs 进程启动器**
支持普通启动与静默启动（`CREATE_NO_WINDOW` 结合 `STARTUPINFOW.wShowWindow = SW_HIDE`），支持附加命令行参数和工作目录设置。

- [ ] **Step 5: 运行测试并确保全部通过**
运行 `cargo test` 确认内存读取与进程执行逻辑正常。

- [ ] **Step 6: 提交底层系统能力模块**
```bash
git add src-tauri/src/system
git commit -m "feat(system): implement Windows memory cleaner and process launcher"
```

---

### Task 3: Tauri Command 命令层与状态存储集成

**Files:**
- Create: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/storage.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `system::memory`, `system::process`
- Produces: 暴露给前端调用的 Tauri Commands:
  - `get_memory_status()`
  - `clean_system_memory()`
  - `execute_launch_item(item: LaunchItem)`
  - `execute_all_launch_items(items: Vec<LaunchItem>)`
  - `save_launcher_config(items: Vec<LaunchItem>)`
  - `load_launcher_config() -> Vec<LaunchItem>`

- [ ] **Step 1: 实现持久化存储 storage.rs**
使用用户标准配置目录存放 `launcher_config.json`，无任何硬编码个人路径。

- [ ] **Step 2: 编写 commands.rs 并绑定到 Tauri Builder**
将核心能力包装为 Tauri 命令，提供统一的错误字符串与返回结构。

- [ ] **Step 3: 运行 cargo check 验证无任何编译警告**
运行 `cargo check` 确保命令分发和类型转换无误。

- [ ] **Step 4: 提交 Tauri 命令与持久化层**
```bash
git add src-tauri/src/commands.rs src-tauri/src/storage.rs src-tauri/src/lib.rs
git commit -m "feat(api): expose Tauri commands for launcher and memory optimizer"
```

---

### Task 4: 前端核心外壳与模块化注册架构 (Fluent UI Shell)

**Files:**
- Create: `src/types/module.ts`
- Create: `src/registry/index.ts`
- Create: `src/components/layout/AppShell.vue`
- Create: `src/components/layout/Sidebar.vue`
- Create: `src/components/layout/HeaderBar.vue`
- Create: `src/App.vue`
- Create: `src/main.ts`
- Create: `src/style.css`

**Interfaces:**
- Consumes: Vue 3, Tailwind CSS, Lucide-Vue-Next
- Produces: 统一的 Win11 Fluent 磨砂质感外壳，以及动态注册各工具页面的 `ToolRegistry`。

- [ ] **Step 1: 编写模块化接口 `src/types/module.ts`**
定义 `ToolModule`、`LaunchItem`、`MemoryStatus` 等核心前端接口。

- [ ] **Step 2: 编写 Fluent UI 质感主题与 Tailwind 样式配置**
配置磨砂玻璃背景、细腻边框阴影、现代按钮与渐变微光效果。

- [ ] **Step 3: 实现 AppShell、Sidebar 与 HeaderBar**
实现侧边栏导航切换、顶部状态栏实时内存摘要气泡、窗口最小化/关闭控制。

- [ ] **Step 4: 验证前端编译**
运行 `npm run build` 确保布局组件编译通过。

- [ ] **Step 5: 提交前端主框架与模块化注册系统**
```bash
git add src/
git commit -m "feat(ui): implement Fluent-styled app shell and tool registry"
```

---

### Task 5: 快速启动器 (Launcher) 视图开发与配置管理

**Files:**
- Create: `src/views/LauncherView.vue`
- Create: `src/components/launcher/LaunchItemCard.vue`
- Create: `src/components/launcher/EditItemModal.vue`
- Modify: `src/registry/index.ts`

**Interfaces:**
- Consumes: Tauri invoke (`load_launcher_config`, `save_launcher_config`, `execute_launch_item`, `execute_all_launch_items`)
- Produces: 启动器功能完整界面，支持增删改查、多选/全选一键启动、静默开关、参数配置。

- [ ] **Step 1: 编写 LaunchItemCard 组件**
卡片展示启动项名称、文件路径、参数、静默标识徽章、单项启动按钮、编辑与删除菜单。

- [ ] **Step 2: 编写 EditItemModal 编辑弹窗**
提供名称、文件选择/手动输入、附加参数、工作目录、静默运行（无黑框模式）开关。

- [ ] **Step 3: 编写 LauncherView 主页面并集成一键启动逻辑**
包含顶部工具条（一键全部启动、添加项、搜索筛选）、列表卡片网格布局，并连接 Tauri 后端存储与启动命令。

- [ ] **Step 4: 将 Launcher 模块注册进 `ToolRegistry`**
注册为高效类别核心工具，侧边栏显示 Rocket 图标。

- [ ] **Step 5: 验证前端构建与交互逻辑**
运行 `npm run build` 验证编译。

- [ ] **Step 6: 提交启动器模块代码**
```bash
git add src/views/LauncherView.vue src/components/launcher/ src/registry/index.ts
git commit -m "feat(launcher): implement launcher UI with silent mode and batch execution"
```

---

### Task 6: 内存清理优化器 (Memory Optimizer) 视图开发

**Files:**
- Create: `src/views/MemoryCleanerView.vue`
- Create: `src/components/memory/MemoryGauge.vue`
- Create: `src/components/memory/ProcessImpactList.vue`
- Modify: `src/registry/index.ts`

**Interfaces:**
- Consumes: Tauri invoke (`get_memory_status`, `clean_system_memory`)
- Produces: 内存优化视图，包含动态圆环仪表盘、实时百分比、一键深度清理动效、释放量统计汇报。

- [ ] **Step 1: 编写 MemoryGauge 内存仪表盘组件**
展示总内存、已用/可用容量，使用平滑 SVG 环形进度条和动态色彩反馈（正常绿/中度黄/高负载红）。

- [ ] **Step 2: 编写 MemoryCleanerView 主页面与清理触发器**
实现一键“深度清理优化”按钮，带流光扫描动画，调用后实时更新内存指标并弹出释放量通知（如“成功清理 1.45 GB 内存”）。

- [ ] **Step 3: 将 MemoryCleaner 模块注册进 `ToolRegistry`**
侧边栏显示 Cpu / Gauge 图标，支持快速切换。

- [ ] **Step 4: 验证构建与组件整合**
运行 `npm run build` 验证无错误。

- [ ] **Step 5: 提交内存清理模块代码**
```bash
git add src/views/MemoryCleanerView.vue src/components/memory/ src/registry/index.ts
git commit -m "feat(memory): implement memory optimizer dashboard and deep clean action"
```

---

### Task 7: 完整端到端编译与功能验证

**Files:**
- Modify: `README.md`
- Test: 全系统端到端测试

**Interfaces:**
- Consumes: 完整前端与后端模块
- Produces: 可稳定运行的 OmniBox 万能工具箱桌面应用

- [ ] **Step 1: 编写项目 README.md 文档**
说明项目特性、架构设计、未来模块扩展指南、构建与调试方法，确保无任何本地隐私路径。

- [ ] **Step 2: 运行完整 Cargo Check 与前端 Build**
运行 `cargo check --manifest-path src-tauri/Cargo.toml` 和 `npm run build` 验证 100% 成功。

- [ ] **Step 3: 运行自动化与单元验证测试**
运行 `cargo test` 验证底层功能，验证空载内存占用符合预期。

- [ ] **Step 4: 完整提交与状态检查**
确保工作区干净，无泄露信息。
```bash
git add .
git commit -m "chore: complete initial release of OmniBox with launcher and memory cleaner"
```

# Windows 现代化万能工具箱 (OmniBox) 架构与设计规范

## 1. 项目愿景与目标

构建一个轻量、极低资源占用（常驻内存 < 35MB）、启动迅速且视觉风格高度统一（Windows 11 Fluent 风格）的现代化桌面端工具箱。该项目为长期演进项目，具备高模块化与可扩展能力，首期实现**多进程快速启动器**与**运行内存深度清理优化**两大核心工具。

- **操作系统支持**：Windows 10 / Windows 11 (x64)
- **核心理念**：极低开销、简单直接、无过度工程化、隐私友好（不包含任何私有硬编码路径）

---

## 2. 技术架构选型

- **底层核心**：Rust (Tauri v2 + Win32 API)
  - 通过 Tauri 框架与底层原生交互，仅利用 Windows 自带的 Evergreen WebView2，免去 Chromium 臃肿的基线开销。
  - 核心逻辑直接通过 Rust 调用 Win32 原生接口（`CreateProcessW`, `EmptyWorkingSet`, `GlobalMemoryStatusEx` 等），保证极致运行效率与精准进程控制。
- **前端界面**：Vue 3 + Vite + TypeScript + Tailwind CSS (Fluent UI 现代化磨砂质感)
  - 现代化组件化架构，响应式状态管理。
  - 统一的毛玻璃（Acrylic/Mica 拟态）质感、圆角、半透明层级、平滑过渡动画及暗黑/明亮主题支持。
- **持久化方案**：
  - 基于应用安全数据目录（`%APPDATA%/OmniBox/config.json` 或 Tauri AppDataDir）存储配置，杜绝硬编码敏感路径。

---

## 3. 模块化与可扩展设计 (Tool Registry)

为支持未来长期扩充新工具，采用模块化注册架构：
1. **模块定义接口 (`ToolModule`)**：
   ```typescript
   export interface ToolModule {
     id: string;              // 工具唯一标识 (如 'launcher', 'memory-cleaner')
     title: string;           // 显示名称
     description: string;     // 简短描述
     icon: string;            // 图标名称 (Lucide 图标)
     category: 'system' | 'efficiency' | 'dev'; // 工具类别
     component: Component;    // 工具主视图组件
   }
   ```
2. **注册中心 (`ToolRegistry`)**：
   - 统一管理所有工具的路由分发、侧边栏展示与快捷调用。
   - 未来新增任何功能（如哈希比对、文本处理、批量重命名等），仅需实现对应组件并在注册中心注入一行配置即可，核心外壳零修改。

---

## 4. 核心功能设计

### 4.1 快速启动器 (Launcher)

#### 需求描述
用户可配置多组启动目标（支持可执行文件 `.exe`、脚本 `.bat`/`.cmd`、快捷方式 `.lnk` 或普通文档），支持附加命令行参数与静默启动，并能一键并发或顺序唤起。

#### 核心数据模型
```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LaunchItem {
    pub id: String,             // 唯一编号 (UUID)
    pub name: String,           // 目标显示名称
    pub path: String,           // 文件或程序路径
    pub args: String,           // 启动参数 (命令行参数)
    pub work_dir: Option<String>,// 工作目录 (可选)
    pub silent: bool,           // 是否静默启动 (后台无窗口运行)
    pub enabled: bool,          // 是否参与一键启动
}
```

#### Rust 底层实现机制
- 对于普通启动：调用 `ShellExecuteExW` 或标准 `std::process::Command`。
- 对于静默启动：直接调用 Win32 `CreateProcessW` API，传入 `CREATE_NO_WINDOW` (0x08000000) 标志位，并将 `STARTUPINFOW` 的 `wShowWindow` 设为 `SW_HIDE` (0)，彻底杜绝黑色控制台窗口或前台窗口弹跳。
- 批量启动支持并发唤起与错误捕获，单项启动失败不影响其余项，并返回详细错误信息。

---

### 4.2 运行内存清理优化 (Memory Optimizer)

#### 需求描述
实时监测系统物理内存使用状态，提供一键清理优化功能，调用系统级接口对常驻进程的多余工作集进行释放，直观显示优化前后释放的容量。

#### 核心指标与监控
- 系统总内存 (Total Physical RAM)
- 当前已用内存与可用内存 (Used / Available RAM)
- 内存使用百分比 (Usage Rate %)
- 通过 `GlobalMemoryStatusEx` API 极速毫秒级采集，极低 CPU 负载。

#### 内存清理机制
1. **进程工作集修剪 (Working Set Trim)**：
   - 遍历当前系统活跃进程快照（`CreateToolhelp32Snapshot`），对有权限访问的进程句柄调用 `EmptyWorkingSet(hProcess)`。
   - 将各个进程长期未被访问的物理内存页强制刷写并移至备用/修改列表，立竿见影释放占用的物理内存。
2. **安全性与容错机制**：
   - 跳过受 Windows 系统内核保护的特权进程（自动忽略 `ACCESS_DENIED`），防止引发系统权限异常或崩溃。
   - 计算清理前后的可用物理内存差值，精准汇报实际释放的内存大小（如：“成功释放 1.25 GB 物理内存”）。

---

## 5. UI 与美术设计规范

- **整体调色板与材质**：
  - 背景色：暗色模式使用深邃深灰黑（`#0d1117` / `#161b22`），辅以现代磨砂卡片层级（`rgba(255, 255, 255, 0.04)`）。
  - 点缀色：Fluent 标志性电光蓝（`#0078d4` / `#2563eb`）及优雅渐变。
  - 材质质感：细边框（`1px border border-white/10`）、柔和阴影、微弱光晕效果。
- **布局设计**：
  - 紧凑型左侧导航栏（折叠/展开），展示各工具图标与标题。
  - 右侧为工具内容区，具备平滑切换动画。
  - 顶部/状态栏实时展示系统内存状态摘要。
- **交互反馈**：
  - 启动按钮、清理按钮具备波纹/脉冲微动画与明确的 Loading 状态。
  - 操作结果均通过 Fluent Toast 通知优雅呈现。

---

## 6. 安全、规范与不侵入约定

1. **隐私友好**：代码及提交历史中不包含任何具体开发者的本地绝对私有路径，配置项支持用户自主选取。
2. **临时文件管理**：所有编译缓存与临时产物严格遵循配置（`target/`, `node_modules/` 均在工作区并受 `.gitignore` 保护，临时调试文件统一存放于 `D:\` 根临时目录或 `.tmp` 目录）。
3. **无过度工程化**：只实现核心所需的数据流与组件，避免多层无用抽象。

---

## 7. 验证与验收标准

1. **构建与运行测试**：项目可顺利执行 `cargo check` / `npm run build`，打包成功且无未处理警告。
2. **启动器功能验收**：
   - 可成功添加、编辑、删除启动项目。
   - 验证普通可执行文件正常启动并带参数执行。
   - 验证静默启动模式下不弹出控制台黑框。
   - 验证一键启动功能按配置正常批量执行。
3. **内存清理功能验收**：
   - 实时准确读取 Windows 内存状态（与任务管理器对比一致）。
   - 点击一键清理后，观察内存占用百分比明显回落，界面精准显示释放的物理内存数值。

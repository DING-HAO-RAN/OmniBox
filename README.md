# OmniBox (万能工具箱)

<div align="center">

![OmniBox Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D4?style=flat-square&logo=windows)
![Tauri Version](https://img.shields.io/badge/Tauri-v2.0-FFC131?style=flat-square&logo=tauri&logoColor=black)
![Vue Version](https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square&logo=vuedotjs)
![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178C6?style=flat-square&logo=typescript)
![Rust](https://img.shields.io/badge/Rust-2021%20Edition-DEA584?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

**专为 Windows 10/11 量身打造的现代化、高颜值、超轻量极速系统桌面工具箱**

</div>

---

## 📖 项目愿景与设计定位

在现代桌面开发环境中，开发者与重度 PC 用户常常面临工具冗杂、后台资源开销巨大的痛点。传统基于 Electron 的桌面实用程序动辄占用上百兆内存与数百兆磁盘空间。

**OmniBox** 应运而生 —— 基于 **Tauri v2 + Rust** 核心构建，前端深度融合 **Windows 11 Fluent Design** 现代化设计语言，借助系统内置的 Microsoft Edge WebView2 运行环境，实现：
- ⚡ **极致轻量**：安装包体积极小，空闲常驻物理内存低于 **35 MB**，冷启动毫秒级响应。
- 🎨 **原生质感**：磨砂亚克力玻璃材质（Acrylic/Mica 视觉风格）、三色自适应流光仪表盘与平滑微交互。
- 🛡️ **底层赋能**：利用 Rust 直调 Win32 原生 API，兼备高性能与高可靠系统底层操纵能力。
- 🧩 **高可扩展**：微内核式 `ToolModule` 模块化插件体系，支持 3 分钟极速扩展独立专属工具。

---

## ✨ 核心特性

### 1. 🚀 快速启动器 (Smart Launcher)
告别杂乱的桌面快捷方式与重复的手工命令行敲击，为日常生产力打造工作流入口：
- **多类型可执行目标统一管理**：无缝支持系统内置程序名（如 `notepad.exe`、`calc.exe`）、环境变量 `PATH` 命令（如 `code`）、本地完整路径可执行程序（`.exe`）以及自动化批处理脚本（`.bat` / `.cmd`）。
- **参数附加与工作目录隔离**：为每个启动项自由配置命令行传参（Arguments）与执行工作路径（WorkDir）。
- **后台完全静默启动 (Silent Execution)**：底层调度注入 Windows `CREATE_NO_WINDOW` 标志位，**彻底消除控制台黑框闪烁**，特别契合后台守护进程与静默脚本。
- **一键批量并发唤起 (Batch Launch)**：Rust 原生多线程并行调度，毫秒内按序并发拉起工作环境所有关联应用，并精准反馈操作系统进程 PID 与启动状态。
- **即时检索与就地编辑**：毫秒级响应的本地实时模糊搜索过滤，支持自由开启/禁用单个启动项与快捷删除管理。

### 2. ⚡ 运行内存深度清理 (Memory Optimizer)
释放物理资源，告别系统卡顿与“假性内存泄漏”：
- **Win32 原生工作集修剪 (K32EmptyWorkingSet)**：直接调用 Windows 内核内存管理接口，毫秒级安全修剪所有活跃用户态进程的工作集（Working Set），立竿见影缩减物理内存占用。
- **安全保障与非侵入式架构**：**绝不强制杀死进程**，绝不破坏用户未保存的工作数据；仅回收闲置和被挂起的内存分页至系统分页池。当程序重新获得焦点时，系统按需缺页调度毫秒级唤醒。
- **关键系统进程豁免防护**：内核级安全防线，智能过滤并跳过 `csrss.exe`、`lsass.exe`、`smss.exe` 等核心安全进程，杜绝蓝屏与系统异常风险。
- **自适应 Fluent 三色环形仪表盘**：
  - `< 60%` 健康状态：电光青蓝渐变流光 (`#06b6d4` → `#3b82f6`)。
  - `60% ~ 80%` 中等负荷：明亮天空蓝至琥珀金黄 (`#38bdf8` → `#f59e0b`)。
  - `≥ 80%` 吃紧告警：热力橙红至玫红警报渐变 (`#f97316` → `#ef4444`)。
- **智能监控与成效分析**：内置 3 秒静默轮询机制（带在途请求互斥防护），直观展现优化前后占用率降幅、本次释放物理内存总容量以及成功修剪的活跃进程数量。

### 3. 🧩 模块化插件式扩展中心 (Tool Registry)
具备生命力与扩展性的工具架构基石：
- **微内核插件规范**：抽象标准 `ToolModule` 接口定义，彻底解耦功能模块与主窗口框架。
- **开箱即用的插槽机制**：仅需编写标准的 Vue 3 单文件组件，即可自动享有侧边栏分类导航、统一标题栏、Toast 全局弹窗通知与主题适配。

---

## 🏛️ 技术架构全景

```text
┌─────────────────────────────────────────────────────────────┐
│                    OmniBox 桌面客户端                       │
├─────────────────────────────────────────────────────────────┤
│  前端层 (Presentation Layer)                                │
│    ├── Vue 3 (Composition API / <script setup>)             │
│    ├── TypeScript (严格类型校验与接口规范)                  │
│    ├── Tailwind CSS (Windows 11 Fluent 视觉与毛玻璃特效)    │
│    ├── Lucide Icons (轻量现代矢量图标库)                    │
│    └── Tool Registry (响应式模块插件注册中心)               │
├─────────────────────────────────────────────────────────────┤
│  IPC 桥梁层 (Inter-Process Communication)                   │
│    └── Tauri v2 Core (类型安全的双向 Command 调用协议)      │
├─────────────────────────────────────────────────────────────┤
│  后端底层服务 (Rust Core & Win32 Native)                     │
│    ├── System Commands (内存状态监测、批量并发启动调度)     │
│    ├── Win32 API 互操作 (windows-sys: 内存管理/进程创建)   │
│    └── Storage Engine (系统标准应用目录 JSON 安全持久化)     │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ 本地开发与构建指南

### 前置环境要求
在开始之前，请确保您的开发机器已就绪以下基础环境：
1. **操作系统**：Windows 10 (1809 以上) 或 Windows 11。
2. **Node.js**：`v18.0.0` 或更高版本（推荐 `v20+` LTS）。
3. **Rust 工具链**：Stable channel（通过 [rustup](https://rustup.rs/) 安装，包含 `cargo` 与 `rustc`）。
4. **WebView2 Runtime**：Windows 10/11 绝大多数已内置；若缺失可从微软官网下载安装。

### 1. 克隆仓库与安装依赖
```bash
# 进入项目根目录
cd omnibox

# 安装前端依赖 (支持 npm / pnpm)
npm install
```

### 2. 本地前端独立预览 (开发调试 UI)
OmniBox 内部实现了平滑的非 Tauri 纯前端 Mock 降级，无需每次编译 Rust 即可极速调试 UI 界面：
```bash
npm run dev
```

### 3. 代码质量与类型校验
项目采用严格的双重强类型规范，确保 0 运行时类型故障：
```bash
# 检查前端 TypeScript 类型完整性与 Vite 构建
npm run build

# 检查 Rust 后端代码与 Win32 互操作安全
npm run check:rust
```

### 4. 运行底层单元测试
内置完备的自动化测试套件（覆盖启动项配置读写、损坏 JSON 回退机制与参数序列化）：
```powershell
# Windows PowerShell 环境下运行测试
powershell -Command "$env:CARGO_HOME = '$pwd\.cargo-home'; cargo test --manifest-path src-tauri/Cargo.toml"
```

### 5. 构建独立 Windows 生产包
```bash
npm run tauri build
```
打包成功后产物将生成于 `src-tauri/target/release/bundle/` 目录中。

---

## 🔌 3 分钟极速插件扩展指南

OmniBox 采用极简的模块化设计。若需新增一个小工具（例如：`端口占用排查器`），只需遵循以下三步：

### 第一步：创建工具视图组件
在 `src/views/` 目录下新建 `PortCheckerView.vue`：
```vue
<template>
  <div class="h-full p-8 text-white">
    <h1 class="text-xl font-bold mb-2">网络端口占用排障</h1>
    <p class="text-xs text-gray-400 mb-6">快速扫描与定位本地被占用的 TCP/UDP 端口</p>
    
    <!-- 你的业务逻辑与界面卡片 -->
    <div class="p-6 rounded-2xl bg-white/[0.03] border border-white/10">
      <span class="text-sm text-gray-300">功能就绪，随时扩展！</span>
    </div>
  </div>
</template>

<script setup lang="ts">
// 编写你的业务逻辑，可自由调用 Tauri invoke 或外部 API
</script>
```

### 第二步：在注册中心注册模块
打开 `src/registry/index.ts`，引入并调用 `registerTool`：
```typescript
import PortCheckerView from '../views/PortCheckerView.vue';

// 注册新工具
registerTool({
  id: 'port-checker',
  title: '端口排查',
  description: '本地网络端口占用诊断分析',
  iconName: 'Network',     // 支持任意 Lucide 图标名称
  category: 'dev',         // 可选: 'system' | 'efficiency' | 'dev'
  component: PortCheckerView,
  order: 4,                // 侧边栏排列展示顺序
});
```

### 第三步：完成！
启动或刷新应用，新的工具将自动呈现在侧边栏对应分类中，并享有全套 Fluent 动画、状态切换与自适应插槽容器。

---

## 🔒 隐私与安全合规承诺

OmniBox 将用户数据安全与隐私放在首位，恪守以下工程原则：

1. **纯本地离线运行**：软件完全离线工作，无任何遥测（Telemetry）、用户行为追踪或静默网络上报逻辑。
2. **拒绝绝对私有路径硬编码**：工程所有持久化配置均由操作系统标准用户目录调度（通过 Tauri 标准 `app_config_dir` 存放于 `%APPDATA%/omnibox/` 中），代码库完全脱敏，适合完全开源。
3. **标准特权最小化**：所有系统底层功能（包括内存工作集释放与进程启动）均在 Windows 标准用户上下文安全运行，不依赖或索取 UAC 管理员提权，保障操作系统底层稳定性。

---

## 📄 开源许可证

本项目基于 [MIT License](LICENSE) 协议开源，欢迎自由使用、学习交流与提交 Pull Request！

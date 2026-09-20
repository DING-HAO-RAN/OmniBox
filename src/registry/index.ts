/**
 * @file index.ts
 * @description 模块化工具响应式注册中心与调度管理器
 * 统一管理所有工具插件的元数据、插槽组件以及视图切换状态
 */

import { ref, computed, markRaw } from 'vue';
import type { ToolModule, ToolCategory } from '../types/module';
import OverviewView from '../components/views/OverviewView.vue';
import LauncherView from '../views/LauncherView.vue';
import MemoryCleanerView from '../views/MemoryCleanerView.vue';
import MarkdownPdfView from '../views/MarkdownPdfView.vue';
import FileCloakerView from '../views/FileCloakerView.vue';
import SettingsView from '../views/SettingsView.vue';
import PerformanceMonitorView from '../views/PerformanceMonitorView.vue';
import SystemToolsView from '../views/SystemToolsView.vue';
import SystemTweaksView from '../views/SystemTweaksView.vue';
import SessionMigratorView from '../views/SessionMigratorView.vue';
import TurboDownloaderView from '../views/TurboDownloaderView.vue';

/**
 * 分类元数据信息定义
 */
export interface CategoryMeta {
  id: ToolCategory;
  title: string;
  iconName: string;
}

/**
 * 支持的系统分类清单
 */
export const CATEGORIES: CategoryMeta[] = [
  { id: 'system', title: '系统优化', iconName: 'Cpu' },
  { id: 'efficiency', title: '高效工具', iconName: 'Rocket' },
  { id: 'dev', title: '开发辅助', iconName: 'Wrench' },
];

// 响应式工具列表存储
const tools = ref<ToolModule[]>([]);

// 当前选中的活跃工具 ID (默认激活概览中心)
const activeToolId = ref<string>('overview');

/**
 * 注册新工具模块
 * @param tool 工具模块定义
 */
export function registerTool(tool: ToolModule): void {
  const existingIndex = tools.value.findIndex((t) => t.id === tool.id);

  // 确保 Vue 组件不被转化为深层响应式代理
  const safeTool: ToolModule = {
    ...tool,
    order: tool.order ?? 99,
    component: markRaw(tool.component),
  };

  if (existingIndex !== -1) {
    console.warn(`[ToolRegistry] 工具 ID "${tool.id}" 已存在，正在更新模块配置`);
    tools.value[existingIndex] = safeTool;
  } else {
    tools.value.push(safeTool);
  }

  // 按权重升序排序 (权重越小越排在前面)
  tools.value.sort((a, b) => (a.order ?? 99) - (b.order ?? 99));

  // 若当前未激活任何工具，则默认激活首个注册的工具
  if (!activeToolId.value) {
    activeToolId.value = tool.id;
  }
}

/**
 * 按分类获取已排序的工具模块列表
 * @param category 工具类别
 */
export function getToolsByCategory(category: ToolCategory): ToolModule[] {
  return tools.value.filter((t) => t.category === category);
}

/**
 * 切换当前选中的视图模块
 * @param id 目标工具 ID
 */
export function setActiveTool(id: string): void {
  const found = tools.value.some((t) => t.id === id);
  if (found) {
    activeToolId.value = id;
  } else {
    console.warn(`[ToolRegistry] 无法激活未注册的工具 ID: "${id}"`);
  }
}

/**
 * 当前激活的工具模块对象 (计算属性)
 */
export const activeTool = computed<ToolModule | undefined>(() => {
  return tools.value.find((t) => t.id === activeToolId.value);
});

/**
 * 统一注册中心对象 (方便模板与组件引用)
 */
export const toolRegistry = {
  get tools() {
    return tools.value;
  },
  get activeToolId() {
    return activeToolId.value;
  },
  registerTool,
  getToolsByCategory,
  setActiveTool,
};

// ==========================================
// 初始化系统默认内建模块
// ==========================================

// 1. 硬件设备与性能实时监控面板
registerTool({
  id: 'performance',
  title: '性能监控',
  description: 'CPU/GPU/内存/存储/网络实时状态与动态波形折线图',
  iconName: 'Activity',
  category: 'system',
  component: PerformanceMonitorView,
  order: 1,
});

// 2. 概览中心控制台 (欢迎面板)
registerTool({
  id: 'overview',
  title: '概览中心',
  description: '系统运行概况与快捷控制仪表盘',
  iconName: 'LayoutDashboard',
  category: 'system',
  component: OverviewView,
  order: 2,
});

// 3. 内存优化器视图 (Task 6 接入)
registerTool({
  id: 'memory',
  title: '内存优化',
  description: 'Windows 进程工作集与待机缓存深度释放',
  iconName: 'Cpu',
  category: 'system',
  component: MemoryCleanerView,
  order: 3,
});

// 4. Windows 原生系统工具快捷开启面板
registerTool({
  id: 'system-tools',
  title: '系统工具',
  description: '组策略、注册表、计算机管理等 20+ 原生专业维护工具',
  iconName: 'Wrench',
  category: 'system',
  component: SystemToolsView,
  order: 4,
});

// 5. Windows 核心特性快速禁用与调优面板
registerTool({
  id: 'system-tweaks',
  title: '系统特性调优',
  description: '一键禁用自动更新、安全中心实时监控并释放 C 盘空间',
  iconName: 'Sliders',
  category: 'system',
  component: SystemTweaksView,
  order: 5,
});

// 6. 一键启动器视图 (Task 5 已正式接入)
registerTool({
  id: 'launcher',
  title: '应用启动器',
  description: '一键并行批量拉起目标工作环境',
  iconName: 'Rocket',
  category: 'efficiency',
  component: LauncherView,
  order: 6,
});

// 7. 高速多线程下载器 (TurboDownloader - 对标 PCL2 / FDM)
registerTool({
  id: 'turbo-downloader',
  title: '高速下载器',
  description: 'WinHTTP 零碎片多线程并发下载与 FDM 分片热力图',
  iconName: 'DownloadCloud',
  category: 'efficiency',
  component: TurboDownloaderView,
  order: 7,
});

// 8. 深度隐藏文件/文件夹 (超级隐藏 + 启动器协同)
registerTool({
  id: 'file-cloaker',
  title: '深度隐藏',
  description: 'Windows 系统级超级隐藏与私密启动联动',
  iconName: 'EyeOff',
  category: 'efficiency',
  component: FileCloakerView,
  order: 8,
});

// 9. 跨设备浏览器会话迁移与免密登录器 (Bilibili/通用网站会话登录器)
registerTool({
  id: 'session-migrator',
  title: 'B站/会话登录器',
  description: '跨设备浏览器会话迁移与免密快速登录器',
  iconName: 'Share2',
  category: 'efficiency',
  component: SessionMigratorView,
  order: 9,
});

// 10. Markdown 实时预览与 PDF 导出工作区
registerTool({
  id: 'markdown-pdf',
  title: 'Markdown 转 PDF',
  description: '实时预览并导出可配置版式的 PDF 文档',
  iconName: 'FileDown',
  category: 'dev',
  component: MarkdownPdfView,
  order: 10,
});

// 9. 通用配置中心 (单独固定在左侧栏最底部)
registerTool({
  id: 'settings',
  title: '通用设置',
  description: '管理员权限、自启策略、界面语言与自动优化配置',
  iconName: 'Settings',
  category: 'system',
  component: SettingsView,
  order: 99,
});

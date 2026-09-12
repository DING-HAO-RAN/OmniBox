/**
 * @file index.ts
 * @description 模块化工具响应式注册中心与调度管理器
 * 统一管理所有工具插件的元数据、插槽组件以及视图切换状态
 */

import { ref, computed, markRaw, h } from 'vue';
import type { ToolModule, ToolCategory } from '../types/module';
import OverviewView from '../components/views/OverviewView.vue';
import PlaceholderView from '../components/views/PlaceholderView.vue';
import LauncherView from '../views/LauncherView.vue';

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

// 1. 概览中心控制台 (欢迎面板)
registerTool({
  id: 'overview',
  title: '概览中心',
  description: '系统运行概况与快捷控制仪表盘',
  iconName: 'LayoutDashboard',
  category: 'system',
  component: OverviewView,
  order: 1,
});

// 2. 内存优化器占位 (Task 6 接入)
registerTool({
  id: 'memory',
  title: '内存优化',
  description: 'Windows 进程工作集深度释放与监控',
  iconName: 'Cpu',
  category: 'system',
  component: h(PlaceholderView, {
    title: '系统内存优化器',
    description: '即将接入 Win32 K32EmptyWorkingSet 原生修剪能力，提供毫秒级进程释放。',
  }),
  order: 2,
});

// 3. 一键启动器视图 (Task 5 已正式接入)
registerTool({
  id: 'launcher',
  title: '应用启动器',
  description: '一键并行批量拉起目标工作环境',
  iconName: 'Rocket',
  category: 'efficiency',
  component: LauncherView,
  order: 3,
});

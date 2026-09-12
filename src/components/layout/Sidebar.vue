<template>
  <!-- 侧边栏主容器：Windows 11 亚克力磨砂半透质感 -->
  <aside
    class="w-56 h-full flex-shrink-0 flex flex-col justify-between bg-[#111620]/80 backdrop-blur-xl border-r border-white/10 select-none z-20"
  >
    <!-- 上半部：品牌标识与工具导航列表 -->
    <div class="flex flex-col min-h-0 flex-1 overflow-hidden">
      <!-- 品牌 Header 区域 -->
      <div class="p-4 pb-3 border-b border-white/5 flex items-center gap-3">
        <div class="w-8 h-8 rounded-xl bg-gradient-to-br from-blue-500 to-sky-600 flex items-center justify-center text-white shadow-md shadow-blue-500/20">
          <Layers class="w-4 h-4" />
        </div>
        <div class="min-w-0 flex-1">
          <h1 class="text-sm font-bold tracking-tight text-white flex items-center gap-1.5">
            OmniBox
            <span class="text-[10px] font-mono px-1 py-0.2 rounded bg-blue-500/20 text-blue-400 border border-blue-500/30">
              Pro
            </span>
          </h1>
          <p class="text-[11px] text-gray-400 truncate">现代化极简工具箱</p>
        </div>
      </div>

      <!-- 分组工具菜单列表 (支持平滑滚动) -->
      <nav class="flex-1 overflow-y-auto px-2.5 py-3 space-y-4 win11-scrollbar">
        <div
          v-for="cat in activeCategories"
          :key="cat.id"
          class="space-y-1"
        >
          <!-- 分类标题 -->
          <div class="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-semibold text-gray-400/90 tracking-wide uppercase">
            <component :is="resolveIcon(cat.iconName)" class="w-3 h-3 text-gray-500" />
            <span>{{ cat.title }}</span>
          </div>

          <!-- 该分类下的工具项目 -->
          <div class="space-y-0.5">
            <button
              v-for="tool in getToolsByCategory(cat.id)"
              :key="tool.id"
              @click="setActiveTool(tool.id)"
              class="group relative w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-all duration-150 text-left border"
              :class="[
                activeToolId === tool.id
                  ? 'bg-white/[0.08] text-white font-semibold shadow-sm border-white/10'
                  : 'text-gray-400 hover:text-gray-200 hover:bg-white/[0.04] border-transparent'
              ]"
              :title="tool.description"
            >
              <!-- Windows 11 标志性激活左侧药丸指示条 -->
              <span
                v-if="activeToolId === tool.id"
                class="absolute left-1 top-1/2 -translate-y-1/2 h-4 w-1 rounded-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.6)]"
              ></span>

              <!-- 工具图标 -->
              <div
                class="w-4 h-4 flex items-center justify-center transition-colors"
                :class="[
                  activeToolId === tool.id
                    ? 'text-blue-400'
                    : 'text-gray-400 group-hover:text-gray-200'
                ]"
              >
                <component :is="resolveIcon(tool.iconName)" class="w-4 h-4" />
              </div>

              <!-- 工具标题 -->
              <span class="truncate flex-1">{{ tool.title }}</span>
            </button>
          </div>
        </div>
      </nav>
    </div>

    <!-- 下半部：系统核心运行状态与版本信息 -->
    <div class="p-3 border-t border-white/5 bg-white/[0.01]">
      <div class="p-2.5 rounded-xl bg-white/[0.02] border border-white/5 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="relative flex h-2 w-2">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
          </span>
          <span class="text-[11px] text-gray-300 font-medium">Rust 内核运行中</span>
        </div>
        <span class="text-[10px] font-mono text-gray-500">v0.1.0</span>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
/**
 * @file Sidebar.vue
 * @description Windows 11 Fluent 磨砂侧边栏导航组件
 */

import { computed } from 'vue';
import {
  Layers,
  LayoutDashboard,
  Cpu,
  Rocket,
  Wrench,
  Settings,
  HardDrive,
  Activity,
  Sparkles,
  HelpCircle,
} from 'lucide-vue-next';
import {
  CATEGORIES,
  getToolsByCategory,
  setActiveTool,
  toolRegistry,
} from '../../registry';

// 活跃工具 ID 计算属性引用
const activeToolId = computed(() => toolRegistry.activeToolId);

// 过滤仅展示有已注册工具的分类
const activeCategories = computed(() => {
  return CATEGORIES.filter((cat) => getToolsByCategory(cat.id).length > 0);
});

// 图标快速映射字典
const iconDictionary: Record<string, any> = {
  Layers,
  LayoutDashboard,
  Cpu,
  Rocket,
  Wrench,
  Settings,
  HardDrive,
  Activity,
  Sparkles,
};

/**
 * 根据图标名称动态解析对应的 Lucide 组件，保底兜底为 HelpCircle
 */
function resolveIcon(name: string) {
  return iconDictionary[name] || HelpCircle;
}
</script>

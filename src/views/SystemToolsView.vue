<template>
  <!-- Windows 原生系统实用工具快捷唤起面板 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 flex items-center justify-center">
              <Wrench class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">Windows 原生管理工具箱</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            一键秒级唤起组策略、注册表、计算机管理、设备驱动、系统配置等 20+ 款 Windows 原生专业维护工具。
          </p>
        </div>

        <!-- 搜索与计数 -->
        <div class="flex items-center gap-3">
          <div class="relative w-56">
            <Search class="w-3.5 h-3.5 text-gray-500 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="searchKeyword"
              type="text"
              placeholder="搜索工具名称或指令 (如 regedit)..."
              class="w-full h-8 pl-8 pr-3 rounded-xl bg-black/40 border border-white/10 text-xs text-gray-200 placeholder-gray-500 focus:outline-none focus:border-emerald-500/50"
            />
          </div>
        </div>
      </div>

      <!-- 分类标签过滤器 -->
      <div class="flex items-center gap-2 mt-4 overflow-x-auto win11-scrollbar pb-1">
        <button
          v-for="cat in categoryTabs"
          :key="cat.id"
          @click="activeCategory = cat.id"
          type="button"
          class="px-3 py-1.5 rounded-xl text-xs font-medium transition-all flex items-center gap-1.5 border"
          :class="activeCategory === cat.id ? 'bg-emerald-500/20 text-emerald-300 border-emerald-500/40 shadow-sm' : 'bg-white/[0.03] text-gray-400 hover:text-gray-200 hover:bg-white/[0.06] border-white/5'"
        >
          <span>{{ cat.title }}</span>
          <span class="text-[10px] font-mono opacity-70">({{ getCategoryCount(cat.id) }})</span>
        </button>
      </div>
    </header>

    <!-- 工具卡片网格列表 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="tool in filteredTools"
          :key="tool.id"
          class="group rounded-2xl p-4 bg-white/[0.025] hover:bg-white/[0.05] border border-white/10 hover:border-emerald-500/30 transition-all flex flex-col justify-between space-y-3"
        >
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2.5">
                <div class="w-8 h-8 rounded-xl bg-white/[0.04] group-hover:bg-emerald-500/10 text-gray-300 group-hover:text-emerald-400 border border-white/10 group-hover:border-emerald-500/30 flex items-center justify-center transition-colors">
                  <component :is="resolveToolIcon(tool.icon_name)" class="w-4 h-4" />
                </div>
                <div>
                  <h3 class="text-xs font-bold text-gray-100 group-hover:text-emerald-300 transition-colors">
                    {{ tool.name }}
                  </h3>
                  <span class="text-[10px] font-mono text-gray-500 block">
                    {{ tool.command }}
                  </span>
                </div>
              </div>
            </div>

            <p class="text-[11px] text-gray-400 leading-relaxed line-clamp-2">
              {{ tool.description }}
            </p>
          </div>

          <div class="pt-2 border-t border-white/5 flex items-center justify-between">
            <span class="text-[10px] text-gray-500">原生系统组件</span>
            <button
              @click="handleLaunchTool(tool)"
              type="button"
              class="px-3 py-1.5 rounded-lg text-xs font-medium text-emerald-300 bg-emerald-500/10 hover:bg-emerald-500/20 active:scale-95 border border-emerald-500/30 hover:border-emerald-500/50 transition-all flex items-center gap-1"
            >
              <ExternalLink class="w-3 h-3" />
              <span>立即打开</span>
            </button>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file SystemToolsView.vue
 * @description Windows 原生管理与实用诊断工具快捷面板
 */

import { ref, computed, onMounted } from 'vue';
import {
  Wrench,
  Search,
  ExternalLink,
  ShieldCheck,
  Database,
  MonitorCheck,
  Cog,
  HardDrive,
  PieChart,
  Activity,
  FolderSymlink,
  Gauge,
  LineChart,
  FileText,
  Cpu,
  Info,
  Sliders,
  Variable,
  LayoutGrid,
  Sparkles,
  Shield,
  Network,
  Key,
  ScreenShare,
  Terminal,
  TerminalSquare,
  HelpCircle,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { SystemToolItem } from '../types/module';
import { useToast } from '../composables/useToast';

const { toast } = useToast();

const tools = ref<SystemToolItem[]>([]);
const searchKeyword = ref('');
const activeCategory = ref('all');

const categoryTabs = [
  { id: 'all', title: '全部工具' },
  { id: 'admin', title: '管理维护' },
  { id: 'diag', title: '性能诊断' },
  { id: 'config', title: '系统配置' },
  { id: 'network', title: '网络终端' },
];

const iconMap: Record<string, any> = {
  ShieldCheck,
  Database,
  MonitorCheck,
  Cog,
  HardDrive,
  PieChart,
  Activity,
  FolderSymlink,
  Gauge,
  LineChart,
  FileText,
  Cpu,
  Info,
  Sliders,
  Variable,
  LayoutGrid,
  Sparkles,
  Shield,
  Network,
  Key,
  ScreenShare,
  Terminal,
  TerminalSquare,
};

function resolveToolIcon(name: string) {
  return iconMap[name] || HelpCircle;
}

function getCategoryCount(catId: string): number {
  if (catId === 'all') return tools.value.length;
  return tools.value.filter((t) => t.category === catId).length;
}

const filteredTools = computed(() => {
  const kw = searchKeyword.value.trim().toLowerCase();
  return tools.value.filter((t) => {
    const matchCat = activeCategory.value === 'all' || t.category === activeCategory.value;
    const matchKw =
      !kw ||
      t.name.toLowerCase().includes(kw) ||
      t.command.toLowerCase().includes(kw) ||
      t.description.toLowerCase().includes(kw);
    return matchCat && matchKw;
  });
});

async function loadTools() {
  if (isTauri()) {
    try {
      tools.value = await invoke<SystemToolItem[]>('get_system_tools');
    } catch (err) {
      console.error('加载系统工具失败:', err);
    }
  } else {
    // 浏览器 Mock 演示数据
    tools.value = [
      {
        id: 'gpedit',
        name: '组策略编辑器',
        description: '配置 Windows 用户权限与核心系统行为 (gpedit.msc)',
        category: 'admin',
        command: 'gpedit.msc',
        icon_name: 'ShieldCheck',
      },
      {
        id: 'regedit',
        name: '注册表编辑器',
        description: '查看与修改 Windows 注册表键值 (regedit.exe)',
        category: 'admin',
        command: 'regedit.exe',
        icon_name: 'Database',
      },
      {
        id: 'compmgmt',
        name: '计算机管理',
        description: '集成管理事件、共享文件夹与磁盘 (compmgmt.msc)',
        category: 'admin',
        command: 'compmgmt.msc',
        icon_name: 'MonitorCheck',
      },
    ];
  }
}

async function handleLaunchTool(tool: SystemToolItem) {
  try {
    if (isTauri()) {
      await invoke('launch_system_tool_cmd', { command: tool.command });
      toast.success('已唤起系统工具', `已打开【${tool.name}】(${tool.command})`);
    } else {
      toast.success('模拟唤起', `已唤起 ${tool.name}`);
    }
  } catch (err) {
    toast.error('唤起工具失败', String(err));
  }
}

onMounted(() => {
  loadTools();
});
</script>

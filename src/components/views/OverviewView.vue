<template>
  <!-- 概览仪表盘视图 -->
  <div class="h-full flex flex-col gap-6 p-6 overflow-y-auto win11-scrollbar">
    <!-- 顶部欢迎 Banner -->
    <section class="relative overflow-hidden rounded-2xl p-6 bg-gradient-to-r from-blue-900/30 via-sky-900/20 to-transparent border border-white/10 backdrop-blur-md">
      <div class="relative z-10 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="inline-flex items-center gap-2 px-2.5 py-1 rounded-full bg-blue-500/10 border border-blue-400/20 text-blue-400 text-xs font-medium mb-3">
            <Sparkles class="w-3.5 h-3.5" />
            <span>Windows 11 Fluent 架构就绪</span>
          </div>
          <h1 class="text-2xl font-bold text-white tracking-tight">
            欢迎使用 OmniBox 万能工具箱
          </h1>
          <p class="text-sm text-gray-400 mt-1 max-w-xl leading-relaxed">
            模块化极简桌面工具集。基于 Rust + Tauri v2 原生底层架构驱动，带来毫秒级响应与极限内存优化。
          </p>
        </div>

        <!-- 快速动作按钮 -->
        <div class="flex items-center gap-3">
          <button
            @click="refreshMemory"
            :disabled="loading"
            class="flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-medium bg-white/5 hover:bg-white/10 border border-white/10 text-gray-200 hover:text-white transition-all duration-150 active:scale-95 disabled:opacity-50"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': loading }" />
            <span>刷新状态</span>
          </button>
        </div>
      </div>

      <!-- 背景光晕装饰 -->
      <div class="absolute -right-12 -bottom-12 w-64 h-64 bg-blue-500/10 rounded-full blur-3xl pointer-events-none"></div>
    </section>

    <!-- 关键指标卡片网格 -->
    <section class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <!-- 内存实时状态卡片 -->
      <div class="group p-5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.05] border border-white/10 hover:border-blue-500/30 transition-all duration-200">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2.5 text-sm font-medium text-gray-300">
            <div class="w-8 h-8 rounded-lg bg-sky-500/15 border border-sky-500/20 flex items-center justify-center text-sky-400">
              <Cpu class="w-4 h-4" />
            </div>
            <span>系统物理内存</span>
          </div>
          <span
            class="text-xs px-2 py-0.5 rounded-full font-medium"
            :class="memStatusClass"
          >
            {{ memStatusText }}
          </span>
        </div>

        <div class="flex items-baseline gap-2 mb-2">
          <span class="text-3xl font-bold text-white tracking-tight">
            {{ memUsagePercent }}%
          </span>
          <span class="text-xs text-gray-400">已占用</span>
        </div>

        <!-- 进度条 -->
        <div class="w-full bg-white/10 rounded-full h-1.5 overflow-hidden mb-3">
          <div
            class="h-full rounded-full transition-all duration-500"
            :class="memProgressBarClass"
            :style="{ width: `${memUsagePercent}%` }"
          ></div>
        </div>

        <div class="flex justify-between text-xs text-gray-400">
          <span>已用: {{ formatBytes(memoryInfo.used_ram) }}</span>
          <span>总计: {{ formatBytes(memoryInfo.total_ram) }}</span>
        </div>
      </div>

      <!-- 快捷启动项卡片 -->
      <div class="group p-5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.05] border border-white/10 hover:border-emerald-500/30 transition-all duration-200">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2.5 text-sm font-medium text-gray-300">
            <div class="w-8 h-8 rounded-lg bg-emerald-500/15 border border-emerald-500/20 flex items-center justify-center text-emerald-400">
              <Rocket class="w-4 h-4" />
            </div>
            <span>一键应用启动器</span>
          </div>
          <span class="text-xs px-2 py-0.5 rounded-full font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            已就绪
          </span>
        </div>

        <p class="text-xs text-gray-400 leading-relaxed mb-4">
          支持一键批处理拉起多进程环境，支持普通与静默工作模式，告别繁琐手动启动。
        </p>

        <div class="flex items-center gap-2 text-xs text-emerald-400 font-medium">
          <Activity class="w-3.5 h-3.5" />
          <span>等待模块接入调度</span>
        </div>
      </div>

      <!-- 架构与安全特性卡片 -->
      <div class="group p-5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.05] border border-white/10 hover:border-indigo-500/30 transition-all duration-200">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2.5 text-sm font-medium text-gray-300">
            <div class="w-8 h-8 rounded-lg bg-indigo-500/15 border border-indigo-500/20 flex items-center justify-center text-indigo-400">
              <ShieldCheck class="w-4 h-4" />
            </div>
            <span>纯本地零遥测</span>
          </div>
          <span class="text-xs px-2 py-0.5 rounded-full font-medium bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
            原生安全
          </span>
        </div>

        <p class="text-xs text-gray-400 leading-relaxed mb-4">
          所有配置与状态均存储于本地应用目录，无任何外部网络追踪，Win32 专属安全机制保障。
        </p>

        <div class="flex items-center gap-2 text-xs text-indigo-400 font-medium">
          <Zap class="w-3.5 h-3.5" />
          <span>低资源驻留守护</span>
        </div>
      </div>
    </section>

    <!-- 模块化生态与插槽状态 -->
    <section class="rounded-2xl p-5 bg-white/[0.02] border border-white/10">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h2 class="text-sm font-semibold text-white">工具插件模块中心</h2>
          <p class="text-xs text-gray-400 mt-0.5">
            当前已接入核心注册中心的工具插槽概况
          </p>
        </div>
        <span class="text-xs font-mono text-gray-500">OmniBox Module Engine v1</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <div
          v-for="item in registeredModulesInfo"
          :key="item.id"
          class="p-3.5 rounded-xl bg-white/[0.02] border border-white/5 flex items-center gap-3 hover:bg-white/[0.04] transition-colors"
        >
          <div class="w-9 h-9 rounded-lg bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400 flex-shrink-0">
            <HardDrive class="w-4 h-4" />
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <h3 class="text-xs font-semibold text-gray-200 truncate">{{ item.title }}</h3>
              <span class="text-[10px] px-1.5 py-0.2 rounded bg-white/10 text-gray-300">{{ item.category }}</span>
            </div>
            <p class="text-[11px] text-gray-400 truncate mt-0.5">{{ item.description }}</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
/**
 * @file OverviewView.vue
 * @description 欢迎概览控制台视图组件
 */

import { ref, computed, onMounted } from 'vue';
import {
  Cpu,
  Rocket,
  Zap,
  ShieldCheck,
  Activity,
  HardDrive,
  Sparkles,
  RefreshCw,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { MemoryStatus } from '../../types/module';
import { useToast } from '../../composables/useToast';
import { toolRegistry } from '../../registry';

const { toast } = useToast();
const loading = ref(false);

// 内存数据响应式对象
const memoryInfo = ref<MemoryStatus>({
  total_ram: 16 * 1024 * 1024 * 1024,
  available_ram: 9.5 * 1024 * 1024 * 1024,
  used_ram: 6.5 * 1024 * 1024 * 1024,
  usage_percent: 40.6,
});

// 当前已注册的模块列表信息
const registeredModulesInfo = computed(() => {
  return toolRegistry.tools.map((t) => ({
    id: t.id,
    title: t.title,
    description: t.description,
    category: t.category === 'system' ? '系统' : t.category === 'efficiency' ? '效率' : '开发',
  }));
});

// 计算占用百分比展示
const memUsagePercent = computed(() => {
  return memoryInfo.value.usage_percent ? memoryInfo.value.usage_percent.toFixed(1) : '0.0';
});

// 内存健康度文本
const memStatusText = computed(() => {
  const percent = memoryInfo.value.usage_percent;
  if (percent >= 85) return '负载严重';
  if (percent >= 70) return '负载较高';
  return '状态良好';
});

// 内存状态徽标样式
const memStatusClass = computed(() => {
  const percent = memoryInfo.value.usage_percent;
  if (percent >= 85) return 'bg-rose-500/15 text-rose-400 border border-rose-500/30';
  if (percent >= 70) return 'bg-amber-500/15 text-amber-400 border border-amber-500/30';
  return 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30';
});

// 内存进度条颜色
const memProgressBarClass = computed(() => {
  const percent = memoryInfo.value.usage_percent;
  if (percent >= 85) return 'bg-gradient-to-r from-amber-500 to-rose-500';
  if (percent >= 70) return 'bg-gradient-to-r from-sky-500 to-amber-500';
  return 'bg-gradient-to-r from-blue-500 to-emerald-400';
});

/**
 * 转换字节数为适合阅读的 GB/MB
 */
function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 GB';
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(1)} GB`;
}

/**
 * 刷新内存状态
 */
async function refreshMemory() {
  loading.value = true;
  try {
    if (isTauri()) {
      const status = await invoke<MemoryStatus>('get_memory_status');
      memoryInfo.value = status;
      toast.success('状态已更新', `当前系统内存占用为 ${status.usage_percent.toFixed(1)}%`);
    } else {
      // 浏览器纯开发环境模拟
      memoryInfo.value = {
        total_ram: 16 * 1024 * 1024 * 1024,
        available_ram: 9.2 * 1024 * 1024 * 1024,
        used_ram: 6.8 * 1024 * 1024 * 1024,
        usage_percent: 42.5,
      };
      toast.info('开发模式', '当前运行于浏览器预览环境，已模拟刷新数据');
    }
  } catch (err) {
    console.error('获取内存状态失败:', err);
    toast.error('刷新失败', String(err));
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  refreshMemory();
});
</script>

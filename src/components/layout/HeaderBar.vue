<template>
  <!-- 顶部标题栏：支持 Windows 原生拖拽与系统控制按钮 -->
  <header
    data-tauri-drag-region
    @dblclick="handleHeaderDblClick"
    class="h-10 w-full flex-shrink-0 flex items-center justify-between px-3 bg-[#0d1117]/90 backdrop-blur-md border-b border-white/10 select-none z-30 cursor-default"
  >
    <!-- 左侧：应用标识与当前活跃模块面包屑 -->
    <div data-tauri-drag-region class="flex items-center gap-2.5 min-w-0">
      <div class="flex items-center gap-1.5 text-xs text-gray-300 font-medium">
        <Layers class="w-3.5 h-3.5 text-blue-400" />
        <span class="text-white font-semibold">OmniBox</span>
      </div>
      <span class="text-gray-600 text-xs">/</span>
      <span class="text-xs text-gray-400 truncate max-w-[140px]">
        {{ activeToolTitle }}
      </span>
    </div>

    <!-- 中间：实时内存简报胶囊 -->
    <div data-tauri-drag-region class="flex items-center justify-center flex-1">
      <button
        v-if="memoryCapsule"
        @click="navigateToMemoryTool"
        class="no-drag flex items-center gap-2 px-2.5 py-0.5 rounded-full bg-white/[0.04] hover:bg-white/[0.08] border border-white/10 transition-all duration-150 text-[11px] text-gray-300 active:scale-95"
        title="点击切换至内存优化管理"
      >
        <!-- 动态状态指示小圆点 -->
        <span class="relative flex h-1.5 w-1.5">
          <span
            class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75"
            :class="capsuleColorClass"
          ></span>
          <span
            class="relative inline-flex rounded-full h-1.5 w-1.5"
            :class="capsuleColorClass"
          ></span>
        </span>
        <span class="font-medium">
          内存 {{ memoryCapsule.percent }}%
        </span>
        <span class="text-gray-500">|</span>
        <span class="text-gray-400">
          {{ memoryCapsule.availableGb }} 可用
        </span>
      </button>
    </div>

    <!-- 右侧：Windows 11 风格自定义窗口操作按钮 -->
    <div class="no-drag flex items-center -mr-1">
      <!-- 最小化 -->
      <button
        @click="handleMinimize"
        class="w-10 h-8 flex items-center justify-center text-gray-400 hover:text-white hover:bg-white/10 rounded-sm transition-colors"
        title="最小化"
      >
        <Minus class="w-3.5 h-3.5" />
      </button>

      <!-- 最大化 / 还原 -->
      <button
        @click="handleToggleMaximize"
        class="w-10 h-8 flex items-center justify-center text-gray-400 hover:text-white hover:bg-white/10 rounded-sm transition-colors"
        :title="isMaximized ? '还原窗口' : '最大化'"
      >
        <Copy v-if="isMaximized" class="w-3 h-3 rotate-180" />
        <Square v-else class="w-3 h-3" />
      </button>

      <!-- 关闭窗口 -->
      <button
        @click="handleClose"
        class="w-10 h-8 flex items-center justify-center text-gray-400 hover:text-white hover:bg-rose-600 rounded-sm transition-colors"
        title="关闭应用"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
/**
 * @file HeaderBar.vue
 * @description Windows 11 Fluent 风格顶部导航栏与原生窗口交互按钮
 */

import { ref, computed, onMounted, onUnmounted } from 'vue';
import { Layers, Minus, Square, Copy, X } from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { MemoryStatus } from '../../types/module';
import { activeTool, setActiveTool } from '../../registry';

// 窗口最大化状态标志
const isMaximized = ref(false);

// 内存简报胶囊数据结构
interface MemoryCapsuleData {
  percent: number;
  availableGb: string;
}

const memoryCapsule = ref<MemoryCapsuleData | null>(null);
let pollTimer: ReturnType<typeof setInterval> | null = null;

// 当前活跃工具的标题
const activeToolTitle = computed(() => activeTool.value?.title || '主页');

// 根据内存使用率计算胶囊指示灯色调
const capsuleColorClass = computed(() => {
  if (!memoryCapsule.value) return 'bg-emerald-400';
  const p = memoryCapsule.value.percent;
  if (p >= 85) return 'bg-rose-400';
  if (p >= 70) return 'bg-amber-400';
  return 'bg-emerald-400';
});

/**
 * 静默轮询系统内存运行简报
 */
async function fetchMemoryCapsule() {
  try {
    if (isTauri()) {
      const res = await invoke<MemoryStatus>('get_memory_status');
      const gb = (res.available_ram / (1024 * 1024 * 1024)).toFixed(1);
      memoryCapsule.value = {
        percent: Math.round(res.usage_percent),
        availableGb: `${gb} GB`,
      };
    } else {
      // 浏览器预览开发降级
      memoryCapsule.value = {
        percent: 42,
        availableGb: '14.5 GB',
      };
    }
  } catch {
    // 捕获潜在异常静默跳过，避免干扰主界面
  }
}

/**
 * 快速跳转至内存优化工具页面
 */
function navigateToMemoryTool() {
  setActiveTool('memory');
}

/**
 * 窗口最小化交互
 */
async function handleMinimize() {
  if (isTauri()) {
    try {
      const win = getCurrentWindow();
      await win.minimize();
    } catch (err) {
      console.error('最小化失败:', err);
    }
  }
}

/**
 * 窗口最大化 / 还原交互
 */
async function handleToggleMaximize() {
  if (isTauri()) {
    try {
      const win = getCurrentWindow();
      await win.toggleMaximize();
      isMaximized.value = await win.isMaximized();
    } catch (err) {
      console.error('窗口最大化切换失败:', err);
    }
  } else {
    isMaximized.value = !isMaximized.value;
  }
}

/**
 * 关闭窗口
 */
async function handleClose() {
  if (isTauri()) {
    try {
      const win = getCurrentWindow();
      await win.close();
    } catch (err) {
      console.error('关闭窗口失败:', err);
    }
  }
}

/**
 * 标题栏空白区域双击最大化/还原
 */
function handleHeaderDblClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest('.no-drag')) return;
  handleToggleMaximize();
}

onMounted(() => {
  fetchMemoryCapsule();
  // 每 4 秒静默同步一次内存状态
  pollTimer = setInterval(fetchMemoryCapsule, 4000);

  // 初始化窗口最大化状态检查
  if (isTauri()) {
    getCurrentWindow().isMaximized().then((max) => {
      isMaximized.value = max;
    }).catch(() => {});
  }
});

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
});
</script>

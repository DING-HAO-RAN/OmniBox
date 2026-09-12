<template>
  <!-- 内存深度优化清理主视图容器 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-blue-500/20 text-blue-400 border border-blue-500/30 flex items-center justify-center">
              <Cpu class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">系统内存深度优化</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            原生调用 Win32 K32EmptyWorkingSet 算法修剪进程工作集，毫秒级释放物理内存。
          </p>
        </div>

        <!-- 顶部操作区：手动刷新状态与自动轮询标识 -->
        <div class="flex items-center gap-3">
          <div class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-white/[0.03] border border-white/10 text-xs text-gray-400">
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>实时监控中 (3s 轮询)</span>
          </div>

          <button
            @click="handleManualRefresh"
            :disabled="isFetching || isCleaning"
            class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] active:bg-white/[0.15] border border-white/10 hover:border-white/20 transition-all disabled:opacity-50"
            title="手动刷新当前内存指标"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isFetching }" />
            <span>刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主展示区：支持垂直平滑滚动 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar space-y-6">
      <!-- 核心仪表盘与清理触发卡片 -->
      <section class="relative overflow-hidden rounded-3xl p-8 bg-gradient-to-b from-white/[0.04] to-white/[0.01] border border-white/10 shadow-2xl backdrop-blur-md">
        <!-- 装饰性微弱发光背景 -->
        <div class="absolute -top-24 left-1/2 -translate-x-1/2 w-96 h-96 bg-blue-500/10 rounded-full blur-3xl pointer-events-none"></div>

        <div class="relative z-10 flex flex-col items-center">
          <!-- 动态 SVG 环形仪表盘 -->
          <MemoryGauge :memory-info="memoryInfo" :loading="isFetching" />

          <!-- 一键深度优化按钮与操作触发区 -->
          <div class="mt-8 flex flex-col items-center gap-3">
            <button
              @click="handleCleanMemory"
              :disabled="isCleaning"
              class="group relative overflow-hidden flex items-center justify-center gap-2.5 px-10 py-3.5 rounded-2xl text-sm font-semibold text-white bg-gradient-to-r from-blue-600 via-sky-500 to-indigo-600 hover:from-blue-500 hover:via-sky-400 hover:to-indigo-500 active:scale-[0.98] disabled:opacity-75 disabled:cursor-not-allowed disabled:transform-none transition-all shadow-xl shadow-blue-500/25 border border-blue-400/40"
            >
              <!-- 按钮内动态波纹与光效 -->
              <span
                v-if="isCleaning"
                class="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent animate-shimmer"
              ></span>

              <Loader2 v-if="isCleaning" class="w-5 h-5 animate-spin text-white" />
              <Zap v-else class="w-5 h-5 text-yellow-300 group-hover:scale-110 group-hover:rotate-12 transition-transform" />

              <span class="tracking-wide">
                {{ isCleaning ? '正在深度修剪进程工作集...' : '一键深度清理优化' }}
              </span>
            </button>

            <p class="text-xs text-gray-400 flex items-center gap-1.5">
              <CheckCircle2 class="w-3.5 h-3.5 text-emerald-400" />
              <span>安全保障：不会强杀程序或丢失未保存工作，仅回收闲置与挂起分页</span>
            </p>
          </div>
        </div>
      </section>

      <!-- 辅助面板网格：清理成效反馈与原理科普卡片 -->
      <section class="grid grid-cols-1 md:grid-cols-2 gap-5">
        <!-- 1. 优化成效与统计反馈面板 -->
        <div class="p-6 rounded-2xl bg-white/[0.03] border border-white/10 flex flex-col justify-between">
          <div>
            <div class="flex items-center justify-between mb-4">
              <div class="flex items-center gap-2 text-sm font-semibold text-white">
                <BarChart3 class="w-4 h-4 text-blue-400" />
                <span>优化成效与反馈</span>
              </div>
              <span
                v-if="lastCleanResult"
                class="text-[11px] px-2 py-0.5 rounded-md font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
              >
                上次优化: {{ lastCleanTime }}
              </span>
              <span
                v-else
                class="text-[11px] px-2 py-0.5 rounded-md font-mono bg-white/5 text-gray-400 border border-white/10"
              >
                尚未执行优化
              </span>
            </div>

            <!-- 若已有优化记录 -->
            <div v-if="lastCleanResult" class="space-y-4">
              <div class="grid grid-cols-2 gap-3">
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
                  <span class="text-[11px] text-gray-400 block mb-1">本次释放容量</span>
                  <div class="flex items-baseline gap-1">
                    <span class="text-xl font-bold font-mono text-emerald-400">
                      {{ formatFreed(lastCleanResult.freed_mb) }}
                    </span>
                  </div>
                </div>

                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
                  <span class="text-[11px] text-gray-400 block mb-1">修剪活跃进程</span>
                  <div class="flex items-baseline gap-1">
                    <span class="text-xl font-bold font-mono text-white">
                      {{ lastCleanResult.processes_trimmed }}
                    </span>
                    <span class="text-xs text-gray-400">个</span>
                  </div>
                </div>
              </div>

              <!-- 优化前后降幅对比条 -->
              <div class="p-3.5 rounded-xl bg-white/[0.02] border border-white/5">
                <div class="flex items-center justify-between text-xs mb-2">
                  <span class="text-gray-400">清理前后占用率变化</span>
                  <div class="flex items-center gap-1.5 font-mono">
                    <span class="text-gray-400 line-through">{{ lastCleanResult.before_usage_percent.toFixed(1) }}%</span>
                    <ArrowDownRight class="w-3.5 h-3.5 text-emerald-400" />
                    <span class="text-emerald-400 font-bold">{{ lastCleanResult.after_usage_percent.toFixed(1) }}%</span>
                    <span class="text-[10px] px-1 rounded bg-emerald-500/20 text-emerald-300 ml-1">
                      -{{ (lastCleanResult.before_usage_percent - lastCleanResult.after_usage_percent).toFixed(1) }}%
                    </span>
                  </div>
                </div>

                <div class="w-full bg-white/10 rounded-full h-1.5 overflow-hidden">
                  <div
                    class="h-full rounded-full bg-gradient-to-r from-blue-500 to-emerald-400 transition-all duration-500"
                    :style="{ width: `${Math.min(100, Math.max(0, lastCleanResult.after_usage_percent))}%` }"
                  ></div>
                </div>
              </div>
            </div>

            <!-- 未优化时的空状态指引 -->
            <div v-else class="py-6 flex flex-col items-center justify-center text-center">
              <div class="w-12 h-12 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400 mb-3">
                <Sparkles class="w-6 h-6" />
              </div>
              <p class="text-xs text-gray-300 font-medium">随时准备一键深度释放</p>
              <p class="text-[11px] text-gray-500 mt-1 max-w-xs">
                点击上方「一键深度清理优化」，即可查看本次优化的释放总量与工作集降幅详情。
              </p>
            </div>
          </div>

          <!-- 底部健康状态评估 -->
          <div class="mt-4 pt-3 border-t border-white/5 flex items-center justify-between text-xs">
            <span class="text-gray-400">系统内存健康度评估</span>
            <div class="flex items-center gap-1.5 font-medium" :class="healthScoreColor">
              <Activity class="w-3.5 h-3.5" />
              <span>{{ healthScoreText }}</span>
            </div>
          </div>
        </div>

        <!-- 2. Windows 工作集安全机制科普 -->
        <div class="p-6 rounded-2xl bg-white/[0.03] border border-white/10 flex flex-col justify-between">
          <div>
            <div class="flex items-center gap-2 text-sm font-semibold text-white mb-4">
              <ShieldCheck class="w-4 h-4 text-emerald-400" />
              <span>Win32 内存修剪机制安全科普</span>
            </div>

            <div class="space-y-3.5 text-xs text-gray-300 leading-relaxed">
              <div class="flex items-start gap-2.5">
                <div class="w-5 h-5 rounded-md bg-blue-500/10 text-blue-400 flex items-center justify-center flex-shrink-0 mt-0.5">
                  1
                </div>
                <div>
                  <strong class="text-gray-100">非侵入式工作集修剪</strong>
                  <p class="text-gray-400 text-[11px] mt-0.5">
                    调用 Windows 原生 API 将长期闲置的代码与数据页安全放回分页池，绝不强制杀死进程，保障数据 100% 完整。
                  </p>
                </div>
              </div>

              <div class="flex items-start gap-2.5">
                <div class="w-5 h-5 rounded-md bg-sky-500/10 text-sky-400 flex items-center justify-center flex-shrink-0 mt-0.5">
                  2
                </div>
                <div>
                  <strong class="text-gray-100">智能按需缺页加载</strong>
                  <p class="text-gray-400 text-[11px] mt-0.5">
                    当后台被修剪的程序重新被用户聚焦唤醒时，操作系统将在毫秒级按需载入活动页，兼顾充裕内存与流畅切换体验。
                  </p>
                </div>
              </div>

              <div class="flex items-start gap-2.5">
                <div class="w-5 h-5 rounded-md bg-indigo-500/10 text-indigo-400 flex items-center justify-center flex-shrink-0 mt-0.5">
                  3
                </div>
                <div>
                  <strong class="text-gray-100">系统内核保护与零提权风险</strong>
                  <p class="text-gray-400 text-[11px] mt-0.5">
                    程序自动避开 csrss.exe、lsass.exe 等底层关键系统安全进程，杜绝蓝屏隐患，安全可信赖。
                  </p>
                </div>
              </div>
            </div>
          </div>

          <div class="mt-4 pt-3 border-t border-white/5 flex items-center justify-between text-[11px] text-gray-500">
            <span>底层驱动: Win32 K32EmptyWorkingSet</span>
            <span>纯本地运行 · 零网络调用</span>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file MemoryCleanerView.vue
 * @description 内存深度优化清理主视图
 * 提供实时 SVG 环形仪表盘状态轮询、一键工作集修剪清理调度、成效对比与释放反馈
 */

import { ref, computed, onMounted, onUnmounted } from 'vue';
import {
  Cpu,
  RefreshCw,
  Zap,
  Loader2,
  CheckCircle2,
  BarChart3,
  ArrowDownRight,
  ShieldCheck,
  Activity,
  Sparkles,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { MemoryStatus, CleanResult } from '../types/module';
import { useToast } from '../composables/useToast';
import MemoryGauge from '../components/memory/MemoryGauge.vue';

const { toast } = useToast();

// 内存指标响应式数据 (默认保底数值)
const memoryInfo = ref<MemoryStatus>({
  total_ram: 16 * 1024 * 1024 * 1024,
  available_ram: 8.5 * 1024 * 1024 * 1024,
  used_ram: 7.5 * 1024 * 1024 * 1024,
  usage_percent: 46.8,
});

// 加载与清理执行中状态
const isFetching = ref(false);
const isCleaning = ref(false);
// 在途请求互斥守卫，防止极端弱环境下高频并发请求堆叠
let isFetchingInFlight = false;

// 最近一次清理优化结果
const lastCleanResult = ref<CleanResult | null>(null);
const lastCleanTime = ref<string | null>(null);

// 定时轮询器句柄
let pollIntervalId: ReturnType<typeof setInterval> | null = null;

/**
 * 格式化 MB 释放容量显示
 */
function formatFreed(mb: number): string {
  if (!mb || mb <= 0) return '0 MB';
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(2)} GB`;
  }
  return `${mb.toFixed(1)} MB`;
}

/**
 * 评估系统内存健康度
 */
const healthScoreText = computed(() => {
  const p = memoryInfo.value.usage_percent;
  if (p < 55) return '状态极佳 · 物理资源充裕';
  if (p < 75) return '负荷正常 · 适宜日常运行';
  if (p < 90) return '负荷偏高 · 建议深度优化';
  return '严重受限 · 内存高度吃紧';
});

const healthScoreColor = computed(() => {
  const p = memoryInfo.value.usage_percent;
  if (p < 55) return 'text-emerald-400';
  if (p < 75) return 'text-sky-400';
  if (p < 90) return 'text-amber-400';
  return 'text-rose-400';
});

/**
 * 从后端拉取当前系统内存实时状态
 * @param silent 是否静默刷新 (非静默时可激活按钮动画)
 */
async function fetchMemoryStatus(silent = false) {
  // 在途互斥守卫：若上一轮请求尚未结算，直接跳过，杜绝高频并发堆积
  if (isFetchingInFlight) return;
  isFetchingInFlight = true;

  if (!silent) {
    isFetching.value = true;
  }
  try {
    if (isTauri()) {
      const status = await invoke<MemoryStatus>('get_memory_status');
      memoryInfo.value = status;
    } else {
      // 浏览器非 Tauri 纯前端开发环境 Mock
      const base = 16 * 1024 * 1024 * 1024;
      const currentPct = memoryInfo.value.usage_percent;
      // 模拟微幅动态波动
      const delta = (Math.random() - 0.5) * 1.5;
      const nextPct = Math.min(95, Math.max(20, currentPct + delta));
      const used = (base * nextPct) / 100;
      memoryInfo.value = {
        total_ram: base,
        used_ram: used,
        available_ram: base - used,
        usage_percent: nextPct,
      };
    }
  } catch (err) {
    console.error('[Memory] 获取内存状态失败:', err);
  } finally {
    if (!silent) {
      isFetching.value = false;
    }
    isFetchingInFlight = false;
  }
}

/**
 * 手动点击刷新内存状态
 */
async function handleManualRefresh() {
  await fetchMemoryStatus(false);
  toast.info('内存状态已同步', `当前系统物理内存占用率为 ${memoryInfo.value.usage_percent.toFixed(1)}%`);
}

/**
 * 执行一键深度清理优化
 */
async function handleCleanMemory() {
  if (isCleaning.value) return;
  isCleaning.value = true;

  try {
    let result: CleanResult;

    if (isTauri()) {
      // 1. 调用 Rust Tauri 核心清理命令
      result = await invoke<CleanResult>('clean_system_memory');

      // 2. 紧接着拉取最新系统内存状态，触发仪表盘平滑回落
      const latestStatus = await invoke<MemoryStatus>('get_memory_status');
      memoryInfo.value = latestStatus;
    } else {
      // 浏览器非 Tauri 纯前端开发环境模拟
      await new Promise((r) => setTimeout(r, 900));

      const beforePct = memoryInfo.value.usage_percent;
      const afterPct = Math.max(28.0, beforePct - 18.5);
      const total = 16 * 1024 * 1024 * 1024;
      const freedBytes = (total * (beforePct - afterPct)) / 100;
      const freedMb = freedBytes / (1024 * 1024);

      result = {
        freed_bytes: freedBytes,
        freed_mb: freedMb,
        processes_trimmed: 68,
        before_usage_percent: beforePct,
        after_usage_percent: afterPct,
      };

      const newUsed = (total * afterPct) / 100;
      memoryInfo.value = {
        total_ram: total,
        used_ram: newUsed,
        available_ram: total - newUsed,
        usage_percent: afterPct,
      };
    }

    // 记录优化成效
    lastCleanResult.value = result;
    lastCleanTime.value = new Date().toLocaleTimeString('zh-CN', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });

    // 弹出优雅 Toast 反馈
    const freedText = formatFreed(result.freed_mb);
    toast.success(
      '深度优化完成！',
      `成功释放 ${freedText} 物理内存，修剪了 ${result.processes_trimmed} 个活跃进程工作集。`
    );
  } catch (err) {
    console.error('[Memory] 深度内存清理失败:', err);
    toast.error('优化清理失败', String(err));
  } finally {
    isCleaning.value = false;
  }
}

// ----------------------------------------------------
// 组件生命周期挂载与安全定时轮询
// ----------------------------------------------------

onMounted(async () => {
  // 首次挂载立即拉取
  await fetchMemoryStatus(true);

  // 开启 3 秒一次的静默轮询更新
  pollIntervalId = setInterval(() => {
    // 若正在清理中或已有在途请求未完成则跳过此轮轮询
    if (!isCleaning.value && !isFetchingInFlight) {
      fetchMemoryStatus(true);
    }
  }, 3000);
});

onUnmounted(() => {
  // 组件卸载时安全释放定时器，防止内存泄漏
  if (pollIntervalId) {
    clearInterval(pollIntervalId);
    pollIntervalId = null;
  }
});
</script>

<style scoped>
/* 按钮内光波扫描动画 */
@keyframes shimmer {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
}

.animate-shimmer {
  animation: shimmer 1.5s infinite linear;
}
</style>

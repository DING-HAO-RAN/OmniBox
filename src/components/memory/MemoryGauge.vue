<template>
  <!-- 内存动态仪表盘与指标概览容器 -->
  <div class="flex flex-col items-center w-full max-w-xl mx-auto select-none">
    <!-- SVG 环形仪表盘核心区 -->
    <div class="relative w-64 h-64 flex items-center justify-center">
      <!-- 动态呼吸背景光晕：随负荷颜色智能联动 -->
      <div
        class="absolute w-48 h-48 rounded-full blur-3xl opacity-30 pointer-events-none transition-all duration-700 animate-pulse"
        :class="glowBgClass"
      ></div>

      <!-- SVG 环形渐变进度条 -->
      <svg
        class="w-full h-full transform -rotate-90 drop-shadow-md"
        viewBox="0 0 240 240"
      >
        <!-- 渐变色定义 -->
        <defs>
          <!-- 健康状态渐变 (电光青蓝 -> 极客翡翠蓝) -->
          <linearGradient id="healthyGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#06b6d4" />
            <stop offset="100%" stop-color="#3b82f6" />
          </linearGradient>

          <!-- 中等负荷渐变 (明亮天空蓝 -> 琥珀金黄) -->
          <linearGradient id="moderateGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#38bdf8" />
            <stop offset="100%" stop-color="#f59e0b" />
          </linearGradient>

          <!-- 高负荷警告渐变 (热力橙红 -> 玫红警报) -->
          <linearGradient id="highGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#f97316" />
            <stop offset="100%" stop-color="#ef4444" />
          </linearGradient>

          <!-- 柔和发光阴影滤镜 -->
          <filter id="gaugeGlow" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="3" result="blur" />
            <feComposite in="SourceGraphic" in2="blur" operator="over" />
          </filter>
        </defs>

        <!-- 背景外层极细刻度底环 -->
        <circle
          cx="120"
          cy="120"
          r="104"
          fill="none"
          stroke="currentColor"
          stroke-width="1"
          stroke-dasharray="3 6"
          class="text-white/10"
        />

        <!-- 背景轨道圆环 -->
        <circle
          cx="120"
          cy="120"
          r="90"
          fill="none"
          stroke="currentColor"
          stroke-width="12"
          class="text-white/[0.06]"
        />

        <!-- 实时渐变进度圆环 (带平滑 dashoffset 过渡与微光效果) -->
        <circle
          cx="120"
          cy="120"
          r="90"
          fill="none"
          :stroke="gaugeGradientUrl"
          stroke-width="12"
          stroke-linecap="round"
          filter="url(#gaugeGlow)"
          class="transition-all duration-700 ease-out"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="dashOffset"
        />
      </svg>

      <!-- 圆环正中央：状态徽章与大号等宽指标数值 -->
      <div class="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
        <!-- 智能负荷状态胶囊徽章 -->
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium border mb-1 transition-all duration-500"
          :class="badgeClass"
        >
          <span class="w-1.5 h-1.5 rounded-full animate-ping" :class="pingDotClass"></span>
          <span>{{ statusLabel }}</span>
        </div>

        <!-- 核心百分比数值 -->
        <div class="flex items-baseline font-mono tracking-tight text-white select-none">
          <span class="text-4xl font-extrabold tracking-tighter">
            {{ displayPercent }}
          </span>
          <span v-if="usagePercent !== null" class="text-lg font-bold text-gray-400 ml-0.5">%</span>
        </div>

        <!-- 辅助副标题说明 -->
        <span class="text-[11px] text-gray-400 font-medium tracking-wide mt-0.5">
          {{ loading ? '正在检测内存...' : '实时物理内存占用' }}
        </span>
      </div>
    </div>

    <!-- 环形下方紧凑排列的 3 个物理内存指标卡片 -->
    <div class="grid grid-cols-3 gap-3 w-full mt-6">
      <!-- 1. 物理总内存 (Total) -->
      <div
        class="group p-3.5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.06] border border-white/10 hover:border-blue-500/30 transition-all duration-200 flex flex-col"
      >
        <div class="flex items-center gap-1.5 text-xs text-gray-400 mb-1.5">
          <Server class="w-3.5 h-3.5 text-blue-400 group-hover:scale-110 transition-transform" />
          <span class="font-medium">物理总内存</span>
        </div>
        <div class="flex items-baseline gap-1 mt-auto">
          <span class="text-lg font-bold font-mono text-white tracking-tight">
            {{ totalGb }}
          </span>
          <span class="text-xs text-gray-400">GB</span>
        </div>
        <div class="text-[10px] text-gray-500 mt-0.5 font-mono">
          Win32 系统识别
        </div>
      </div>

      <!-- 2. 当前已用内存 (Used) -->
      <div
        class="group p-3.5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.06] border border-white/10 hover:border-amber-500/30 transition-all duration-200 flex flex-col"
      >
        <div class="flex items-center gap-1.5 text-xs text-gray-400 mb-1.5">
          <Activity class="w-3.5 h-3.5 group-hover:scale-110 transition-transform" :class="indicatorIconColor" />
          <span class="font-medium">已用内存</span>
        </div>
        <div class="flex items-baseline gap-1 mt-auto">
          <span class="text-lg font-bold font-mono tracking-tight" :class="indicatorTextColor">
            {{ usedGb }}
          </span>
          <span class="text-xs text-gray-400">GB</span>
        </div>
        <div class="text-[10px] text-gray-500 mt-0.5 font-mono">
          占比 {{ displayPercent }}%
        </div>
      </div>

      <!-- 3. 剩余可用内存 (Available) -->
      <div
        class="group p-3.5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.06] border border-white/10 hover:border-emerald-500/30 transition-all duration-200 flex flex-col"
      >
        <div class="flex items-center gap-1.5 text-xs text-gray-400 mb-1.5">
          <Sparkles class="w-3.5 h-3.5 text-emerald-400 group-hover:scale-110 transition-transform" />
          <span class="font-medium">剩余可用</span>
        </div>
        <div class="flex items-baseline gap-1 mt-auto">
          <span class="text-lg font-bold font-mono text-emerald-400 tracking-tight">
            {{ availableGb }}
          </span>
          <span class="text-xs text-gray-400">GB</span>
        </div>
        <div class="text-[10px] text-gray-500 mt-0.5 font-mono">
          富余 {{ availablePercent }}%
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * @file MemoryGauge.vue
 * @description 高质感 SVG 环形渐变内存动态仪表盘组件
 * 包含平滑 CSS stroke-dashoffset 过渡、智能三色负荷自适应以及物理内存核心指标卡片
 */

import { computed } from 'vue';
import { Server, Activity, Sparkles } from 'lucide-vue-next';
import type { SystemMemoryInfo } from '../../types/module';
import { metricNumber } from '../../lib/metric';

const props = withDefaults(
  defineProps<{
    /** 内存运行状态数据；尚未取得真实指标时为空 */
    memoryInfo: SystemMemoryInfo | null;
    /** 是否正处于刷新或清理中 */
    loading?: boolean;
  }>(),
  {
    loading: false,
  }
);

// 圆环半径与周长常数 (R = 90)
const RADIUS = 90;
const circumference = 2 * Math.PI * RADIUS; // 约 565.4867

// 通过 helper 读取使用率；无效指标不参与算术。
const usagePercent = computed(() => metricNumber(props.memoryInfo?.usage_percent));

const displayPercent = computed(() => {
  const percent = usagePercent.value;
  return percent === null ? '—' : Math.min(100, Math.max(0, percent)).toFixed(1);
});

// 无值时显示空环，避免把缺失指标伪造成有效百分比。
const dashOffset = computed(() => {
  const percent = usagePercent.value;
  return percent === null ? circumference : circumference * (1 - Math.min(100, Math.max(0, percent)) / 100);
});

// 负荷等级划分：< 60% 健康；60% - 80% 中等；>= 80% 高负荷
const loadLevel = computed<'healthy' | 'moderate' | 'high'>(() => {
  const percent = usagePercent.value;
  if (percent !== null && percent >= 80) return 'high';
  if (percent !== null && percent >= 60) return 'moderate';
  return 'healthy';
});

// SVG 进度条渐变 URL
const gaugeGradientUrl = computed(() => {
  switch (loadLevel.value) {
    case 'high':
      return 'url(#highGradient)';
    case 'moderate':
      return 'url(#moderateGradient)';
    case 'healthy':
    default:
      return 'url(#healthyGradient)';
  }
});

// 负荷状态文字标签
const statusLabel = computed(() => {
  if (usagePercent.value === null) return '暂不可用';
  switch (loadLevel.value) {
    case 'high':
      return '负载偏高';
    case 'moderate':
      return '负荷适中';
    case 'healthy':
    default:
      return '运行健康';
  }
});

// 状态徽章样式类
const badgeClass = computed(() => {
  if (usagePercent.value === null) return 'bg-white/5 text-gray-400 border-white/10';
  switch (loadLevel.value) {
    case 'high':
      return 'bg-rose-500/15 text-rose-400 border-rose-500/30';
    case 'moderate':
      return 'bg-amber-500/15 text-amber-400 border-amber-500/30';
    case 'healthy':
    default:
      return 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30';
  }
});

// 脉冲小圆点样式
const pingDotClass = computed(() => {
  if (usagePercent.value === null) return 'bg-gray-400';
  switch (loadLevel.value) {
    case 'high':
      return 'bg-rose-400';
    case 'moderate':
      return 'bg-amber-400';
    case 'healthy':
    default:
      return 'bg-emerald-400';
  }
});

// 背景呼吸光晕样式类
const glowBgClass = computed(() => {
  if (usagePercent.value === null) return 'bg-gray-500/20';
  switch (loadLevel.value) {
    case 'high':
      return 'bg-rose-500/20';
    case 'moderate':
      return 'bg-amber-500/20';
    case 'healthy':
    default:
      return 'bg-blue-500/20';
  }
});

// 指标高亮文本与图标颜色
const indicatorTextColor = computed(() => {
  if (usagePercent.value === null) return 'text-gray-400';
  switch (loadLevel.value) {
    case 'high':
      return 'text-rose-400';
    case 'moderate':
      return 'text-amber-400';
    case 'healthy':
    default:
      return 'text-blue-400';
  }
});

const indicatorIconColor = computed(() => {
  if (usagePercent.value === null) return 'text-gray-400';
  switch (loadLevel.value) {
    case 'high':
      return 'text-rose-400';
    case 'moderate':
      return 'text-amber-400';
    case 'healthy':
    default:
      return 'text-blue-400';
  }
});

// 格式化为 GB 保留两位小数
function toGb(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes)) return '—';
  return (bytes / (1024 * 1024 * 1024)).toFixed(2);
}

const totalBytes = computed(() => metricNumber(props.memoryInfo?.total_physical_bytes));
const usedBytes = computed(() => metricNumber(props.memoryInfo?.used_physical_bytes));
const availableBytes = computed(() => metricNumber(props.memoryInfo?.available_physical_bytes));
const totalGb = computed(() => toGb(totalBytes.value));
const usedGb = computed(() => toGb(usedBytes.value));
const availableGb = computed(() => toGb(availableBytes.value));

// 只有总量与可用量都有效时才计算剩余百分比。
const availablePercent = computed(() => {
  const total = totalBytes.value;
  const available = availableBytes.value;
  if (total === null || available === null || total <= 0) return '—';
  return Math.max(0, Math.min(100, (available / total) * 100)).toFixed(1);
});
</script>

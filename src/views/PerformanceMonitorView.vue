<template>
  <!-- 硬件设备与性能实时监控面板 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-sky-500/20 text-sky-400 border border-sky-500/30 flex items-center justify-center">
              <Activity class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">硬件配置与实时性能监控</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            原生采集 CPU、GPU、物理内存、磁盘分区及网络适配器实时吞吐状态与走势图。
          </p>
        </div>

        <div class="flex items-center gap-3">
          <div class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-white/[0.03] border border-white/10 text-xs text-gray-400">
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>实时刷新 (1.5s)</span>
          </div>

          <button
            @click="fetchData"
            type="button"
            class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] active:bg-white/[0.15] border border-white/10 transition-all"
            title="手动刷新硬件状态"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isFetching }" />
            <span>刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主滚动内容区 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar space-y-6">
      <!-- 1. 核心处理器与图形设备卡片 -->
      <section class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- CPU 处理器卡片 -->
        <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="w-8 h-8 rounded-xl bg-blue-500/10 text-blue-400 border border-blue-500/20 flex items-center justify-center">
                <Cpu class="w-4 h-4" />
              </div>
              <div>
                <h2 class="text-xs font-semibold text-gray-300">中央处理器 (CPU)</h2>
                <p class="text-[10px] text-gray-500">{{ perfData.cpu_logical_cores }} 逻辑核心</p>
              </div>
            </div>
            <span class="text-xl font-bold font-mono text-blue-400">
              {{ perfData.cpu_usage_percent.toFixed(1) }}%
            </span>
          </div>

          <p class="text-xs font-semibold text-gray-100 truncate" :title="perfData.cpu_name">
            {{ perfData.cpu_name }}
          </p>

          <!-- CPU 实时折线图 (SVG 走势波形) -->
          <div class="h-20 w-full pt-1">
            <svg class="w-full h-full overflow-visible" preserveAspectRatio="none" viewBox="0 0 100 40">
              <defs>
                <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.4" />
                  <stop offset="100%" stop-color="#3b82f6" stop-opacity="0.0" />
                </linearGradient>
              </defs>
              <polygon :points="cpuAreaPoints" fill="url(#cpuGradient)" />
              <polyline
                :points="cpuLinePoints"
                fill="none"
                stroke="#3b82f6"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
        </div>

        <!-- GPU 显卡卡片 -->
        <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="w-8 h-8 rounded-xl bg-purple-500/10 text-purple-400 border border-purple-500/20 flex items-center justify-center">
                <Monitor class="w-4 h-4" />
              </div>
              <div>
                <h2 class="text-xs font-semibold text-gray-300">图形显示核心 (GPU)</h2>
                <p class="text-[10px] text-gray-500">主显示渲染适配器</p>
              </div>
            </div>
            <span class="px-2.5 py-0.5 rounded-full text-[11px] font-medium bg-purple-500/10 text-purple-300 border border-purple-500/20">
              DirectX 就绪
            </span>
          </div>

          <p class="text-xs font-semibold text-gray-100 truncate" :title="perfData.gpu_name">
            {{ perfData.gpu_name }}
          </p>

          <div class="grid grid-cols-2 gap-3 pt-2">
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
              <span class="text-[10px] text-gray-500">驱动状态</span>
              <p class="text-xs font-medium text-emerald-400">运行良好</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
              <span class="text-[10px] text-gray-500">显示模式</span>
              <p class="text-xs font-medium text-gray-300">WDDM 硬件加速</p>
            </div>
          </div>
        </div>
      </section>

      <!-- 2. 物理内存与网络速率 -->
      <section class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- 内存状态 -->
        <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="w-8 h-8 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center justify-center">
                <Layers class="w-4 h-4" />
              </div>
              <div>
                <h2 class="text-xs font-semibold text-gray-300">系统物理运行内存</h2>
                <p class="text-[10px] text-gray-500">
                  {{ formatBytes(perfData.memory.used_ram) }} / {{ formatBytes(perfData.memory.total_ram) }}
                </p>
              </div>
            </div>
            <span class="text-xl font-bold font-mono text-emerald-400">
              {{ perfData.memory.usage_percent.toFixed(1) }}%
            </span>
          </div>

          <!-- 内存走势图 -->
          <div class="h-20 w-full pt-1">
            <svg class="w-full h-full overflow-visible" preserveAspectRatio="none" viewBox="0 0 100 40">
              <defs>
                <linearGradient id="memGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#10b981" stop-opacity="0.4" />
                  <stop offset="100%" stop-color="#10b981" stop-opacity="0.0" />
                </linearGradient>
              </defs>
              <polygon :points="memAreaPoints" fill="url(#memGradient)" />
              <polyline
                :points="memLinePoints"
                fill="none"
                stroke="#10b981"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
        </div>

        <!-- 实时网络流量 -->
        <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="w-8 h-8 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/20 flex items-center justify-center">
                <Network class="w-4 h-4" />
              </div>
              <div class="min-w-0">
                <h2 class="text-xs font-semibold text-gray-300">网络连接吞吐量</h2>
                <p class="text-[10px] text-gray-500 truncate max-w-[180px]" :title="perfData.network.adapter_name">
                  {{ perfData.network.adapter_name }}
                </p>
              </div>
            </div>

            <div class="flex items-center gap-3 text-right">
              <div>
                <span class="text-[10px] text-gray-500 block">↓ 下行速率</span>
                <span class="text-xs font-mono font-bold text-sky-400">
                  {{ formatSpeed(perfData.network.rx_speed_bps) }}
                </span>
              </div>
              <div>
                <span class="text-[10px] text-gray-500 block">↑ 上行速率</span>
                <span class="text-xs font-mono font-bold text-amber-400">
                  {{ formatSpeed(perfData.network.tx_speed_bps) }}
                </span>
              </div>
            </div>
          </div>

          <!-- 网络流量走势图 -->
          <div class="h-20 w-full pt-1">
            <svg class="w-full h-full overflow-visible" preserveAspectRatio="none" viewBox="0 0 100 40">
              <defs>
                <linearGradient id="netGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.4" />
                  <stop offset="100%" stop-color="#f59e0b" stop-opacity="0.0" />
                </linearGradient>
              </defs>
              <polygon :points="netAreaPoints" fill="url(#netGradient)" />
              <polyline
                :points="netLinePoints"
                fill="none"
                stroke="#f59e0b"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
        </div>
      </section>

      <!-- 3. 存储磁盘分区详情 -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-4">
        <div class="flex items-center justify-between border-b border-white/5 pb-3">
          <div class="flex items-center gap-2">
            <HardDrive class="w-4 h-4 text-sky-400" />
            <h2 class="text-sm font-semibold text-gray-200">存储磁盘与卷分区</h2>
          </div>
          <span class="text-xs text-gray-400">检测到 {{ perfData.disks.length }} 个逻辑分区</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div
            v-for="disk in perfData.disks"
            :key="disk.letter"
            class="p-4 rounded-xl bg-white/[0.02] border border-white/5 hover:border-white/15 transition-all space-y-2.5"
          >
            <div class="flex items-center justify-between">
              <span class="text-xs font-bold text-gray-200">{{ disk.label }}</span>
              <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-white/5 text-gray-400 border border-white/10">
                {{ disk.file_system }}
              </span>
            </div>

            <!-- 进度条 -->
            <div class="w-full h-2 rounded-full bg-white/10 overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-500"
                :class="disk.usage_percent > 85 ? 'bg-rose-500' : disk.usage_percent > 70 ? 'bg-amber-500' : 'bg-blue-500'"
                :style="{ width: `${disk.usage_percent}%` }"
              ></div>
            </div>

            <div class="flex items-center justify-between text-[11px] text-gray-400">
              <span>空闲 {{ formatBytes(disk.available_bytes) }}</span>
              <span class="font-mono text-gray-300">共 {{ formatBytes(disk.total_bytes) }} ({{ disk.usage_percent }}%)</span>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file PerformanceMonitorView.vue
 * @description 系统硬件设备信息与性能实时监控面板
 */

import { ref, computed, onMounted, onUnmounted } from 'vue';
import {
  Activity,
  RefreshCw,
  Cpu,
  Monitor,
  Layers,
  Network,
  HardDrive,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { HardwarePerformance } from '../types/module';

const isFetching = ref(false);

const perfData = ref<HardwarePerformance>({
  cpu_name: '检测中...',
  cpu_logical_cores: 8,
  cpu_usage_percent: 15.0,
  gpu_name: '检测中...',
  memory: {
    total_ram: 16 * 1024 * 1024 * 1024,
    available_ram: 8 * 1024 * 1024 * 1024,
    used_ram: 8 * 1024 * 1024 * 1024,
    usage_percent: 50.0,
  },
  disks: [],
  network: {
    adapter_name: '正在检测网卡...',
    rx_speed_bps: 0,
    tx_speed_bps: 0,
    total_rx_bytes: 0,
    total_tx_bytes: 0,
  },
});

// 历史波形队列 (保留最近 25 个点)
const MAX_POINTS = 25;
const cpuHistory = ref<number[]>(new Array(MAX_POINTS).fill(15));
const memHistory = ref<number[]>(new Array(MAX_POINTS).fill(50));
const netHistory = ref<number[]>(new Array(MAX_POINTS).fill(0));

let timer: ReturnType<typeof setInterval> | null = null;

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
}

function formatSpeed(bps: number): string {
  if (!bps || bps <= 0) return '0 KB/s';
  const kb = bps / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  const mb = kb / 1024;
  return `${mb.toFixed(2)} MB/s`;
}

// 生成 SVG 折线与填充多边形点集
function buildSvgPoints(history: number[], maxVal = 100) {
  const step = 100 / (MAX_POINTS - 1);
  const linePoints = history
    .map((val, idx) => {
      const x = idx * step;
      const normalized = Math.min(maxVal, Math.max(0, val));
      const y = 40 - (normalized / maxVal) * 38;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');

  const areaPoints = `0,40 ${linePoints} 100,40`;
  return { linePoints, areaPoints };
}

const cpuLinePoints = computed(() => buildSvgPoints(cpuHistory.value, 100).linePoints);
const cpuAreaPoints = computed(() => buildSvgPoints(cpuHistory.value, 100).areaPoints);

const memLinePoints = computed(() => buildSvgPoints(memHistory.value, 100).linePoints);
const memAreaPoints = computed(() => buildSvgPoints(memHistory.value, 100).areaPoints);

const netLinePoints = computed(() => {
  const maxNet = Math.max(1024 * 100, ...netHistory.value);
  return buildSvgPoints(netHistory.value, maxNet).linePoints;
});
const netAreaPoints = computed(() => {
  const maxNet = Math.max(1024 * 100, ...netHistory.value);
  return buildSvgPoints(netHistory.value, maxNet).areaPoints;
});

async function fetchData() {
  if (isFetching.value) return;
  isFetching.value = true;
  try {
    if (isTauri()) {
      const snap = await invoke<HardwarePerformance>('get_performance_snapshot');
      perfData.value = snap;

      // 推入历史队列
      cpuHistory.value.push(snap.cpu_usage_percent);
      if (cpuHistory.value.length > MAX_POINTS) cpuHistory.value.shift();

      memHistory.value.push(snap.memory.usage_percent);
      if (memHistory.value.length > MAX_POINTS) memHistory.value.shift();

      netHistory.value.push(snap.network.rx_speed_bps);
      if (netHistory.value.length > MAX_POINTS) netHistory.value.shift();
    } else {
      // 浏览器 Mock 演示数据
      const rndCpu = Math.min(95, Math.max(5, 20 + (Math.random() - 0.5) * 20));
      perfData.value.cpu_usage_percent = rndCpu;
      cpuHistory.value.push(rndCpu);
      if (cpuHistory.value.length > MAX_POINTS) cpuHistory.value.shift();

      const rndMem = Math.min(95, Math.max(20, 52 + (Math.random() - 0.5) * 5));
      perfData.value.memory.usage_percent = rndMem;
      memHistory.value.push(rndMem);
      if (memHistory.value.length > MAX_POINTS) memHistory.value.shift();

      const rndNet = Math.floor(Math.random() * 1024 * 1024 * 2);
      perfData.value.network.rx_speed_bps = rndNet;
      netHistory.value.push(rndNet);
      if (netHistory.value.length > MAX_POINTS) netHistory.value.shift();
    }
  } catch (err) {
    console.error('获取硬件性能失败:', err);
  } finally {
    isFetching.value = false;
  }
}

onMounted(() => {
  fetchData();
  timer = setInterval(fetchData, 1500);
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
});
</script>

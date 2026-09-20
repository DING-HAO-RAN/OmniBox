<template>
  <!-- FDM 经典多线程分片热力图矩阵组件 -->
  <div class="space-y-1.5 select-none">
    <div class="flex items-center justify-between text-[10px] text-gray-400 font-mono">
      <span class="flex items-center gap-1.5">
        <span class="w-1.5 h-1.5 rounded-full bg-blue-400"></span>
        <span>分块传输拓扑 ({{ chunks.length }} 线程分片)</span>
      </span>
      <span>已完成 {{ finishedCount }} / {{ chunks.length }} 块</span>
    </div>

    <!-- 连续分片微型条形矩阵 -->
    <div class="flex items-center gap-1 w-full h-3.5 bg-black/40 p-0.5 rounded-md border border-white/5 overflow-hidden">
      <div
        v-for="chunk in chunks"
        :key="chunk.id"
        class="h-full rounded-sm transition-all duration-300 relative group flex-1 min-w-[6px] overflow-hidden"
        :class="getChunkBgClass(chunk)"
        :title="getChunkTooltip(chunk)"
      >
        <!-- 正在下载时的分片内部进度填充条 -->
        <div
          v-if="!chunk.is_finished && chunk.downloaded > 0"
          class="h-full bg-blue-400/90 transition-all duration-200"
          :style="{ width: `${getChunkPercent(chunk)}%` }"
        ></div>

        <!-- 悬浮精美 Tooltip 浮层 -->
        <div
          class="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 hidden group-hover:flex flex-col items-center pointer-events-none z-30"
        >
          <div
            class="px-2.5 py-1.5 rounded-lg bg-[#1f242d] border border-white/10 text-[10px] font-mono text-gray-200 whitespace-nowrap shadow-xl shadow-black/80 space-y-0.5"
          >
            <div class="flex items-center justify-between gap-3 font-bold text-white">
              <span>分片 #{{ chunk.id + 1 }}</span>
              <span :class="chunk.is_finished ? 'text-emerald-400' : 'text-blue-400'">
                {{ getChunkPercent(chunk) }}%
              </span>
            </div>
            <div class="text-gray-400 text-[9px]">
              {{ formatBytes(chunk.downloaded) }} / {{ formatBytes(getChunkSize(chunk)) }}
            </div>
            <div class="text-[8px] text-gray-500">
              偏移: {{ chunk.start }} ~ {{ chunk.end }}
            </div>
          </div>
          <!-- 小三角形箭头 -->
          <div class="w-0 h-0 border-x-4 border-x-transparent border-t-4 border-t-[#1f242d] -mt-px"></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { DownloadChunk } from '../../types/module';

const props = defineProps<{
  chunks: DownloadChunk[];
  totalBytes?: number;
  isDownloading?: boolean;
}>();

function getChunkSize(chunk: DownloadChunk): number {
  return Math.max(1, chunk.end - chunk.start + 1);
}

function getChunkPercent(chunk: DownloadChunk): number {
  if (chunk.is_finished) return 100;
  const size = getChunkSize(chunk);
  return Math.min(100, Math.round((chunk.downloaded / size) * 100));
}

function getChunkBgClass(chunk: DownloadChunk): string {
  if (chunk.is_finished) {
    return 'bg-gradient-to-r from-emerald-500 to-teal-500 shadow-[0_0_6px_rgba(16,185,129,0.3)]';
  }
  if (chunk.downloaded > 0) {
    return 'bg-blue-600/40 border border-blue-400/40 animate-pulse';
  }
  return 'bg-white/[0.04] border border-white/5';
}

function getChunkTooltip(chunk: DownloadChunk): string {
  return `分片 #${chunk.id + 1}: ${getChunkPercent(chunk)}% (${formatBytes(chunk.downloaded)} / ${formatBytes(getChunkSize(chunk))})`;
}

const finishedCount = computed(() => {
  return props.chunks.filter((c) => c.is_finished).length;
});

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}
</script>

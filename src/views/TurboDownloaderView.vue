<template>
  <!-- 高速多线程下载器 (TurboDownloader) 主控制中心 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-5 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-8 h-8 rounded-xl bg-gradient-to-br from-blue-500/20 to-sky-500/20 text-blue-400 border border-blue-500/30 flex items-center justify-center shadow-lg shadow-blue-500/10">
              <DownloadCloud class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">高速多线程下载器</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            Windows 原生 WinHTTP 驱动 · 零碎片直接 Seek 并发写入 · 断点续传 · FDM 经典分片热力图。
          </p>
        </div>

        <!-- 顶部操作区与统计指标 -->
        <div class="flex items-center gap-3">
          <!-- 全局瞬时总速率胶囊 -->
          <div class="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-xl bg-white/[0.03] border border-white/10 text-xs font-mono">
            <span
              class="w-2 h-2 rounded-full"
              :class="totalSpeedBps > 0 ? 'bg-blue-400 animate-ping' : 'bg-gray-500'"
            ></span>
            <span class="text-gray-400">总速率:</span>
            <span class="font-bold text-white" :class="totalSpeedBps > 0 ? 'text-sky-400' : 'text-gray-400'">
              {{ formatSpeed(totalSpeedBps) }}
            </span>
          </div>

          <!-- 新建下载按钮 -->
          <button
            @click="newModalVisible = true"
            type="button"
            class="px-5 py-2 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-blue-600 to-sky-500 hover:from-blue-500 hover:to-sky-400 active:scale-95 shadow-lg shadow-blue-500/20 transition-all flex items-center gap-1.5"
          >
            <Plus class="w-3.5 h-3.5" />
            <span>新建下载任务</span>
          </button>
        </div>
      </div>

      <!-- 任务筛选标签行 -->
      <div class="flex items-center justify-between mt-4">
        <div class="inline-flex p-1 rounded-xl bg-black/40 border border-white/10 text-xs">
          <button
            v-for="tab in filterTabs"
            :key="tab.id"
            @click="activeFilter = tab.id"
            type="button"
            class="px-3 py-1 rounded-lg font-medium transition-all"
            :class="activeFilter === tab.id ? 'bg-blue-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'"
          >
            <span>{{ tab.label }}</span>
            <span class="text-[10px] font-mono opacity-80 ml-1">({{ getFilterCount(tab.id) }})</span>
          </button>
        </div>

        <div class="text-[11px] text-gray-500 font-mono">
          共 {{ tasks.length }} 个任务 · 已完成 {{ completedCount }}
        </div>
      </div>
    </header>

    <!-- 任务卡片滚动列表区 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-5 win11-scrollbar space-y-4">
      <!-- 空列表引导 -->
      <div
        v-if="filteredTasks.length === 0"
        class="h-64 rounded-3xl border border-white/5 bg-white/[0.01] flex flex-col items-center justify-center text-center space-y-3"
      >
        <div class="w-12 h-12 rounded-2xl bg-blue-500/10 text-blue-400 border border-blue-500/20 flex items-center justify-center">
          <DownloadCloud class="w-6 h-6" />
        </div>
        <div class="max-w-xs">
          <h3 class="text-sm font-semibold text-gray-200">暂无下载任务</h3>
          <p class="text-xs text-gray-500 mt-1">
            点击上方“新建下载任务”按钮，粘贴 URL 即可体验多线程全速并发下载与 FDM 热力图。
          </p>
        </div>
      </div>

      <!-- 任务卡片列表 -->
      <div
        v-for="task in filteredTasks"
        :key="task.id"
        class="group rounded-2xl p-5 bg-white/[0.025] hover:bg-white/[0.045] border border-white/10 hover:border-white/20 transition-all space-y-3.5 shadow-xl backdrop-blur-md"
      >
        <!-- 卡片头部：文件名、状态胶囊、右上角操作 -->
        <div class="flex items-start justify-between gap-4">
          <div class="flex items-start gap-3 min-w-0">
            <div class="w-10 h-10 rounded-xl bg-white/[0.03] border border-white/10 flex items-center justify-center text-blue-400 flex-shrink-0 mt-0.5">
              <FileDown class="w-5 h-5" />
            </div>
            <div class="min-w-0 space-y-1">
              <div class="flex items-center gap-2">
                <h3 class="text-sm font-bold text-gray-100 truncate" :title="task.file_name">
                  {{ task.file_name }}
                </h3>
                <!-- 状态徽章 -->
                <span
                  class="px-2 py-0.5 rounded-full text-[10px] font-medium border flex-shrink-0"
                  :class="getStatusBadgeClass(task.status)"
                >
                  {{ getStatusText(task.status) }}
                </span>
                <span
                  v-if="task.supports_range"
                  class="px-1.5 py-0.2 rounded text-[9px] font-mono bg-blue-500/15 text-blue-300 border border-blue-500/30 flex-shrink-0"
                >
                  {{ task.thread_count }} 线程
                </span>
              </div>
              <p class="text-[11px] font-mono text-gray-400 truncate max-w-[500px]" :title="task.save_path">
                {{ task.save_path }}
              </p>
            </div>
          </div>

          <!-- 右上角快捷操作按钮组 -->
          <div class="flex items-center gap-2 flex-shrink-0">
            <!-- 暂停按钮 -->
            <button
              v-if="task.status === 'Downloading' || task.status === 'Pending'"
              @click="handlePause(task.id)"
              type="button"
              class="px-3 py-1.5 rounded-lg text-xs font-medium text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 transition-all flex items-center gap-1"
              title="暂停任务"
            >
              <Pause class="w-3.5 h-3.5" />
              <span>暂停</span>
            </button>

            <!-- 继续按钮 -->
            <button
              v-else-if="task.status === 'Paused' || task.status === 'Failed'"
              @click="handleResume(task.id)"
              type="button"
              class="px-3 py-1.5 rounded-lg text-xs font-medium text-emerald-300 bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/30 transition-all flex items-center gap-1"
              title="继续断点下载"
            >
              <Play class="w-3.5 h-3.5" />
              <span>继续</span>
            </button>

            <!-- 打开文件 -->
            <button
              v-if="task.status === 'Completed'"
              @click="handleOpenFile(task.save_path)"
              type="button"
              class="px-3 py-1.5 rounded-lg text-xs font-medium text-sky-300 bg-sky-500/10 hover:bg-sky-500/20 border border-sky-500/30 transition-all flex items-center gap-1"
              title="直接打开已下载文件"
            >
              <ExternalLink class="w-3.5 h-3.5" />
              <span>打开</span>
            </button>

            <!-- 定位文件夹 -->
            <button
              @click="handleOpenFolder(task.save_path)"
              type="button"
              class="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-white/5 border border-white/5 transition-all"
              title="在 Windows 资源管理器中显示"
            >
              <FolderOpen class="w-3.5 h-3.5" />
            </button>

            <!-- 删除任务 -->
            <button
              @click="promptDelete(task)"
              type="button"
              class="p-1.5 rounded-lg text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20 transition-all"
              title="删除任务"
            >
              <Trash2 class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        <!-- 主进度条与进度指标 -->
        <div class="space-y-1.5">
          <!-- 主进度条 -->
          <div class="w-full h-2 rounded-full bg-white/10 overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="task.status === 'Completed' ? 'bg-emerald-500' : task.status === 'Paused' ? 'bg-amber-500' : task.status === 'Failed' ? 'bg-rose-500' : 'bg-gradient-to-r from-blue-600 via-sky-500 to-indigo-500'"
              :style="{ width: `${task.progress_percent}%` }"
            ></div>
          </div>

          <!-- 指标文字：已下载/总大小、百分比、速率、ETA -->
          <div class="flex items-center justify-between text-[11px] font-mono text-gray-400">
            <div class="flex items-center gap-3">
              <span>{{ formatBytes(task.downloaded_bytes) }} / {{ formatBytes(task.total_bytes) }}</span>
              <span class="font-bold text-white">({{ task.progress_percent.toFixed(1) }}%)</span>
            </div>

            <div class="flex items-center gap-3">
              <span v-if="task.status === 'Downloading'" class="text-sky-400 font-bold">
                ↓ {{ formatSpeed(task.speed_bps) }}
              </span>
              <span v-if="task.status === 'Downloading' && task.eta_seconds > 0" class="text-gray-400">
                剩余: {{ formatEta(task.eta_seconds) }}
              </span>
              <span v-if="task.error_message" class="text-rose-400 truncate max-w-[200px]" :title="task.error_message">
                {{ task.error_message }}
              </span>
            </div>
          </div>
        </div>

        <!-- FDM 经典多线程分片热力图可视化 -->
        <div v-if="task.chunks && task.chunks.length > 0" class="pt-1">
          <ChunkHeatmap
            :chunks="task.chunks"
            :total-bytes="task.total_bytes"
            :is-downloading="task.status === 'Downloading'"
          />
        </div>
      </div>
    </main>

    <!-- 新建下载弹窗 -->
    <NewDownloadModal
      :visible="newModalVisible"
      @close="newModalVisible = false"
      @created="fetchTasks"
    />

    <!-- 删除任务确认弹窗 -->
    <div
      v-if="deleteModalTask"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-md"
      @click.self="deleteModalTask = null"
    >
      <div class="w-full max-w-sm rounded-3xl p-6 bg-[#161b22] border border-white/10 shadow-2xl space-y-4">
        <h3 class="text-sm font-bold text-white">确认删除下载任务？</h3>
        <p class="text-xs text-gray-400 break-all">
          任务: {{ deleteModalTask.file_name }}
        </p>
        <label class="flex items-center gap-2 text-xs text-gray-300 cursor-pointer pt-1">
          <input type="checkbox" v-model="deleteLocalFile" class="rounded bg-black/40 border-white/20 accent-rose-500" />
          <span>同时从磁盘中永久删除已下载的文件</span>
        </label>
        <div class="flex items-center justify-end gap-2.5 pt-2 border-t border-white/5">
          <button
            @click="deleteModalTask = null"
            type="button"
            class="px-4 py-2 rounded-xl text-xs text-gray-400 hover:text-white hover:bg-white/5"
          >
            取消
          </button>
          <button
            @click="confirmDelete"
            type="button"
            class="px-5 py-2 rounded-xl text-xs font-semibold text-white bg-rose-600 hover:bg-rose-500 shadow-lg shadow-rose-600/20"
          >
            删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * @file TurboDownloaderView.vue
 * @description Windows 11 Fluent 风格的高速多线程下载器控制中心
 * 集成 FDM 经典分片热力图、任务生命周期调度与 WinHTTP 零碎片引擎。
 */

import { ref, computed, onMounted, onUnmounted } from 'vue';
import {
  DownloadCloud,
  Plus,
  FileDown,
  Pause,
  Play,
  ExternalLink,
  FolderOpen,
  Trash2,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { DownloadTask, DownloadTaskStatus } from '../types/module';
import { useToast } from '../composables/useToast';
import ChunkHeatmap from '../components/downloader/ChunkHeatmap.vue';
import NewDownloadModal from '../components/downloader/NewDownloadModal.vue';

const { toast } = useToast();

const tasks = ref<DownloadTask[]>([]);
const activeFilter = ref<'all' | 'downloading' | 'paused' | 'completed'>('all');
const newModalVisible = ref(false);

const deleteModalTask = ref<DownloadTask | null>(null);
const deleteLocalFile = ref(false);

let pollTimer: ReturnType<typeof setTimeout> | null = null;
let isComponentAlive = true;

const filterTabs = [
  { id: 'all' as const, label: '全部' },
  { id: 'downloading' as const, label: '下载中' },
  { id: 'paused' as const, label: '已暂停' },
  { id: 'completed' as const, label: '已完成' },
];

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

function formatSpeed(bps: number): string {
  if (!bps || bps <= 0) return '0 KB/s';
  const kb = bps / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  const mb = kb / 1024;
  return `${mb.toFixed(2)} MB/s`;
}

function formatEta(seconds: number): string {
  if (!seconds || seconds <= 0) return '--';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

function getStatusBadgeClass(status: DownloadTaskStatus): string {
  switch (status) {
    case 'Downloading':
      return 'bg-blue-500/15 text-blue-300 border-blue-500/30';
    case 'Paused':
      return 'bg-amber-500/15 text-amber-300 border-amber-500/30';
    case 'Completed':
      return 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30';
    case 'Failed':
      return 'bg-rose-500/15 text-rose-300 border-rose-500/30';
    default:
      return 'bg-gray-500/15 text-gray-400 border-gray-500/30';
  }
}

function getStatusText(status: DownloadTaskStatus): string {
  switch (status) {
    case 'Downloading':
      return '下载中';
    case 'Paused':
      return '已暂停';
    case 'Completed':
      return '已完成';
    case 'Failed':
      return '下载失败';
    case 'Pending':
      return '等待中';
    case 'Cancelled':
      return '已取消';
  }
}

const filteredTasks = computed(() => {
  return tasks.value.filter((t) => {
    if (activeFilter.value === 'all') return true;
    if (activeFilter.value === 'downloading') return t.status === 'Downloading' || t.status === 'Pending';
    if (activeFilter.value === 'paused') return t.status === 'Paused';
    if (activeFilter.value === 'completed') return t.status === 'Completed';
    return true;
  });
});

function getFilterCount(filter: 'all' | 'downloading' | 'paused' | 'completed'): number {
  if (filter === 'all') return tasks.value.length;
  if (filter === 'downloading') {
    return tasks.value.filter((t) => t.status === 'Downloading' || t.status === 'Pending').length;
  }
  if (filter === 'paused') {
    return tasks.value.filter((t) => t.status === 'Paused').length;
  }
  if (filter === 'completed') {
    return tasks.value.filter((t) => t.status === 'Completed').length;
  }
  return 0;
}

const completedCount = computed(() => {
  return tasks.value.filter((t) => t.status === 'Completed').length;
});

const totalSpeedBps = computed(() => {
  return tasks.value
    .filter((t) => t.status === 'Downloading')
    .reduce((sum, t) => sum + (t.speed_bps || 0), 0);
});

async function fetchTasks() {
  if (isTauri()) {
    try {
      const list = await invoke<DownloadTask[]>('downloader_list_tasks');
      tasks.value = list;
    } catch (err) {
      console.error('获取下载任务列表失败:', err);
    }
  } else {
    // 浏览器 Mock 数据
    if (tasks.value.length === 0) {
      tasks.value = [
        {
          id: 'mock-1',
          url: 'https://example.com/archive.zip',
          file_name: 'vscode_windows_x64.zip',
          save_path: 'C:\\Users\\User\\Downloads\\vscode_windows_x64.zip',
          total_bytes: 128 * 1024 * 1024,
          downloaded_bytes: 64 * 1024 * 1024,
          progress_percent: 50.0,
          speed_bps: 14 * 1024 * 1024,
          eta_seconds: 5,
          status: 'Downloading',
          thread_count: 8,
          supports_range: true,
          created_at: Date.now(),
          chunks: [
            { id: 0, start: 0, end: 16777215, downloaded: 16777216, is_finished: true },
            { id: 1, start: 16777216, end: 33554431, downloaded: 16777216, is_finished: true },
            { id: 2, start: 33554432, end: 50331647, downloaded: 10000000, is_finished: false },
            { id: 3, start: 50331648, end: 67108863, downloaded: 8000000, is_finished: false },
            { id: 4, start: 67108864, end: 83886079, downloaded: 5000000, is_finished: false },
            { id: 5, start: 83886080, end: 100663295, downloaded: 4000000, is_finished: false },
            { id: 6, start: 100663296, end: 117440511, downloaded: 3000000, is_finished: false },
            { id: 7, start: 117440512, end: 134217727, downloaded: 2000000, is_finished: false },
          ],
        },
      ];
    }
  }
}

async function handlePause(id: string) {
  try {
    if (isTauri()) {
      await invoke('downloader_pause_task', { id });
    } else {
      const t = tasks.value.find((i) => i.id === id);
      if (t) t.status = 'Paused';
    }
    toast.info('任务已暂停');
    await fetchTasks();
  } catch (err) {
    toast.error('暂停失败', String(err));
  }
}

async function handleResume(id: string) {
  try {
    if (isTauri()) {
      await invoke('downloader_resume_task', { id });
    } else {
      const t = tasks.value.find((i) => i.id === id);
      if (t) t.status = 'Downloading';
    }
    toast.success('已恢复下载');
    await fetchTasks();
  } catch (err) {
    toast.error('恢复失败', String(err));
  }
}

async function handleOpenFile(path: string) {
  try {
    if (isTauri()) {
      await invoke('downloader_open_file', { path });
    } else {
      toast.info('模拟打开文件');
    }
  } catch (err) {
    toast.error('打开文件失败', String(err));
  }
}

async function handleOpenFolder(path: string) {
  try {
    if (isTauri()) {
      await invoke('downloader_open_folder', { path });
    } else {
      toast.info('模拟打开所在文件夹');
    }
  } catch (err) {
    toast.error('定位文件夹失败', String(err));
  }
}

function promptDelete(task: DownloadTask) {
  deleteModalTask.value = task;
  deleteLocalFile.value = false;
}

async function confirmDelete() {
  if (!deleteModalTask.value) return;
  const id = deleteModalTask.value.id;
  const delFile = deleteLocalFile.value;
  deleteModalTask.value = null;

  try {
    if (isTauri()) {
      await invoke('downloader_delete_task', { id, deleteFile: delFile });
    } else {
      tasks.value = tasks.value.filter((i) => i.id !== id);
    }
    toast.info('任务已删除');
    await fetchTasks();
  } catch (err) {
    toast.error('删除任务失败', String(err));
  }
}

async function pollLoop() {
  if (!isComponentAlive) return;
  await fetchTasks();
  if (!isComponentAlive) return;

  // 动态频率轮询：有活跃下载任务时 600ms 刷新保证热力图顺畅，空闲时降频至 2000ms 减少 IPC
  const hasActive = tasks.value.some((t) => t.status === 'Downloading' || t.status === 'Pending');
  const delay = hasActive ? 600 : 2000;
  pollTimer = setTimeout(pollLoop, delay);
}

onMounted(() => {
  isComponentAlive = true;
  pollLoop();
});

onUnmounted(() => {
  isComponentAlive = false;
  if (pollTimer) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
});
</script>

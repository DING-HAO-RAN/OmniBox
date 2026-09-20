<template>
  <!-- 新建高速下载任务弹窗 (Fluent Design 拟态对话框) -->
  <div
    v-if="visible"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-md transition-all duration-200"
    @click.self="handleClose"
  >
    <div
      class="w-full max-w-lg rounded-3xl p-6 bg-[#161b22]/95 border border-white/10 shadow-2xl shadow-black/80 flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- 弹窗标题栏 -->
      <div class="flex items-center justify-between border-b border-white/5 pb-3">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-xl bg-blue-500/20 text-blue-400 border border-blue-500/30 flex items-center justify-center shadow-lg shadow-blue-500/10">
            <DownloadCloud class="w-4 h-4" />
          </div>
          <div>
            <h3 class="text-sm font-bold text-white">新建多线程高速下载任务</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              原生 WinHTTP 网络栈 · 零碎片直接 Seek 并行写入
            </p>
          </div>
        </div>
        <button
          @click="handleClose"
          type="button"
          class="text-gray-400 hover:text-white p-1 rounded-lg hover:bg-white/5 transition-colors"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- 表单输入区域 -->
      <div class="space-y-4 pt-1 text-xs">
        <!-- 1. URL 输入框 -->
        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <label class="text-gray-300 font-medium">下载链接 (URL)：</label>
            <span v-if="isProbing" class="text-[11px] text-blue-400 flex items-center gap-1 font-mono">
              <Loader2 class="w-3 h-3 animate-spin" />
              <span>探测元数据中...</span>
            </span>
          </div>
          <div class="relative">
            <input
              ref="urlInputRef"
              v-model="url"
              type="text"
              placeholder="https://... 或 http://..."
              class="w-full h-10 px-3.5 pr-20 rounded-xl bg-black/40 border border-white/10 focus:border-blue-500/70 focus:ring-2 focus:ring-blue-500/20 text-white placeholder-gray-500 outline-none font-mono text-[11px] transition-all"
              @blur="handleUrlBlur"
              @keydown.enter="handleUrlBlur"
            />
            <button
              @click="handleManualProbe"
              type="button"
              :disabled="!url.trim() || isProbing"
              class="absolute right-1.5 top-1.5 h-7 px-2.5 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 font-medium transition-colors flex items-center gap-1 disabled:opacity-40"
            >
              <Search class="w-3 h-3" />
              <span>探测</span>
            </button>
          </div>

          <!-- 探测结果状态反馈徽章 -->
          <div v-if="probeResult" class="p-2.5 rounded-xl bg-white/[0.02] border border-white/5 flex items-center justify-between text-[11px]">
            <div class="flex items-center gap-2">
              <span
                class="px-2 py-0.5 rounded text-[10px] font-medium"
                :class="probeResult.supports_range ? 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30' : 'bg-amber-500/15 text-amber-300 border border-amber-500/30'"
              >
                {{ probeResult.supports_range ? '● 支持多线程分块加速' : '● 单线程流式下载' }}
              </span>
            </div>
            <span class="font-mono text-gray-300 font-semibold">
              大小: {{ formatBytes(probeResult.total_bytes) }}
            </span>
          </div>
        </div>

        <!-- 2. 保存文件名 -->
        <div class="space-y-1.5">
          <label class="block text-gray-300 font-medium">保存文件名：</label>
          <input
            v-model="fileName"
            type="text"
            placeholder="自定义保存文件名 (留空则自动从 URL 或 Header 解析)"
            class="w-full h-9 px-3.5 rounded-xl bg-black/40 border border-white/10 focus:border-blue-500/70 focus:ring-2 focus:ring-blue-500/20 text-white placeholder-gray-500 outline-none font-mono text-[11px] transition-all"
          />
        </div>

        <!-- 3. 保存目录 -->
        <div class="space-y-1.5">
          <label class="block text-gray-300 font-medium">保存目录：</label>
          <div class="flex gap-2">
            <input
              v-model="saveDir"
              type="text"
              placeholder="默认下载文件夹 (留空使用系统 Downloads 目录)"
              class="flex-1 h-9 px-3 rounded-xl bg-black/40 border border-white/10 text-gray-200 font-mono text-[11px] truncate focus:outline-none focus:border-blue-500"
              :title="saveDir"
            />
            <button
              @click="handleChooseDir"
              type="button"
              class="px-3.5 py-1.5 rounded-xl bg-white/5 hover:bg-white/10 text-gray-300 font-medium whitespace-nowrap border border-white/10 transition-colors flex items-center gap-1"
            >
              <FolderOpen class="w-3.5 h-3.5" />
              <span>选择目录</span>
            </button>
          </div>
        </div>

        <!-- 4. 并发线程数滑块 -->
        <div class="space-y-2 pt-1 border-t border-white/5">
          <div class="flex items-center justify-between">
            <span class="text-gray-300 font-medium">并发分片线程数：</span>
            <span class="font-mono font-bold text-blue-400 text-sm">{{ threads }} 线程</span>
          </div>
          <input
            type="range"
            min="1"
            max="32"
            step="1"
            v-model.number="threads"
            class="w-full accent-blue-500 cursor-pointer"
          />
          <div class="flex items-center justify-between text-[10px] text-gray-500">
            <span>1 线程 (单连接)</span>
            <span>8 线程 (推荐)</span>
            <span>16 线程</span>
            <span>32 线程 (全速并发)</span>
          </div>
        </div>
      </div>

      <!-- 底部操作按钮 -->
      <div class="flex items-center justify-end gap-2.5 pt-3 border-t border-white/5">
        <button
          @click="handleClose"
          type="button"
          class="px-4 py-2 rounded-xl text-xs font-medium text-gray-400 hover:text-white hover:bg-white/5 transition-colors"
        >
          取消
        </button>
        <button
          @click="handleSubmit"
          :disabled="!url.trim() || isSubmitting"
          type="button"
          class="px-6 py-2 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-blue-600 via-sky-500 to-indigo-600 hover:from-blue-500 hover:via-sky-400 hover:to-indigo-500 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed shadow-lg shadow-blue-500/25 transition-all flex items-center gap-1.5"
        >
          <Loader2 v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
          <Download class="w-3.5 h-3.5" />
          <span>立即开始全速下载</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { DownloadCloud, X, Search, Loader2, FolderOpen, Download } from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { DownloadUrlMeta, NewDownloadTaskParams } from '../../types/module';
import { useToast } from '../../composables/useToast';

const { toast } = useToast();

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'created'): void;
}>();

const url = ref('');
const fileName = ref('');
const saveDir = ref('');
const threads = ref(8);

const isProbing = ref(false);
const isSubmitting = ref(false);
const probeResult = ref<DownloadUrlMeta | null>(null);
const urlInputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.visible,
  (val) => {
    if (val) {
      url.value = '';
      fileName.value = '';
      saveDir.value = '';
      threads.value = 8;
      probeResult.value = null;
      isProbing.value = false;
      isSubmitting.value = false;
      nextTick(() => {
        urlInputRef.value?.focus();
      });
    }
  }
);

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '未知大小';
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

async function probeUrl() {
  const u = url.value.trim();
  if (!u || isProbing.value) return;

  isProbing.value = true;
  try {
    if (isTauri()) {
      const meta = await invoke<DownloadUrlMeta>('downloader_probe_url', { url: u });
      probeResult.value = meta;
      if (!fileName.value && meta.suggested_filename) {
        fileName.value = meta.suggested_filename;
      }
    } else {
      // 浏览器 Mock
      probeResult.value = {
        total_bytes: 128 * 1024 * 1024,
        supports_range: true,
        suggested_filename: 'archive_package.zip',
      };
      if (!fileName.value) {
        fileName.value = 'archive_package.zip';
      }
    }
  } catch {
    // 探测失败不阻塞用户手动创建
  } finally {
    isProbing.value = false;
  }
}

function handleUrlBlur() {
  if (url.value.trim() && !probeResult.value) {
    probeUrl();
  }
}

function handleManualProbe() {
  probeUrl();
}

async function handleChooseDir() {
  if (isTauri()) {
    try {
      const selected = await invoke<string | null>('downloader_choose_dir');
      if (selected) {
        saveDir.value = selected;
      }
    } catch {
      // ignore
    }
  }
}

function handleClose() {
  emit('close');
}

async function handleSubmit() {
  const u = url.value.trim();
  if (!u) return;

  isSubmitting.value = true;
  try {
    const params: NewDownloadTaskParams = {
      url: u,
      file_name: fileName.value.trim() || null,
      save_dir: saveDir.value.trim() || null,
      threads: threads.value,
    };

    if (isTauri()) {
      await invoke('downloader_create_task', { params });
    }
    toast.success('下载任务已创建', `已开始拉取【${params.file_name || '文件'}】`);
    emit('created');
    emit('close');
  } catch (err) {
    toast.error('创建下载任务失败', String(err));
  } finally {
    isSubmitting.value = false;
  }
}
</script>

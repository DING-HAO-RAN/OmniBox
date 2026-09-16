<template>
  <!-- 深度文件隐藏密室主视图 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-purple-500/20 text-purple-400 border border-purple-500/30 flex items-center justify-center">
              <EyeOff class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">深度隐私密室 (超级隐藏)</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            利用 Windows 系统受保护属性，使文件/文件夹即使在开启“显示隐藏文件”时也绝对不可见；可联动一键启动器使用。
          </p>
        </div>

        <div class="flex items-center gap-2">
          <span class="text-xs text-purple-300/80 bg-purple-500/10 border border-purple-500/20 px-3 py-1.5 rounded-xl flex items-center gap-1.5 font-medium">
            <Shield class="w-3.5 h-3.5 text-purple-400" />
            已保护 {{ cloakedList.length }} 个私密项目
          </span>
        </div>
      </div>
    </header>

    <!-- 主展示区 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar space-y-6">
      <!-- 快速隐藏新项目卡片 -->
      <section class="rounded-2xl p-6 bg-white/[0.03] border border-white/10 shadow-xl backdrop-blur-md">
        <h2 class="text-sm font-semibold text-gray-200 flex items-center gap-2 mb-4">
          <Lock class="w-4 h-4 text-purple-400" />
          <span>添加需要深度隐藏的文件或文件夹</span>
        </h2>

        <div class="grid grid-cols-1 md:grid-cols-12 gap-3">
          <!-- 目标路径输入框 -->
          <div class="md:col-span-8 relative">
            <input
              v-model="inputPath"
              type="text"
              placeholder="请输入或选择文件/程序/文件夹路径 (例如 D:\Games\SecretApp.exe)..."
              class="w-full h-10 px-3.5 pr-24 rounded-xl bg-black/40 border border-white/10 hover:border-white/20 focus:border-purple-500/70 focus:ring-2 focus:ring-purple-500/20 text-xs text-gray-100 placeholder-gray-500 transition-all outline-none font-mono"
              @keydown.enter="handleCloakNew"
            />
            <button
              @click="handleBrowseFile"
              type="button"
              class="absolute right-1.5 top-1.5 h-7 px-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-[11px] text-gray-200 font-medium transition-colors flex items-center gap-1"
            >
              <FolderSearch class="w-3 h-3" />
              <span>选择文件</span>
            </button>
          </div>

          <!-- 备注 -->
          <div class="md:col-span-4">
            <input
              v-model="inputNote"
              type="text"
              placeholder="备注标签 (可选，如：私密启动项)..."
              class="w-full h-10 px-3.5 rounded-xl bg-black/40 border border-white/10 hover:border-white/20 focus:border-purple-500/70 focus:ring-2 focus:ring-purple-500/20 text-xs text-gray-100 placeholder-gray-500 transition-all outline-none"
              @keydown.enter="handleCloakNew"
            />
          </div>
        </div>

        <div class="mt-4 flex flex-col sm:flex-row items-center justify-between gap-3 pt-3 border-t border-white/5">
          <div class="flex items-center gap-2 text-[11px] text-gray-400">
            <HelpCircle class="w-3.5 h-3.5 text-purple-400 flex-shrink-0" />
            <span>添加后文件在资源管理器中将彻底消失；可在下方一键将其添加到“快速启动器”中进行私密调用。</span>
          </div>

          <button
            @click="handleCloakNew"
            :disabled="!inputPath.trim() || isSubmitting"
            type="button"
            class="px-5 py-2 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-lg shadow-purple-500/20 flex items-center gap-2 flex-shrink-0"
          >
            <Loader2 v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
            <EyeOff v-else class="w-3.5 h-3.5" />
            <span>执行深度隐藏</span>
          </button>
        </div>
      </section>

      <!-- 已深度隐藏的项目列表 -->
      <section class="space-y-3">
        <div class="flex items-center justify-between px-1">
          <h2 class="text-xs font-semibold text-gray-400 uppercase tracking-wider flex items-center gap-1.5">
            <FolderKey class="w-3.5 h-3.5 text-purple-400" />
            <span>受保护的私密项目清单</span>
          </h2>
          <span class="text-[11px] text-gray-500">共 {{ cloakedList.length }} 项</span>
        </div>

        <!-- 空列表引导 -->
        <div
          v-if="cloakedList.length === 0"
          class="rounded-2xl p-12 border border-white/5 bg-white/[0.01] flex flex-col items-center justify-center text-center space-y-3"
        >
          <div class="w-12 h-12 rounded-2xl bg-purple-500/10 text-purple-400 border border-purple-500/20 flex items-center justify-center">
            <ShieldAlert class="w-6 h-6" />
          </div>
          <div class="max-w-xs">
            <h3 class="text-sm font-semibold text-gray-200">暂无隐藏项目</h3>
            <p class="text-xs text-gray-500 mt-1 leading-relaxed">
              输入或选择需要隐身的文件/程序，隐藏后即便他人开启了“显示隐藏文件”也无法察觉。
            </p>
          </div>
        </div>

        <!-- 卡片列表 -->
        <div v-else class="grid grid-cols-1 gap-3">
          <div
            v-for="item in cloakedList"
            :key="item.id"
            class="group rounded-2xl p-4 bg-white/[0.025] hover:bg-white/[0.045] border border-white/10 hover:border-purple-500/30 transition-all flex flex-col sm:flex-row sm:items-center justify-between gap-4"
          >
            <!-- 左侧：图标与路径信息 -->
            <div class="flex items-start gap-3.5 min-w-0">
              <div
                class="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0 border"
                :class="item.is_cloaked ? 'bg-purple-500/10 text-purple-400 border-purple-500/30' : 'bg-gray-500/10 text-gray-400 border-gray-500/30'"
              >
                <Folder v-if="item.is_dir" class="w-5 h-5" />
                <FileCode v-else class="w-5 h-5" />
              </div>

              <div class="min-w-0 space-y-1">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-semibold text-gray-200 truncate">{{ item.name }}</span>
                  <span
                    v-if="item.is_cloaked"
                    class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-purple-500/15 text-purple-300 border border-purple-500/30 flex items-center gap-1"
                  >
                    <EyeOff class="w-2.5 h-2.5" />
                    <span>绝对隐身</span>
                  </span>
                  <span
                    v-else
                    class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-gray-500/15 text-gray-400 border border-gray-500/30 flex items-center gap-1"
                  >
                    <Eye class="w-2.5 h-2.5" />
                    <span>已恢复可见</span>
                  </span>
                </div>

                <p class="text-xs font-mono text-gray-400 truncate max-w-[420px]" :title="item.path">
                  {{ item.path }}
                </p>

                <p v-if="item.note" class="text-[11px] text-purple-300/80">
                  备注: {{ item.note }}
                </p>
              </div>
            </div>

            <!-- 右侧：快捷操作区 -->
            <div class="flex items-center gap-2 flex-wrap sm:flex-nowrap flex-shrink-0 justify-end">
              <!-- 核心协同按钮：发送至一键启动器 -->
              <button
                @click="handleSendToLauncher(item)"
                type="button"
                class="px-3 py-1.5 rounded-xl text-xs font-medium text-sky-300 bg-sky-500/10 hover:bg-sky-500/20 border border-sky-500/30 hover:border-sky-500/50 transition-all flex items-center gap-1.5"
                title="将此隐藏程序添加到一键启动器中，隐身状态下依然能随时唤起运行！"
              >
                <Rocket class="w-3 h-3 text-sky-400" />
                <span>加入启动器</span>
              </button>

              <!-- 切换隐藏/可见 -->
              <button
                v-if="item.is_cloaked"
                @click="handleToggleCloak(item)"
                type="button"
                class="px-3 py-1.5 rounded-xl text-xs font-medium text-gray-300 bg-white/[0.04] hover:bg-white/[0.08] border border-white/10 hover:border-white/20 transition-all flex items-center gap-1"
                title="临时取消超级隐藏，使其在资源管理器中重新可见"
              >
                <Eye class="w-3 h-3" />
                <span>解除隐形</span>
              </button>
              <button
                v-else
                @click="handleToggleCloak(item)"
                type="button"
                class="px-3 py-1.5 rounded-xl text-xs font-medium text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/30 transition-all flex items-center gap-1"
                title="重新对其施加超级隐藏"
              >
                <EyeOff class="w-3 h-3" />
                <span>重新隐形</span>
              </button>

              <!-- 移除记录 -->
              <button
                @click="handleRemoveRecord(item)"
                type="button"
                class="p-2 rounded-xl text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20 transition-all"
                title="从密室清单移除 (会自动恢复文件正常可见)"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file FileCloakerView.vue
 * @description 深度隐私密室主视图
 * 支持文件/文件夹超级隐藏、查看清单、解除可见以及一键联动至快速启动器
 */

import { ref, onMounted } from 'vue';
import {
  EyeOff,
  Eye,
  Lock,
  FolderSearch,
  HelpCircle,
  FolderKey,
  Shield,
  ShieldAlert,
  Folder,
  FileCode,
  Rocket,
  Trash2,
  Loader2,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { CloakedItem } from '../types/module';
import { useToast } from '../composables/useToast';
import { setActiveTool } from '../registry';

const { toast } = useToast();

const cloakedList = ref<CloakedItem[]>([]);
const inputPath = ref('');
const inputNote = ref('');
const isSubmitting = ref(false);

/**
 * 加载所有已记录的隐藏项目
 */
async function loadList() {
  if (isTauri()) {
    try {
      cloakedList.value = await invoke<CloakedItem[]>('load_cloaked_list');
    } catch (err) {
      console.error('加载隐藏项目失败:', err);
    }
  } else {
    // 浏览器 Mock 演示数据
    cloakedList.value = [
      {
        id: 'mock-1',
        name: 'PrivateGame.exe',
        path: 'D:\\Games\\PrivateGame.exe',
        is_dir: false,
        added_at: Date.now(),
        is_cloaked: true,
        note: '私密应用',
      },
    ];
  }
}

/**
 * 调用系统文件对话框选择任意文件
 */
async function handleBrowseFile() {
  if (isTauri()) {
    try {
      const selected = await invoke<string | null>('choose_any_file');
      if (selected) {
        inputPath.value = selected;
      }
    } catch (err) {
      toast.error('选择文件失败', String(err));
    }
  } else {
    inputPath.value = 'D:\\SecretDocuments\\ProjectData.exe';
  }
}

/**
 * 提交并施加深度隐身
 */
async function handleCloakNew() {
  const p = inputPath.value.trim();
  if (!p) return;

  isSubmitting.value = true;
  try {
    if (isTauri()) {
      const item = await invoke<CloakedItem>('cloak_file_or_dir', {
        path: p,
        note: inputNote.value.trim() || null,
      });
      toast.success('已施加超级隐藏！', `目标【${item.name}】在资源管理器中已彻底不可见。`);
      inputPath.value = '';
      inputNote.value = '';
      await loadList();
    } else {
      cloakedList.value.push({
        id: `mock-${Date.now()}`,
        name: p.split('\\').pop() || p,
        path: p,
        is_dir: !p.includes('.'),
        added_at: Date.now(),
        is_cloaked: true,
        note: inputNote.value,
      });
      inputPath.value = '';
      inputNote.value = '';
      toast.success('模拟隐藏成功');
    }
  } catch (err) {
    toast.error('隐藏失败', String(err));
  } finally {
    isSubmitting.value = false;
  }
}

/**
 * 切换隐身状态
 */
async function handleToggleCloak(item: CloakedItem) {
  try {
    if (isTauri()) {
      if (item.is_cloaked) {
        await invoke('uncloak_file_or_dir', { id: item.id });
        toast.info('已恢复可见', `【${item.name}】已恢复为普通可见属性。`);
      } else {
        await invoke('recloak_file_or_dir', { id: item.id });
        toast.success('已重新隐藏', `【${item.name}】已重新施加超级隐藏。`);
      }
      await loadList();
    } else {
      item.is_cloaked = !item.is_cloaked;
      toast.info('模拟状态切换');
    }
  } catch (err) {
    toast.error('操作失败', String(err));
  }
}

/**
 * 从清单中移除记录
 */
async function handleRemoveRecord(item: CloakedItem) {
  try {
    if (isTauri()) {
      await invoke('remove_cloaked_record', { id: item.id });
      toast.info('已移除保护记录', `已解除【${item.name}】的隐藏并移出清单。`);
      await loadList();
    } else {
      cloakedList.value = cloakedList.value.filter((i) => i.id !== item.id);
      toast.info('已移除模拟项');
    }
  } catch (err) {
    toast.error('移除失败', String(err));
  }
}

/**
 * 核心协同：发送至一键启动器
 */
async function handleSendToLauncher(item: CloakedItem) {
  try {
    if (isTauri()) {
      await invoke('send_cloaked_to_launcher', {
        id: item.id,
        silent: false,
      });
      toast.success('已发送至一键启动器！', `【${item.name}】已成功加入启动列表，即便处于隐身状态亦可随时一键唤起。`);
    } else {
      toast.success('模拟发送成功', '已联动至启动器。');
    }
    // 提示用户是否需要跳转
    setTimeout(() => {
      setActiveTool('launcher');
    }, 600);
  } catch (err) {
    toast.error('联动失败', String(err));
  }
}

onMounted(() => {
  loadList();
});
</script>

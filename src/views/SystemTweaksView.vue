<template>
  <!-- Windows 核心特性快速禁用与调优面板 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-rose-500/20 text-rose-400 border border-rose-500/30 flex items-center justify-center">
              <Sliders class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">系统核心特性快速禁用与优化</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            一键安全禁用高负载系统服务、拦截强制自动更新、关闭安全中心实时监控并释放数十 GB 磁盘空间。
          </p>
        </div>

        <!-- 权限状态与刷新 -->
        <div class="flex items-center gap-3">
          <div
            class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl border text-xs font-medium"
            :class="isAdmin ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
          >
            <ShieldCheck v-if="isAdmin" class="w-3.5 h-3.5 text-emerald-400" />
            <ShieldAlert v-else class="w-3.5 h-3.5 text-amber-400" />
            <span>{{ isAdmin ? '管理员权限已激活' : '标准权限 (建议提权)' }}</span>
          </div>

          <button
            v-if="!isAdmin"
            @click="handleRestartAsAdmin"
            type="button"
            class="px-3 py-1.5 rounded-xl text-xs font-medium text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 transition-all flex items-center gap-1"
          >
            <span>一键提权重启</span>
          </button>

          <button
            @click="loadTweaks"
            :disabled="isLoading"
            type="button"
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] border border-white/10 transition-all disabled:opacity-50"
            title="刷新系统特性状态"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            <span>刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主展示区 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar space-y-4">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div
          v-for="tweak in tweaks"
          :key="tweak.id"
          class="group rounded-2xl p-5 bg-white/[0.025] hover:bg-white/[0.045] border border-white/10 hover:border-white/20 transition-all flex flex-col justify-between space-y-4 shadow-xl backdrop-blur-md"
        >
          <div class="space-y-2.5">
            <!-- 标题与状态开关 -->
            <div class="flex items-start justify-between gap-3">
              <div>
                <div class="flex items-center gap-2">
                  <h2 class="text-sm font-bold text-gray-100">{{ tweak.title }}</h2>
                  <span
                    class="px-2 py-0.5 rounded-full text-[10px] font-medium border"
                    :class="tweak.is_disabled ? 'bg-rose-500/10 text-rose-300 border-rose-500/30' : 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30'"
                  >
                    {{ tweak.is_disabled ? '● 已完全禁用' : '● 系统默认运行' }}
                  </span>
                </div>
                <span class="inline-block mt-1 text-[11px] font-medium px-2 py-0.5 rounded bg-white/[0.04] text-sky-300 border border-white/5">
                  {{ tweak.impact }}
                </span>
              </div>

              <!-- 一键开关 Toggle -->
              <label class="relative inline-flex items-center cursor-pointer flex-shrink-0">
                <input
                  type="checkbox"
                  :checked="tweak.is_disabled"
                  :disabled="tweakLoadingId === tweak.id"
                  @change="handleToggleTweak(tweak, !tweak.is_disabled)"
                  class="sr-only peer"
                />
                <div class="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-rose-600"></div>
              </label>
            </div>

            <!-- 描述信息 -->
            <p class="text-xs text-gray-400 leading-relaxed">
              {{ tweak.description }}
            </p>
          </div>

          <div class="pt-3 border-t border-white/5 flex items-center justify-between text-[11px] text-gray-500">
            <span>需系统管理员权限</span>
            <span v-if="tweakLoadingId === tweak.id" class="text-sky-400 flex items-center gap-1">
              <Loader2 class="w-3 h-3 animate-spin" />
              <span>正在应用策略...</span>
            </span>
            <span v-else class="text-gray-400">点击右上方开关立即切换</span>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file SystemTweaksView.vue
 * @description Windows 核心特性快速禁用与系统优化控制面板
 */

import { ref, onMounted } from 'vue';
import {
  Sliders,
  ShieldCheck,
  ShieldAlert,
  RefreshCw,
  Loader2,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { SystemTweakItem } from '../types/module';
import { useToast } from '../composables/useToast';

const { toast } = useToast();

const tweaks = ref<SystemTweakItem[]>([]);
const isAdmin = ref(false);
const isLoading = ref(false);
const tweakLoadingId = ref<string | null>(null);

async function checkAdminStatus() {
  if (isTauri()) {
    try {
      isAdmin.value = await invoke<boolean>('get_admin_status');
    } catch {
      isAdmin.value = false;
    }
  }
}

async function loadTweaks() {
  isLoading.value = true;
  try {
    if (isTauri()) {
      tweaks.value = await invoke<SystemTweakItem[]>('get_system_tweaks');
    } else {
      // 浏览器 Mock 演示数据
      tweaks.value = [
        {
          id: 'windows_update',
          title: 'Windows 自动更新',
          description: '彻底拦截并禁用 Windows Update 强制后台下载、打补丁与自动重启更新行为。',
          impact: '杜绝打扰 · 锁定稳定版本',
          is_disabled: false,
          requires_admin: true,
        },
        {
          id: 'windows_defender',
          title: 'Windows Defender 安全中心实时防护',
          description: '关闭 Defender 后台实时扫描与 Antimalware Service 进程的高 CPU/磁盘占用。',
          impact: '显著降低 CPU 占用 · 防止误报拦截',
          is_disabled: false,
          requires_admin: true,
        },
        {
          id: 'hibernation',
          title: '系统休眠功能 (hiberfil.sys)',
          description: '关闭系统深度休眠，彻底删除 C 盘根目录下与物理内存同等大小的巨大休眠文件。',
          impact: '立即释放 8GB ~ 32GB 磁盘空间',
          is_disabled: true,
          requires_admin: true,
        },
      ];
    }
  } catch (err) {
    console.error('加载系统特性失败:', err);
  } finally {
    isLoading.value = false;
  }
}

async function handleToggleTweak(tweak: SystemTweakItem, disable: boolean) {
  tweakLoadingId.value = tweak.id;
  try {
    if (isTauri()) {
      await invoke('apply_system_tweak', { id: tweak.id, disable });
      toast.success(
        disable ? '已成功禁用特性' : '已成功启用特性',
        `【${tweak.title}】状态已切换并写入系统策略。`
      );
      await loadTweaks();
    } else {
      tweak.is_disabled = disable;
      toast.success('模拟状态切换', `【${tweak.title}】已切换为 ${disable ? '禁用' : '启用'}`);
    }
  } catch (err) {
    toast.error('操作失败', String(err));
    await loadTweaks();
  } finally {
    tweakLoadingId.value = null;
  }
}

async function handleRestartAsAdmin() {
  if (isTauri()) {
    try {
      toast.info('正在请求管理员提权', '请在稍后弹出的 UAC 窗口中确认授权');
      await invoke('request_restart_as_admin');
    } catch (err) {
      toast.error('提权启动失败', String(err));
    }
  } else {
    isAdmin.value = !isAdmin.value;
    toast.info('模拟提权状态');
  }
}

onMounted(async () => {
  await Promise.all([checkAdminStatus(), loadTweaks()]);
});
</script>

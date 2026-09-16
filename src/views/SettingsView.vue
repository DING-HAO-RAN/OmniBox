<template>
  <!-- 通用设置中心主视图 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 flex items-center justify-center">
              <Settings class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">通用配置中心</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            自定义系统运行权限、开机启动策略、界面语言以及自动化优化调度。
          </p>
        </div>

        <div class="flex items-center gap-2">
          <button
            @click="saveSettings(true)"
            :disabled="isSaving"
            type="button"
            class="px-4 py-2 rounded-xl text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 active:scale-95 disabled:opacity-50 transition-all flex items-center gap-1.5 shadow-lg shadow-blue-500/20"
          >
            <Check class="w-3.5 h-3.5" />
            <span>{{ isSaving ? '保存中...' : '保存设置' }}</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主设置面板，支持平滑滚动 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar space-y-6">
      <!-- 1. 运行权限管理 -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-4">
        <div class="flex items-center justify-between border-b border-white/5 pb-3">
          <div class="flex items-center gap-2">
            <ShieldAlert class="w-4 h-4 text-amber-400" />
            <h2 class="text-sm font-semibold text-gray-200">系统权限管理</h2>
          </div>
          <span
            class="px-2.5 py-0.5 rounded-full text-xs font-medium border"
            :class="isAdmin ? 'bg-amber-500/10 text-amber-300 border-amber-500/30' : 'bg-gray-500/10 text-gray-400 border-gray-500/30'"
          >
            {{ isAdmin ? '已获得管理员特权 (内核优化可用)' : '标准用户权限' }}
          </span>
        </div>

        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 py-1">
          <div>
            <h3 class="text-xs font-medium text-gray-200">以管理员身份重新启动</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              获得最高管理员权限可激活与 PCL2 相同的系统待机缓存 (Standby List) 深度释放，并提权管理受保护进程。
            </p>
          </div>
          <button
            @click="handleRestartAsAdmin"
            type="button"
            class="px-4 py-2 rounded-xl text-xs font-medium text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 transition-all flex items-center gap-1.5 flex-shrink-0"
          >
            <ShieldCheck class="w-3.5 h-3.5 text-amber-400" />
            <span>以管理员身份重启</span>
          </button>
        </div>
      </section>

      <!-- 2. 开机启动与行为 -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-5">
        <div class="flex items-center gap-2 border-b border-white/5 pb-3">
          <Power class="w-4 h-4 text-blue-400" />
          <h2 class="text-sm font-semibold text-gray-200">自启与常规行为</h2>
        </div>

        <!-- 开机自启 -->
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-xs font-medium text-gray-200">开机自动启动</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              在 Windows 登录时自动启动 OmniBox，即刻就绪快速启动与系统监控。
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              v-model="settings.auto_start"
              @change="saveSettings(false)"
              class="sr-only peer"
            />
            <div class="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
          </label>
        </div>

        <!-- 开机最小化 -->
        <div class="flex items-center justify-between pt-2 border-t border-white/5">
          <div>
            <h3 class="text-xs font-medium text-gray-200">开机时后台静默最小化</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              随系统启动时不弹出主窗口，默默在后台就绪。
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              v-model="settings.start_minimized"
              @change="saveSettings(false)"
              class="sr-only peer"
            />
            <div class="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
          </label>
        </div>

        <!-- 关闭窗口行为 -->
        <div class="flex items-center justify-between pt-2 border-t border-white/5">
          <div>
            <h3 class="text-xs font-medium text-gray-200">关闭窗口时最小化到系统后台</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              点击标题栏“关闭”按钮时不完全退出，保持后台快速待命。
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              v-model="settings.close_to_tray"
              @change="saveSettings(false)"
              class="sr-only peer"
            />
            <div class="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
          </label>
        </div>
      </section>

      <!-- 3. 语言与界面 -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-4">
        <div class="flex items-center gap-2 border-b border-white/5 pb-3">
          <Globe class="w-4 h-4 text-emerald-400" />
          <h2 class="text-sm font-semibold text-gray-200">语言与界面偏好</h2>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-xs font-medium text-gray-200">显示语言 (Language)</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              切换应用程序界面显示的语言。
            </p>
          </div>

          <select
            v-model="settings.language"
            @change="saveSettings(false)"
            class="h-9 px-3 rounded-xl bg-black/40 border border-white/10 hover:border-white/20 text-xs text-gray-200 focus:outline-none focus:border-emerald-500/50"
          >
            <option value="zh-CN">简体中文 (Simplified Chinese)</option>
            <option value="en-US">English (United States)</option>
          </select>
        </div>
      </section>

      <!-- 4. 内存智能自动清理策略 -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-5">
        <div class="flex items-center gap-2 border-b border-white/5 pb-3">
          <Cpu class="w-4 h-4 text-sky-400" />
          <h2 class="text-sm font-semibold text-gray-200">内存智能自动优化</h2>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-xs font-medium text-gray-200">启用后台智能自动清理</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              当系统物理内存占用超过设定阈值时，自动在后台静默执行工作集修剪。
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              v-model="settings.auto_clean_memory"
              @change="saveSettings(false)"
              class="sr-only peer"
            />
            <div class="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
          </label>
        </div>

        <div v-if="settings.auto_clean_memory" class="pt-2 border-t border-white/5 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-300">触发阈值</span>
            <span class="text-xs font-mono font-semibold text-sky-400">{{ settings.auto_clean_threshold }}%</span>
          </div>
          <input
            type="range"
            min="60"
            max="95"
            step="5"
            v-model.number="settings.auto_clean_threshold"
            @change="saveSettings(false)"
            class="w-full accent-blue-500 cursor-pointer"
          />
          <p class="text-[11px] text-gray-500">
            当系统内存占用率超过 {{ settings.auto_clean_threshold }}% 时自动释放多余内存。
          </p>
        </div>
      </section>

      <!-- 5. 数据管理与关于 -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-4">
        <div class="flex items-center gap-2 border-b border-white/5 pb-3">
          <Database class="w-4 h-4 text-purple-400" />
          <h2 class="text-sm font-semibold text-gray-200">数据与系统关于</h2>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs text-gray-400">
          <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
            <span class="text-gray-500 text-[11px]">底层架构</span>
            <p class="font-medium text-gray-200">Tauri v2 + Rust Win32 + Vue 3</p>
          </div>
          <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
            <span class="text-gray-500 text-[11px]">资源占用</span>
            <p class="font-medium text-emerald-400">常驻空载内存 ~23 MB</p>
          </div>
        </div>

        <div class="pt-2 text-[11px] text-gray-500 flex items-center justify-between">
          <span>OmniBox 万能工具箱 v0.1.0</span>
          <span>纯本地无遥测 · 适用于 Windows 10/11</span>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file SettingsView.vue
 * @description 通用设置中心
 * 提供管理员提权重启、开机自启动配置、语言偏好、智能内存自动清理与常规行为设定
 */

import { ref, onMounted } from 'vue';
import {
  Settings,
  ShieldAlert,
  ShieldCheck,
  Power,
  Globe,
  Cpu,
  Database,
  Check,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '../types/module';
import { useToast } from '../composables/useToast';

const { toast } = useToast();

const isAdmin = ref(false);
const isSaving = ref(false);

const settings = ref<AppSettings>({
  run_as_admin_default: false,
  language: 'zh-CN',
  auto_start: false,
  start_minimized: false,
  close_to_tray: false,
  auto_clean_memory: false,
  auto_clean_threshold: 80,
});

/**
 * 加载当前管理员权限状态
 */
async function checkAdminStatus() {
  if (isTauri()) {
    try {
      isAdmin.value = await invoke<boolean>('get_admin_status');
    } catch {
      isAdmin.value = false;
    }
  }
}

/**
 * 加载当前设置
 */
async function loadSettings() {
  if (isTauri()) {
    try {
      settings.value = await invoke<AppSettings>('get_app_settings');
    } catch (err) {
      console.error('加载应用程序设置失败:', err);
    }
  }
}

/**
 * 保存当前设置
 */
async function saveSettings(showToast = true) {
  isSaving.value = true;
  try {
    if (isTauri()) {
      await invoke('update_app_settings', { settings: settings.value });
    }
    if (showToast) {
      toast.success('设置已保存', '配置已即时生效并持久化。');
    }
  } catch (err) {
    toast.error('保存设置失败', String(err));
  } finally {
    isSaving.value = false;
  }
}

/**
 * 以管理员身份重启
 */
async function handleRestartAsAdmin() {
  if (isTauri()) {
    try {
      toast.info('正在请求管理员授权', '请在 Windows UAC 窗口中确认授权');
      await invoke('request_restart_as_admin');
    } catch (err) {
      toast.error('提权重启失败', String(err));
    }
  } else {
    isAdmin.value = !isAdmin.value;
    toast.info('模拟权限状态切换');
  }
}

onMounted(async () => {
  await Promise.all([checkAdminStatus(), loadSettings()]);
});
</script>

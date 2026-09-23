<template>
  <!-- 系统限制深度解除控制面板 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-8 h-8 rounded-xl bg-amber-500/20 text-amber-400 border border-amber-500/30 flex items-center justify-center shadow-lg shadow-amber-500/10">
              <Unlock class="w-4 h-4" />
            </div>
            <div>
              <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
                <span>系统限制深度解除</span>
                <span class="text-xs px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-300 border border-amber-500/20 font-normal">
                  全量解除引擎
                </span>
              </h1>
            </div>
          </div>
          <p class="text-xs text-gray-400 mt-1.5 max-w-3xl leading-relaxed">
            一键调用一切底层手段，深度解除 Windows USB 存储驱动封锁、可移动介质组策略只读、双击提示无法访问/拒绝访问，以及网络代理劫持、Hosts 封锁、Winsock/IP 堆栈与防火墙限制。
          </p>
        </div>

        <!-- 权限状态与操作主按钮群 -->
        <div class="flex flex-wrap items-center gap-2.5">
          <!-- 权限标签 -->
          <div
            class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl border text-xs font-medium"
            :class="isAdmin ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
          >
            <ShieldCheck v-if="isAdmin" class="w-3.5 h-3.5 text-emerald-400" />
            <ShieldAlert v-else class="w-3.5 h-3.5 text-amber-400" />
            <span>{{ isAdmin ? '管理员权限已激活' : '标准权限 (建议提权)' }}</span>
          </div>

          <!-- 一键提权重启 -->
          <button
            v-if="!isAdmin"
            @click="handleRestartAsAdmin"
            type="button"
            class="px-3 py-1.5 rounded-xl text-xs font-medium text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 transition-all flex items-center gap-1.5 shadow-sm"
          >
            <span>一键提权重启</span>
          </button>

          <!-- 刷新预检状态 -->
          <button
            @click="runScanOverview"
            :disabled="isExecuting"
            type="button"
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] border border-white/10 transition-all disabled:opacity-50"
            title="重新检测系统限制项状态"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isScanning }" />
            <span>状态预检</span>
          </button>

          <!-- 一键全量解除所有限制主按钮 -->
          <button
            @click="handleUnrestrictEverything"
            :disabled="isExecuting"
            type="button"
            class="flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-bold text-white bg-gradient-to-r from-amber-600 via-rose-600 to-indigo-600 hover:from-amber-500 hover:via-rose-500 hover:to-indigo-500 border border-amber-400/30 shadow-lg shadow-amber-500/20 transition-all active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Loader2 v-if="executingTarget === 'everything'" class="w-4 h-4 animate-spin" />
            <Sparkles v-else class="w-4 h-4 text-amber-200" />
            <span>一键全量解除所有限制</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主展示与交互区 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar space-y-6">
      <!-- 重启提示 Banner (当执行涉及 Winsock/TCP-IP 堆栈时) -->
      <div
        v-if="needsRebootNotice"
        class="rounded-2xl p-4 bg-amber-500/10 border border-amber-500/30 flex items-start justify-between gap-3 text-amber-200 text-xs shadow-lg backdrop-blur-md animate-in fade-in"
      >
        <div class="flex items-start gap-2.5">
          <AlertTriangle class="w-4 h-4 text-amber-400 mt-0.5 flex-shrink-0" />
          <div class="space-y-1">
            <div class="font-bold text-amber-100">底层网络重置已成功，建议重启系统使策略彻底生效</div>
            <p class="text-amber-300/80 leading-relaxed">
              Winsock 目录与 TCP/IP 传输协议栈的核心驱动重置需要系统重新初始化网络套接字。当前已生效，重启电脑后网络通信体验最佳。
            </p>
          </div>
        </div>
        <button
          @click="needsRebootNotice = false"
          type="button"
          class="text-amber-400/70 hover:text-amber-200 text-xs px-2 py-1 rounded-lg hover:bg-white/5 transition-colors"
        >
          知道了
        </button>
      </div>

      <!-- 核心双专区：USB 解禁与网络解禁 -->
      <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
        <!-- 专区 1: USB 使用限制全量解除 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 hover:border-white/15 transition-all flex flex-col justify-between shadow-xl backdrop-blur-md">
          <div class="space-y-4">
            <!-- 头部概况与单项一键按钮 -->
            <div class="flex items-center justify-between pb-3 border-b border-white/5">
              <div class="flex items-center gap-3">
                <div class="w-9 h-9 rounded-xl bg-sky-500/10 text-sky-400 border border-sky-500/20 flex items-center justify-center">
                  <Usb class="w-5 h-5" />
                </div>
                <div>
                  <h2 class="text-sm font-bold text-gray-100 flex items-center gap-2">
                    <span>USB 存储与外设使用限制</span>
                    <span
                      class="px-2 py-0.5 rounded-full text-[10px] font-medium border"
                      :class="usbIssuesCount > 0 ? 'bg-rose-500/10 text-rose-300 border-rose-500/30' : 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30'"
                    >
                      {{ usbIssuesCount > 0 ? `检测到 ${usbIssuesCount} 项限制` : '未发现策略封锁' }}
                    </span>
                  </h2>
                  <p class="text-[11px] text-gray-400 mt-0.5">
                    涵盖驱动服务、组策略封锁、写保护、盘符双击无法访问与 ACL 权限修复
                  </p>
                </div>
              </div>

              <!-- 单项解除按钮 -->
              <button
                @click="handleUnrestrictUsb"
                :disabled="isExecuting"
                type="button"
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold text-sky-300 bg-sky-500/10 hover:bg-sky-500/20 border border-sky-500/30 transition-all disabled:opacity-50"
              >
                <Loader2 v-if="executingTarget === 'usb'" class="w-3.5 h-3.5 animate-spin" />
                <Unlock v-else class="w-3.5 h-3.5" />
                <span>一键解除 USB 限制</span>
              </button>
            </div>

            <!-- 包含的手段清单列表 -->
            <div class="space-y-2">
              <div
                v-for="step in usbSteps"
                :key="step.id"
                class="rounded-xl p-3 bg-white/[0.02] border border-white/5 flex items-start justify-between gap-3 text-xs hover:bg-white/[0.035] transition-colors"
              >
                <div class="space-y-1">
                  <div class="flex items-center gap-2">
                    <span class="font-medium text-gray-200">{{ step.title }}</span>
                    <!-- 标签 -->
                    <span
                      v-if="step.id === 'explorer_drive_restrictions' || step.id === 'removable_volume_acls' || step.id === 'mountpoints2_cache'"
                      class="text-[9px] px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 border border-amber-500/30"
                    >
                      解决双击无法访问
                    </span>
                  </div>
                  <p class="text-[11px] text-gray-400 leading-relaxed">{{ step.message }}</p>
                  <div v-if="step.details" class="text-[10px] text-gray-500 font-mono mt-0.5">
                    {{ step.details }}
                  </div>
                </div>

                <!-- 状态指示图标 -->
                <div class="flex-shrink-0 mt-0.5">
                  <span
                    v-if="step.status === 'fixed'"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                  >
                    <CheckCircle2 class="w-3 h-3" />
                    <span>已解除</span>
                  </span>
                  <span
                    v-else-if="step.status === 'failed'"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-rose-500/20 text-rose-300 border border-rose-500/30"
                  >
                    <XCircle class="w-3 h-3" />
                    <span>受限/异常</span>
                  </span>
                  <span
                    v-else-if="step.status === 'warning'"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-amber-500/20 text-amber-300 border border-amber-500/30"
                  >
                    <AlertTriangle class="w-3 h-3" />
                    <span>需注意</span>
                  </span>
                  <span
                    v-else
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-white/[0.05] text-gray-300 border border-white/10"
                  >
                    <CheckCircle2 class="w-3 h-3 text-sky-400" />
                    <span>正常</span>
                  </span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 专区 2: 网络使用限制全量解除 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 hover:border-white/15 transition-all flex flex-col justify-between shadow-xl backdrop-blur-md">
          <div class="space-y-4">
            <!-- 头部概况与单项一键按钮 -->
            <div class="flex items-center justify-between pb-3 border-b border-white/5">
              <div class="flex items-center gap-3">
                <div class="w-9 h-9 rounded-xl bg-violet-500/10 text-violet-400 border border-violet-500/20 flex items-center justify-center">
                  <Globe class="w-5 h-5" />
                </div>
                <div>
                  <h2 class="text-sm font-bold text-gray-100 flex items-center gap-2">
                    <span>网络与互联网使用限制</span>
                    <span
                      class="px-2 py-0.5 rounded-full text-[10px] font-medium border"
                      :class="networkIssuesCount > 0 ? 'bg-rose-500/10 text-rose-300 border-rose-500/30' : 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30'"
                    >
                      {{ networkIssuesCount > 0 ? `检测到 ${networkIssuesCount} 项限制` : '未发现策略封锁' }}
                    </span>
                  </h2>
                  <p class="text-[11px] text-gray-400 mt-0.5">
                    涵盖系统与 WinHTTP 代理重置、Hosts 纯净恢复、Winsock/IP 堆栈及防火墙
                  </p>
                </div>
              </div>

              <!-- 单项解除按钮 -->
              <button
                @click="handleUnrestrictNetwork"
                :disabled="isExecuting"
                type="button"
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold text-violet-300 bg-violet-500/10 hover:bg-violet-500/20 border border-violet-500/30 transition-all disabled:opacity-50"
              >
                <Loader2 v-if="executingTarget === 'network'" class="w-3.5 h-3.5 animate-spin" />
                <Unlock v-else class="w-3.5 h-3.5" />
                <span>一键解除网络限制</span>
              </button>
            </div>

            <!-- 包含的手段清单列表 -->
            <div class="space-y-2">
              <div
                v-for="step in networkSteps"
                :key="step.id"
                class="rounded-xl p-3 bg-white/[0.02] border border-white/5 flex items-start justify-between gap-3 text-xs hover:bg-white/[0.035] transition-colors"
              >
                <div class="space-y-1">
                  <div class="flex items-center gap-2">
                    <span class="font-medium text-gray-200">{{ step.title }}</span>
                  </div>
                  <p class="text-[11px] text-gray-400 leading-relaxed">{{ step.message }}</p>
                  <div v-if="step.details" class="text-[10px] text-gray-500 font-mono mt-0.5">
                    {{ step.details }}
                  </div>
                </div>

                <!-- 状态指示图标 -->
                <div class="flex-shrink-0 mt-0.5">
                  <span
                    v-if="step.status === 'fixed'"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                  >
                    <CheckCircle2 class="w-3 h-3" />
                    <span>已解除</span>
                  </span>
                  <span
                    v-else-if="step.status === 'failed'"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-rose-500/20 text-rose-300 border border-rose-500/30"
                  >
                    <XCircle class="w-3 h-3" />
                    <span>受限/异常</span>
                  </span>
                  <span
                    v-else-if="step.status === 'warning'"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-amber-500/20 text-amber-300 border border-amber-500/30"
                  >
                    <AlertTriangle class="w-3 h-3" />
                    <span>需注意</span>
                  </span>
                  <span
                    v-else
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium bg-white/[0.05] text-gray-300 border border-white/10"
                  >
                    <CheckCircle2 class="w-3 h-3 text-sky-400" />
                    <span>正常</span>
                  </span>
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>

      <!-- 实时诊断与执行控制台日志 (Live Execution Console) -->
      <section class="rounded-2xl p-5 bg-[#0a0d13]/80 border border-white/10 shadow-2xl backdrop-blur-md space-y-3">
        <div class="flex items-center justify-between pb-2 border-b border-white/5">
          <div class="flex items-center gap-2 text-xs font-bold text-gray-300">
            <Terminal class="w-4 h-4 text-emerald-400" />
            <span>执行诊断与操作审计日志</span>
            <span class="text-[10px] font-normal text-gray-500 font-mono">
              ({{ logs.length }} 条记录)
            </span>
          </div>

          <div class="flex items-center gap-2">
            <button
              @click="clearLogs"
              type="button"
              class="text-[11px] text-gray-400 hover:text-gray-200 px-2.5 py-1 rounded-lg hover:bg-white/5 transition-colors flex items-center gap-1"
            >
              <Trash2 class="w-3 h-3" />
              <span>清空日志</span>
            </button>
          </div>
        </div>

        <!-- 终端日志内容 -->
        <div
          ref="terminalLogRef"
          class="h-48 overflow-y-auto font-mono text-[11px] space-y-1.5 pr-2 win11-scrollbar bg-black/30 rounded-xl p-3 border border-white/5"
        >
          <div v-if="logs.length === 0" class="text-gray-500 italic py-2">
            暂无操作日志，点击上方「状态预检」或「一键全量解除所有限制」将在此实时显示详细执行过程...
          </div>
          <div
            v-for="(log, idx) in logs"
            :key="idx"
            class="flex items-start gap-2 leading-relaxed"
            :class="getLogColorClass(log.level)"
          >
            <span class="text-gray-500 select-none flex-shrink-0">[{{ log.time }}]</span>
            <span class="font-bold flex-shrink-0">[{{ log.tag }}]</span>
            <span class="break-all">{{ log.text }}</span>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file SystemUnblockView.vue
 * @description Windows USB 使用限制与网络限制全量一键解除控制中心
 * 覆盖驱动服务、组策略封锁、双击无法访问/拒绝访问、代理重置、Hosts 恢复、Winsock 及防火墙
 */

import { ref, computed, onMounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  Unlock,
  ShieldCheck,
  ShieldAlert,
  RefreshCw,
  Loader2,
  Sparkles,
  AlertTriangle,
  Usb,
  Globe,
  CheckCircle2,
  XCircle,
  Terminal,
  Trash2,
} from 'lucide-vue-next';
import { showToast } from '../composables/useToast';
import type {
  RestrictionOverview,
  UnrestrictReport,
  UnrestrictStepResult,
} from '../types/module';

// 日志条目模型
interface ConsoleLogEntry {
  time: string;
  tag: string;
  text: string;
  level: 'info' | 'success' | 'warn' | 'error';
}

// 界面响应式状态
const isAdmin = ref(false);
const isScanning = ref(false);
const isExecuting = ref(false);
const executingTarget = ref<'' | 'everything' | 'usb' | 'network'>('');
const needsRebootNotice = ref(false);

// 步骤清单与预检统计
const allSteps = ref<UnrestrictStepResult[]>([]);
const terminalLogRef = ref<HTMLDivElement | null>(null);
const logs = ref<ConsoleLogEntry[]>([]);

// 过滤出 USB 与网络步骤
const usbSteps = computed(() => allSteps.value.filter((s) => s.category === 'usb'));
const networkSteps = computed(() => allSteps.value.filter((s) => s.category === 'network'));

const usbIssuesCount = computed(() => usbSteps.value.filter((s) => s.status === 'failed').length);
const networkIssuesCount = computed(() => networkSteps.value.filter((s) => s.status === 'failed').length);

/**
 * 记录控制台日志并滚动至底部
 */
function appendLog(tag: string, text: string, level: ConsoleLogEntry['level'] = 'info') {
  const now = new Date();
  const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`;
  logs.value.push({ time: timeStr, tag, text, level });

  nextTick(() => {
    if (terminalLogRef.value) {
      terminalLogRef.value.scrollTop = terminalLogRef.value.scrollHeight;
    }
  });
}

function clearLogs() {
  logs.value = [];
}

function getLogColorClass(level: ConsoleLogEntry['level']) {
  switch (level) {
    case 'success':
      return 'text-emerald-400';
    case 'warn':
      return 'text-amber-400';
    case 'error':
      return 'text-rose-400';
    default:
      return 'text-gray-300';
  }
}

/**
 * 检查当前程序管理员权限
 */
async function checkAdminStatus() {
  try {
    const admin = await invoke<boolean>('get_admin_status');
    isAdmin.value = admin;
  } catch (err) {
    console.error('获取管理员权限失败:', err);
  }
}

/**
 * 申请提权重启应用
 */
async function handleRestartAsAdmin() {
  try {
    appendLog('提权申请', '正在请求 Windows UAC 提权并以管理员身份重启 OmniBox...', 'warn');
    await invoke('request_restart_as_admin');
  } catch (err) {
    showToast({
      title: '提权重启失败',
      message: String(err),
      type: 'error',
    });
    appendLog('提权异常', `提权重启失败: ${err}`, 'error');
  }
}

/**
 * 运行系统全量限制状态预检
 */
async function runScanOverview() {
  if (isScanning.value || isExecuting.value) return;
  isScanning.value = true;
  appendLog('预检扫描', '正在全面扫描系统 USB 策略、双击访问权限及网络代理/Hosts/堆栈配置...', 'info');

  try {
    const overview = await invoke<RestrictionOverview>('get_restriction_status');
    allSteps.value = overview.details;

    const totalIssues = overview.usb_issues_count + overview.network_issues_count;
    if (totalIssues > 0) {
      appendLog(
        '扫描结果',
        `扫描完毕，检测到共 ${totalIssues} 项限制封锁 (USB: ${overview.usb_issues_count}, 网络: ${overview.network_issues_count})`,
        'warn'
      );
      showToast({
        title: '检测到系统限制策略',
        message: `发现 ${totalIssues} 项限制，建议点击一键全量解除`,
        type: 'warning',
      });
    } else {
      appendLog('扫描结果', '扫描完毕，当前系统暂未检测到限制策略，所有功能处于正常通行状态', 'success');
      showToast({
        title: '系统状态良好',
        message: '未发现策略封锁项',
        type: 'success',
      });
    }
  } catch (err) {
    appendLog('扫描失败', `预检调用失败: ${err}`, 'error');
    showToast({
      title: '扫描预检失败',
      message: String(err),
      type: 'error',
    });
  } finally {
    isScanning.value = false;
  }
}

/**
 * 格式化输出报告到控制台并更新界面
 */
function applyReportToState(report: UnrestrictReport, scopeTitle: string) {
  allSteps.value = report.steps;
  if (report.needs_reboot) {
    needsRebootNotice.value = true;
  }

  appendLog(
    scopeTitle,
    `执行完成！总计检查/处理 ${report.total_steps} 项，成功修复/解除 ${report.fixed_count} 项，无需修改 ${report.clean_count} 项，失败 ${report.failed_count} 项。`,
    report.success ? 'success' : 'warn'
  );

  // 逐项详细日志
  for (const step of report.steps) {
    const level = step.status === 'fixed' ? 'success' : step.status === 'failed' ? 'error' : step.status === 'warning' ? 'warn' : 'info';
    appendLog(
      step.status === 'fixed' ? '已修复' : step.status === 'clean' ? '正常' : step.status === 'warning' ? '提示' : '失败',
      `${step.title}: ${step.message}`,
      level
    );
  }

  showToast({
    title: `${scopeTitle}已完成`,
    message: report.message,
    type: report.success ? 'success' : 'warning',
    duration: 5000,
  });
}

/**
 * 一键解除 USB 使用限制
 */
async function handleUnrestrictUsb() {
  if (isExecuting.value) return;
  isExecuting.value = true;
  executingTarget.value = 'usb';
  appendLog('USB解除', '开始全量执行 USB 限制解除方案 (驱动服务/组策略/双击权限/ACL/MountPoints)...', 'info');

  try {
    const report = await invoke<UnrestrictReport>('unrestrict_usb_all');
    applyReportToState(report, 'USB 限制解除');
  } catch (err) {
    appendLog('USB解除失败', `执行报错: ${err}`, 'error');
    showToast({
      title: 'USB 解除流程失败',
      message: String(err),
      type: 'error',
    });
  } finally {
    isExecuting.value = false;
    executingTarget.value = '';
  }
}

/**
 * 一键解除网络使用限制
 */
async function handleUnrestrictNetwork() {
  if (isExecuting.value) return;
  isExecuting.value = true;
  executingTarget.value = 'network';
  appendLog('网络解除', '开始全量执行网络限制解除方案 (代理/PAC/Hosts/Winsock/IP堆栈/防火墙)...', 'info');

  try {
    const report = await invoke<UnrestrictReport>('unrestrict_network_all');
    applyReportToState(report, '网络限制解除');
  } catch (err) {
    appendLog('网络解除失败', `执行报错: ${err}`, 'error');
    showToast({
      title: '网络解除流程失败',
      message: String(err),
      type: 'error',
    });
  } finally {
    isExecuting.value = false;
    executingTarget.value = '';
  }
}

/**
 * 一键使用一切方法全量解除所有限制 (USB + 网络)
 */
async function handleUnrestrictEverything() {
  if (isExecuting.value) return;
  isExecuting.value = true;
  executingTarget.value = 'everything';
  appendLog('全量解除', '【一键全量解除】已启动：按序执行 USB 存储深度解禁与网络协议策略出厂重置...', 'info');

  try {
    const report = await invoke<UnrestrictReport>('unrestrict_everything');
    applyReportToState(report, '全量限制解除');
  } catch (err) {
    appendLog('全量解除失败', `执行报错: ${err}`, 'error');
    showToast({
      title: '全量解除流程失败',
      message: String(err),
      type: 'error',
    });
  } finally {
    isExecuting.value = false;
    executingTarget.value = '';
  }
}

onMounted(async () => {
  await checkAdminStatus();
  await runScanOverview();
});
</script>

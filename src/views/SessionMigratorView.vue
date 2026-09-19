<template>
  <!-- 跨设备浏览器会话迁移与免密登录器 (Browser Session Migrator) 主视图 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-5 pb-3 border-b border-white/5 bg-white/[0.01]">
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-8 h-8 rounded-xl bg-gradient-to-br from-pink-500/20 to-purple-500/20 text-pink-400 border border-pink-500/30 flex items-center justify-center shadow-lg shadow-pink-500/10">
              <Share2 class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">跨设备浏览器会话迁移</h1>
          </div>
          <p class="text-xs text-amber-300/90 bg-amber-500/10 border border-amber-500/20 px-3 py-1 rounded-lg mt-2 inline-block">
            仅用于迁移你本人合法拥有或明确获授权的账户会话。Cookie 等同于短期凭据，请谨慎保存和传输。
          </p>
        </div>

        <div class="flex items-center gap-2">
          <span
            v-if="runningBrowser?.is_running"
            class="px-3 py-1 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-300 border border-emerald-500/30 flex items-center gap-1.5"
          >
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>CDP 运行中 (PID {{ runningBrowser.pid }})</span>
          </span>
        </div>
      </div>
    </header>

    <!-- 主展示滚动区 -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-5 win11-scrollbar space-y-5">
      <!-- 1. 浏览器连接配置分组框 (QGroupBox "浏览器连接" 1:1 对齐) -->
      <section class="rounded-2xl p-6 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-4">
        <h2 class="text-sm font-bold text-gray-200 flex items-center gap-2 border-b border-white/5 pb-2.5">
          <Globe class="w-4 h-4 text-blue-400" />
          <span>浏览器连接设置</span>
        </h2>

        <div class="grid grid-cols-1 md:grid-cols-12 gap-3.5 text-xs">
          <!-- 浏览器类型 -->
          <div class="md:col-span-4 space-y-1.5">
            <label class="block text-gray-400 font-medium">浏览器：</label>
            <select
              v-model="browserKind"
              @change="handleBrowserChanged"
              class="w-full h-9 px-3 rounded-xl bg-black/40 border border-white/10 text-white focus:outline-none focus:border-blue-500"
            >
              <option value="chrome">Google Chrome</option>
              <option value="edge">Microsoft Edge</option>
            </select>
          </div>

          <!-- CDP 端口 -->
          <div class="md:col-span-3 space-y-1.5">
            <label class="block text-gray-400 font-medium">CDP 端口：</label>
            <input
              type="number"
              v-model.number="cdpPort"
              min="1024"
              max="65535"
              @change="handlePortChanged"
              class="w-full h-9 px-3 rounded-xl bg-black/40 border border-white/10 font-mono text-white focus:outline-none focus:border-blue-500"
            />
          </div>

          <!-- 独立配置目录 -->
          <div class="md:col-span-5 space-y-1.5">
            <label class="block text-gray-400 font-medium">独立配置目录：</label>
            <div class="flex gap-2">
              <input
                type="text"
                v-model="profileDir"
                placeholder="项目专用配置目录；不要选择日常浏览器的 User Data 目录"
                class="w-full h-9 px-3 rounded-xl bg-black/40 border border-white/10 text-gray-200 font-mono text-[11px] truncate focus:outline-none focus:border-blue-500"
                :title="profileDir"
              />
              <button
                @click="handleChooseProfileDir"
                type="button"
                class="px-3 py-1.5 rounded-xl bg-white/5 hover:bg-white/10 text-gray-300 font-medium whitespace-nowrap border border-white/10 transition-colors"
              >
                选择目录
              </button>
            </div>
          </div>
        </div>

        <!-- 操作按钮行 (一键启动本地 CDP / 停止 / 生成桌面启动脚本) -->
        <div class="flex flex-wrap items-center gap-3 pt-2">
          <button
            @click="handleStartCdp"
            :disabled="isBusy || runningBrowser?.is_running"
            type="button"
            class="px-4 py-2 rounded-xl text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 active:scale-95 disabled:opacity-50 transition-all flex items-center gap-1.5 shadow-lg shadow-blue-500/20"
          >
            <Play class="w-3.5 h-3.5" />
            <span>一键启动本地 CDP</span>
          </button>

          <button
            @click="handleStopCdp"
            :disabled="isBusy || !runningBrowser?.is_running"
            type="button"
            class="px-4 py-2 rounded-xl text-xs font-semibold text-gray-300 bg-white/5 hover:bg-rose-600 hover:text-white active:scale-95 disabled:opacity-40 transition-all flex items-center gap-1.5 border border-white/10"
          >
            <Square class="w-3.5 h-3.5" />
            <span>停止 CDP 浏览器</span>
          </button>

          <button
            @click="handleCreateLauncher"
            :disabled="isBusy"
            type="button"
            class="px-4 py-2 rounded-xl text-xs font-semibold text-gray-300 bg-white/5 hover:bg-white/10 active:scale-95 transition-all flex items-center gap-1.5 border border-white/10"
          >
            <FileCode2 class="w-3.5 h-3.5" />
            <span>生成桌面启动脚本</span>
          </button>
        </div>

        <!-- 详细参数 (本地 CDP 地址、目标域名、站点 URL、包含子域名) -->
        <div class="grid grid-cols-1 md:grid-cols-12 gap-3.5 text-xs pt-3 border-t border-white/5">
          <div class="md:col-span-4 space-y-1.5">
            <label class="block text-gray-400 font-medium">本地 CDP 地址：</label>
            <input
              type="text"
              v-model="cdpEndpoint"
              placeholder="http://127.0.0.1:9222"
              class="w-full h-9 px-3 rounded-xl bg-black/40 border border-white/10 text-gray-200 font-mono focus:outline-none focus:border-blue-500"
            />
          </div>

          <div class="md:col-span-4 space-y-1.5">
            <label class="block text-gray-400 font-medium">目标域名：</label>
            <input
              type="text"
              v-model="targetDomain"
              placeholder="bilibili.com"
              class="w-full h-9 px-3 rounded-xl bg-black/40 border border-white/10 text-white font-mono focus:outline-none focus:border-blue-500"
            />
          </div>

          <div class="md:col-span-4 space-y-1.5">
            <label class="block text-gray-400 font-medium">站点 URL：</label>
            <input
              type="text"
              v-model="siteUrl"
              placeholder="https://www.bilibili.com/"
              class="w-full h-9 px-3 rounded-xl bg-black/40 border border-white/10 text-white font-mono focus:outline-none focus:border-blue-500"
            />
          </div>

          <div class="md:col-span-12 flex items-center gap-2 pt-1">
            <label class="flex items-center gap-2 text-xs text-gray-300 cursor-pointer">
              <input type="checkbox" v-model="includeSubdomains" class="rounded bg-black/40 border-white/20 accent-blue-500" />
              <span>包含目标域名的子域名（仅在确有需要时勾选）</span>
            </label>
          </div>
        </div>
      </section>

      <!-- 2. 功能选项卡 (导出 / 导入) -->
      <section class="rounded-2xl bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md overflow-hidden">
        <!-- 标签页头部导航 -->
        <div class="flex items-center border-b border-white/10 bg-white/[0.02]">
          <button
            @click="activeTab = 'export'"
            type="button"
            class="px-6 py-3 text-xs font-bold transition-all relative flex items-center gap-2"
            :class="activeTab === 'export' ? 'text-blue-400 bg-white/[0.04]' : 'text-gray-400 hover:text-gray-200'"
          >
            <Upload class="w-3.5 h-3.5" />
            <span>导出到加密文件</span>
            <span v-if="activeTab === 'export'" class="absolute bottom-0 left-0 right-0 h-0.5 bg-blue-500"></span>
          </button>

          <button
            @click="activeTab = 'import'"
            type="button"
            class="px-6 py-3 text-xs font-bold transition-all relative flex items-center gap-2"
            :class="activeTab === 'import' ? 'text-purple-400 bg-white/[0.04]' : 'text-gray-400 hover:text-gray-200'"
          >
            <Download class="w-3.5 h-3.5" />
            <span>从加密文件导入</span>
            <span v-if="activeTab === 'import'" class="absolute bottom-0 left-0 right-0 h-0.5 bg-purple-500"></span>
          </button>
        </div>

        <!-- 标签内容区 -->
        <div class="p-6">
          <!-- 导出 Tab -->
          <div v-if="activeTab === 'export'" class="space-y-4">
            <p class="text-xs text-gray-300">
              在浏览器 A 中确认已经登录，然后选择一个仅由你控制的输出位置。导出文件不会包含明文 Cookie。
            </p>

            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 text-[11px] text-gray-400 font-mono">
              默认 JSON 存放目录：{{ sessionsDir }}
            </div>

            <div class="flex flex-wrap items-center gap-3 pt-2">
              <button
                @click="openExportPasswordModal('standard')"
                :disabled="isBusy"
                type="button"
                class="px-5 py-2.5 rounded-xl text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 active:scale-95 disabled:opacity-50 transition-all flex items-center gap-1.5 shadow-lg shadow-blue-500/20"
              >
                <FileLock class="w-3.5 h-3.5" />
                <span>选择位置并导出</span>
              </button>

              <button
                @click="openExportPasswordModal('extension')"
                :disabled="isBusy"
                type="button"
                class="px-5 py-2.5 rounded-xl text-xs font-semibold text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/30 active:scale-95 disabled:opacity-50 transition-all flex items-center gap-1.5"
              >
                <FileSliders class="w-3.5 h-3.5" />
                <span>导出扩展兼容文件（导入日常浏览器 profile）</span>
              </button>
            </div>
          </div>

          <!-- 导入 Tab -->
          <div v-else class="space-y-4">
            <p class="text-xs text-gray-300">
              先把加密文件安全传到设备 B，再连接目标浏览器。导入不会删除目标浏览器已有 Cookie。
            </p>

            <!-- 文件选择输入行 -->
            <div class="flex items-center gap-2">
              <input
                type="text"
                v-model="importFilePath"
                placeholder="可选择 sessions 目录或其他位置的加密 JSON 文件"
                class="flex-1 h-10 px-3.5 rounded-xl bg-black/40 border border-white/10 text-xs text-white font-mono focus:outline-none focus:border-purple-500"
              />
              <button
                @click="handleChooseImportFile"
                type="button"
                class="px-3.5 py-2.5 rounded-xl bg-white/5 hover:bg-white/10 text-xs text-gray-200 font-medium border border-white/10 transition-colors whitespace-nowrap"
              >
                选择 JSON 文件
              </button>
              <button
                @click="importFilePath = ''"
                type="button"
                class="px-3.5 py-2.5 rounded-xl bg-white/5 hover:bg-white/10 text-xs text-gray-400 hover:text-white transition-colors"
              >
                清空
              </button>
            </div>

            <div class="flex flex-wrap items-center gap-3 pt-2">
              <button
                @click="handleOpenSessionsDir"
                type="button"
                class="px-4 py-2 rounded-xl text-xs font-medium text-gray-300 bg-white/5 hover:bg-white/10 border border-white/10 transition-all flex items-center gap-1.5"
              >
                <FolderOpen class="w-3.5 h-3.5" />
                <span>打开项目 sessions 存放目录</span>
              </button>

              <button
                @click="openImportPasswordModal"
                :disabled="isBusy || !importFilePath.trim()"
                type="button"
                class="px-5 py-2 rounded-xl text-xs font-semibold text-white bg-purple-600 hover:bg-purple-500 active:scale-95 disabled:opacity-50 transition-all flex items-center gap-1.5 shadow-lg shadow-purple-500/20"
              >
                <Download class="w-3.5 h-3.5" />
                <span>导入已选择的 JSON 文件</span>
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- 3. 运行状态面板 -->
      <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-2.5 shadow-md">
        <h2 class="text-xs font-bold text-gray-300">运行状态</h2>
        <div class="flex items-center gap-2">
          <Loader2 v-if="isBusy" class="w-4 h-4 animate-spin text-blue-400 flex-shrink-0" />
          <CheckCircle2 v-else class="w-4 h-4 text-emerald-400 flex-shrink-0" />
          <p class="text-xs text-gray-300 break-all">{{ statusMessage }}</p>
        </div>
        <!-- 进度动效条 -->
        <div v-if="isBusy" class="w-full h-1 rounded-full bg-white/5 overflow-hidden">
          <div class="w-1/3 h-full bg-blue-500 rounded-full animate-indeterminate"></div>
        </div>
      </section>

      <!-- 4. 底部安全提示 -->
      <div class="text-[11px] text-gray-500 text-center pb-2">
        安全提示：不要把 CDP 端口暴露到局域网或公网；网站可能因设备变化而要求重新验证。
      </div>
    </main>

    <!-- 口令输入弹窗 -->
    <PasswordModal
      :visible="passwordModalVisible"
      :title="passwordModalTitle"
      :confirm="passwordModalConfirm"
      @submit="handlePasswordSubmit"
      @cancel="passwordModalVisible = false"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * @file SessionMigratorView.vue
 * @description 跨设备浏览器会话迁移器 (Browser Session Migrator) 主视图
 * 1:1 复刻原项目的连接配置、CDP 控制、加密导出与安全导入全部交互与功能。
 */

import { ref, onMounted, onUnmounted } from 'vue';
import {
  Share2,
  Globe,
  Play,
  Square,
  FileCode2,
  Upload,
  Download,
  FileLock,
  FileSliders,
  FolderOpen,
  CheckCircle2,
  Loader2,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type {
  RunningBrowserInfo,
  SessionMigratorDefaults,
  SessionBundle,
  CookieRecord,
} from '../types/module';
import { useToast } from '../composables/useToast';
import PasswordModal from '../components/session-migrator/PasswordModal.vue';

const { toast } = useToast();

const browserKind = ref<'chrome' | 'edge'>('chrome');
const cdpPort = ref(9222);
const profileDir = ref('');
const cdpEndpoint = ref('http://127.0.0.1:9222');
const targetDomain = ref('bilibili.com');
const siteUrl = ref('https://www.bilibili.com/');
const includeSubdomains = ref(true);

const activeTab = ref<'export' | 'import'>('export');
const sessionsDir = ref('');
const importFilePath = ref('');

const runningBrowser = ref<RunningBrowserInfo | null>(null);
const isBusy = ref(false);
const statusMessage = ref('就绪。请先启动浏览器的本地 CDP 调试端口。');

// 口令弹窗控制
const passwordModalVisible = ref(false);
const passwordModalTitle = ref('会话文件口令');
const passwordModalConfirm = ref(false);
let pendingPasswordAction: ((pwd: string) => Promise<void>) | null = null;

// 定时同步 CDP 状态
let statusTimer: ReturnType<typeof setInterval> | null = null;

// Base64 编码辅助函数
function uint8ToBase64(bytes: Uint8Array): string {
  let binary = '';
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

// Base64 解码辅助函数
function base64ToUint8(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

// 域名规范化检查
function normalizeDomain(value: string): string {
  let domain = value.trim().toLowerCase().replace(/^\.+/, '');
  if (!domain || domain.includes('/') || domain.includes(':') || domain.includes(' ')) {
    throw new Error('目标域名必须是不带协议和路径的主机名');
  }
  return domain;
}

function domainMatches(cookieDomain: string, requestedDomain: string, subdomains: boolean): boolean {
  const c = normalizeDomain(cookieDomain);
  const r = normalizeDomain(requestedDomain);
  return c === r || (subdomains && c.endsWith('.' + r));
}

// 切换浏览器类型时更新默认配置目录
async function handleBrowserChanged() {
  if (isTauri()) {
    try {
      const defaults = await invoke<SessionMigratorDefaults>('session_migrator_get_defaults');
      profileDir.value =
        browserKind.value === 'chrome' ? defaults.default_chrome_profile : defaults.default_edge_profile;
    } catch {
      // ignore
    }
  }
}

function handlePortChanged() {
  if (cdpEndpoint.value.startsWith('http://127.0.0.1:') || cdpEndpoint.value.startsWith('http://localhost:')) {
    cdpEndpoint.value = `http://127.0.0.1:${cdpPort.value}`;
  }
}

async function handleChooseProfileDir() {
  if (isTauri()) {
    try {
      const selected = await invoke<string | null>('session_migrator_choose_profile_dir');
      if (selected) {
        profileDir.value = selected;
      }
    } catch (err) {
      toast.error('选择目录失败', String(err));
    }
  }
}

// 一键启动本地 CDP
async function handleStartCdp() {
  if (isBusy.value) return;
  isBusy.value = true;
  statusMessage.value = '正在启动本地 CDP 浏览器并等待调试就绪...';

  try {
    if (isTauri()) {
      const info = await invoke<RunningBrowserInfo>('session_migrator_launch_cdp', {
        browser: browserKind.value,
        port: cdpPort.value,
        profileDir: profileDir.value,
      });
      runningBrowser.value = info;
      cdpEndpoint.value = info.endpoint;
      statusMessage.value =
        '本地 CDP 已启动。请在新浏览器窗口中手动登录并完成 MFA，确认登录成功后再导出或导入。';
      toast.success('本地 CDP 已启动', `端口: ${info.port}，浏览器版本: ${info.browser_version}`);
    } else {
      runningBrowser.value = {
        browser: browserKind.value,
        endpoint: `http://127.0.0.1:${cdpPort.value}`,
        port: cdpPort.value,
        profile_dir: profileDir.value,
        pid: 1234,
        browser_version: 'Chrome 128.0 (Mock)',
        is_running: true,
      };
      statusMessage.value = '本地 CDP 已启动 (Mock 模式)。';
    }
  } catch (err) {
    statusMessage.value = `启动 CDP 浏览器失败：${err}`;
    toast.error('启动失败', String(err));
  } finally {
    isBusy.value = false;
  }
}

// 停止 CDP 浏览器
async function handleStopCdp() {
  if (isBusy.value) return;
  isBusy.value = true;
  try {
    if (isTauri()) {
      await invoke('session_migrator_stop_cdp');
      runningBrowser.value = null;
      statusMessage.value = 'CDP 浏览器已安全停止。';
      toast.info('已停止', 'CDP 浏览器进程已关闭。');
    } else {
      runningBrowser.value = null;
      statusMessage.value = 'CDP 浏览器已停止。';
    }
  } catch (err) {
    toast.error('停止失败', String(err));
  } finally {
    isBusy.value = false;
  }
}

// 生成桌面启动脚本
async function handleCreateLauncher() {
  try {
    if (isTauri()) {
      const scriptPath = await invoke<string>('session_migrator_create_launcher', {
        browser: browserKind.value,
        port: cdpPort.value,
        profileDir: profileDir.value,
        destPath: null,
      });
      toast.success('已生成启动脚本', `已成功保存到桌面：${scriptPath}`);
    } else {
      toast.success('模拟生成成功', '已生成桌面启动脚本。');
    }
  } catch (err) {
    toast.error('生成启动脚本失败', String(err));
  }
}

// 选择待导入的 JSON 文件
async function handleChooseImportFile() {
  if (isTauri()) {
    try {
      const file = await invoke<string | null>('session_migrator_choose_file', { mode: 'open' });
      if (file) {
        importFilePath.value = file;
      }
    } catch (err) {
      toast.error('选择文件失败', String(err));
    }
  }
}

// 打开 sessions 文件夹
async function handleOpenSessionsDir() {
  if (isTauri()) {
    try {
      await invoke('session_migrator_open_sessions_dir');
    } catch (err) {
      toast.error('打开目录失败', String(err));
    }
  }
}

// 打开导出密码弹窗
function openExportPasswordModal(formatType: 'standard' | 'extension') {
  if (!cdpEndpoint.value || !targetDomain.value.trim() || !siteUrl.value.trim()) {
    toast.error('参数不完整', 'CDP 地址、目标域名和站点 URL 都不能为空。');
    return;
  }
  passwordModalTitle.value = formatType === 'standard' ? '会话文件加密口令' : '扩展会话文件加密口令';
  passwordModalConfirm.value = true;
  pendingPasswordAction = async (pwd) => {
    await doExport(pwd, formatType);
  };
  passwordModalVisible.value = true;
}

// 打开导入密码弹窗
function openImportPasswordModal() {
  if (!importFilePath.value.trim()) {
    toast.error('请选择文件', '请先选择待导入的加密 JSON 文件。');
    return;
  }
  if (!cdpEndpoint.value || !targetDomain.value.trim() || !siteUrl.value.trim()) {
    toast.error('参数不完整', 'CDP 地址、目标域名和站点 URL 都不能为空。');
    return;
  }
  passwordModalTitle.value = '请输入解密口令';
  passwordModalConfirm.value = false;
  pendingPasswordAction = async (pwd) => {
    await doImport(pwd);
  };
  passwordModalVisible.value = true;
}

async function handlePasswordSubmit(pwd: string) {
  passwordModalVisible.value = false;
  if (pendingPasswordAction) {
    const action = pendingPasswordAction;
    pendingPasswordAction = null;
    await action(pwd);
  }
}

// 连接目标 CDP 浏览器并通过 WebSocket 执行提取/注入
async function executeCdpCommand(endpoint: string, site: string, method: string, params: any = {}): Promise<any> {
  const versionRes = await fetch(`${endpoint}/json/version`);
  if (!versionRes.ok) throw new Error('无法连接到本地 CDP 端口，请确认浏览器已启动');

  const listRes = await fetch(`${endpoint}/json/list`);
  const pages: any[] = await listRes.json();
  let target = pages.find((p) => p.type === 'page');
  if (!target) {
    const newRes = await fetch(`${endpoint}/json/new?${encodeURIComponent(site)}`, { method: 'PUT' });
    target = await newRes.json();
  }
  if (!target?.webSocketDebuggerUrl) {
    throw new Error('CDP 浏览器没有可用的页面 WebSocket 调试连接');
  }

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise<void>((resolve, reject) => {
    ws.onopen = () => resolve();
    ws.onerror = () => reject(new Error('无法建立 CDP WebSocket 握手，请检查浏览器连接'));
    setTimeout(() => reject(new Error('连接 CDP 页面超时')), 5000);
  });

  return new Promise((resolve, reject) => {
    const id = Math.floor(Math.random() * 100000) + 1;
    const handler = (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.id === id) {
          ws.removeEventListener('message', handler);
          ws.close();
          if (msg.error) {
            reject(new Error(msg.error.message || JSON.stringify(msg.error)));
          } else {
            resolve(msg.result);
          }
        }
      } catch (err) {
        // ignore other messages
      }
    };
    ws.addEventListener('message', handler);
    ws.send(JSON.stringify({ id, method, params }));
    setTimeout(() => {
      ws.removeEventListener('message', handler);
      ws.close();
      reject(new Error(`CDP 命令 [${method}] 响应超时`));
    }, 15000);
  });
}

// 执行导出操作
async function doExport(passphrase: string, formatType: 'standard' | 'extension') {
  isBusy.value = true;
  statusMessage.value = '正在连接 CDP 并提取目标站点 Cookie...';

  try {
    const domain = normalizeDomain(targetDomain.value);
    const site = siteUrl.value.trim();
    const subdomains = includeSubdomains.value;

    let cookies: CookieRecord[] = [];
    let metadataVals: any = {};

    if (isTauri()) {
      const getRes = await executeCdpCommand(cdpEndpoint.value, site, 'Network.getCookies', { urls: [site] });
      const raw = getRes.cookies || [];
      const current = Date.now() / 1000;

      for (const item of raw) {
        if (!domainMatches(item.domain, domain, subdomains)) continue;
        const exp = item.expires && item.expires > 0 ? Number(item.expires) : null;
        if (exp !== null && exp <= current) continue;

        cookies.push({
          domain: item.domain,
          path: item.path || '/',
          name: item.name || '',
          value: item.value || '',
          expirationDate: exp,
          secure: Boolean(item.secure),
          httpOnly: Boolean(item.httpOnly),
          sameSite: item.sameSite || 'Lax',
        });
      }

      if (cookies.length === 0) {
        throw new Error(`在目标站点 [${site}] 未检测到属于 [${domain}] 的登录 Cookie，请先在浏览器窗口中完成登录。`);
      }

      try {
        const evalRes = await executeCdpCommand(cdpEndpoint.value, site, 'Runtime.evaluate', {
          expression: `({
            userAgent: navigator.userAgent,
            acceptLanguage: navigator.language || '',
            timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || '',
            screenResolution: \`\${window.screen?.width || 0}x\${window.screen?.height || 0}\`
          })`,
          returnByValue: true,
        });
        metadataVals = evalRes.result?.value || {};
      } catch {
        // ignore metadata eval failure
      }
    } else {
      cookies = [
        {
          domain: '.bilibili.com',
          path: '/',
          name: 'SESSDATA',
          value: 'mock_sessdata_value',
          expirationDate: 1789000000,
          secure: true,
          httpOnly: true,
          sameSite: 'Lax',
        },
      ];
    }

    const bundle: SessionBundle = {
      schema: 'bsm/session-bundle/v1',
      targetDomain: domain,
      siteUrl: site,
      metadata: {
        exportedAt: new Date().toISOString(),
        operatingSystem: 'Windows',
        browser: browserKind.value,
        browserVersion: runningBrowser.value?.browser_version || '128.0',
        userAgent: metadataVals.userAgent || navigator.userAgent,
        acceptLanguage: metadataVals.acceptLanguage || navigator.language,
        timezone: metadataVals.timezone || 'Asia/Shanghai',
        screenResolution: metadataVals.screenResolution || `${window.screen.width}x${window.screen.height}`,
      },
      cookies,
    };

    // 使用 Web Crypto API 生成 PBKDF2 + AES-256-GCM 格式 bsm/2 文件
    const salt = window.crypto.getRandomValues(new Uint8Array(16));
    const nonce = window.crypto.getRandomValues(new Uint8Array(12));
    const AAD = new TextEncoder().encode('bsm:v2');

    const passwordKey = await crypto.subtle.importKey(
      'raw',
      new TextEncoder().encode(passphrase),
      'PBKDF2',
      false,
      ['deriveKey']
    );

    const aesKey = await crypto.subtle.deriveKey(
      { name: 'PBKDF2', salt, iterations: 600000, hash: 'SHA-256' },
      passwordKey,
      { name: 'AES-GCM', length: 256 },
      false,
      ['encrypt']
    );

    const plaintext = new TextEncoder().encode(JSON.stringify(bundle));
    const encryptedBuf = await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv: nonce, additionalData: AAD },
      aesKey,
      plaintext
    );

    const envelope = {
      format: 'bsm/2',
      kdf: {
        name: 'PBKDF2-HMAC-SHA256',
        salt: uint8ToBase64(salt),
        iterations: 600000,
      },
      cipher: {
        name: 'AES-256-GCM',
        nonce: uint8ToBase64(nonce),
        aad: 'bsm:v2',
      },
      ciphertext: uint8ToBase64(new Uint8Array(encryptedBuf)),
    };

    const envelopeJson = JSON.stringify(envelope, null, 2);

    // 弹出文件保存位置
    let savePath: string | null = null;
    if (isTauri()) {
      savePath = await invoke<string | null>('session_migrator_choose_file', {
        mode: formatType === 'standard' ? 'save' : 'save_extension',
      });
    } else {
      savePath = 'D:\\sessions\\bilibili.session.json';
    }

    if (!savePath) {
      statusMessage.value = '导出已取消。未写入文件。';
      return;
    }

    let fileHash = '';
    if (isTauri()) {
      fileHash = await invoke<string>('session_migrator_write_file', {
        path: savePath,
        content: envelopeJson,
      });
    }

    statusMessage.value = `已生成 AES-256-GCM 加密会话文件 (包含 ${cookies.length} 个 Cookie)。`;
    importFilePath.value = savePath;
    activeTab.value = 'import';

    toast.success(
      '导出加密文件成功！',
      `文件路径: ${savePath}\nSHA-256: ${fileHash}\n已自动填充至导入面板。`
    );
  } catch (err) {
    statusMessage.value = `导出失败：${err}`;
    toast.error('导出失败', String(err));
  } finally {
    isBusy.value = false;
  }
}

// 执行导入操作
async function doImport(passphrase: string) {
  isBusy.value = true;
  statusMessage.value = '正在读取并解密会话文件...';

  try {
    const filePath = importFilePath.value.trim();
    let fileContent = '';
    if (isTauri()) {
      fileContent = await invoke<string>('session_migrator_read_file', { path: filePath });
    } else {
      fileContent = '{}';
    }

    const envelope = JSON.parse(fileContent);
    if (envelope.format !== 'bsm/2') {
      throw new Error(
        `暂不支持该会话文件格式 [${envelope.format || 'unknown'}]，本程序原生支持 bsm/2 (PBKDF2 + AES-256-GCM) 互通格式。`
      );
    }

    const salt = base64ToUint8(envelope.kdf.salt);
    const nonce = base64ToUint8(envelope.cipher.nonce);
    const ciphertext = base64ToUint8(envelope.ciphertext);
    const AAD = new TextEncoder().encode(envelope.cipher.aad || 'bsm:v2');

    const passwordKey = await crypto.subtle.importKey(
      'raw',
      new TextEncoder().encode(passphrase),
      'PBKDF2',
      false,
      ['deriveKey']
    );

    const aesKey = await crypto.subtle.deriveKey(
      { name: 'PBKDF2', salt, iterations: Number(envelope.kdf.iterations) || 600000, hash: 'SHA-256' },
      passwordKey,
      { name: 'AES-GCM', length: 256 },
      false,
      ['decrypt']
    );

    let bundle: SessionBundle;
    try {
      const decryptedBuf = await crypto.subtle.decrypt(
        { name: 'AES-GCM', iv: nonce, additionalData: AAD },
        aesKey,
        ciphertext
      );
      bundle = JSON.parse(new TextDecoder().decode(decryptedBuf));
    } catch {
      throw new Error('口令错误，或加密会话文件已损坏/被篡改。');
    }

    if (bundle.schema !== 'bsm/session-bundle/v1' || !Array.isArray(bundle.cookies)) {
      throw new Error('解密后的会话数据结构非法。');
    }

    const requestedDomain = normalizeDomain(targetDomain.value);
    if (normalizeDomain(bundle.targetDomain) !== requestedDomain) {
      throw new Error(
        `导入文件的目标域名 [${bundle.targetDomain}] 与当前设定的目标域名 [${requestedDomain}] 不一致。`
      );
    }

    statusMessage.value = '正在向目标浏览器注入 Cookie 并导航...';

    const current = Date.now() / 1000;
    const cdpCookiesToSet: any[] = [];

    for (const c of bundle.cookies) {
      if (!domainMatches(c.domain, requestedDomain, includeSubdomains.value)) continue;
      if (c.expirationDate && c.expirationDate <= current) continue;

      cdpCookiesToSet.push({
        name: c.name,
        value: c.value,
        domain: c.domain,
        path: c.path,
        secure: c.secure,
        httpOnly: c.httpOnly,
        sameSite: c.sameSite && ['Strict', 'Lax', 'None'].includes(c.sameSite) ? c.sameSite : 'Lax',
        expires: c.expirationDate ? c.expirationDate : -1,
      });
    }

    if (isTauri()) {
      // 注入 Cookie
      await executeCdpCommand(cdpEndpoint.value, siteUrl.value.trim(), 'Network.setCookies', {
        cookies: cdpCookiesToSet,
      });
      // 导航到目标站点
      await executeCdpCommand(cdpEndpoint.value, siteUrl.value.trim(), 'Page.navigate', {
        url: siteUrl.value.trim(),
      });
    }

    statusMessage.value = `导入完成。已成功注入 ${cdpCookiesToSet.length} 个 Cookie。目标浏览器保持运行。`;
    toast.success(
      '会话导入完成！',
      `已成功注入 ${cdpCookiesToSet.length} 个 Cookie。\n请在目标浏览器窗口中直接查看登录状态。`
    );
  } catch (err) {
    statusMessage.value = `操作失败：${err}`;
    toast.error('导入失败', String(err));
  } finally {
    isBusy.value = false;
  }
}

async function syncStatus() {
  if (isTauri()) {
    try {
      const status = await invoke<RunningBrowserInfo | null>('session_migrator_get_cdp_status');
      runningBrowser.value = status;
    } catch {
      // ignore
    }
  }
}

onMounted(async () => {
  if (isTauri()) {
    try {
      const defaults = await invoke<SessionMigratorDefaults>('session_migrator_get_defaults');
      sessionsDir.value = defaults.sessions_dir;
      profileDir.value =
        browserKind.value === 'chrome' ? defaults.default_chrome_profile : defaults.default_edge_profile;
      cdpPort.value = defaults.default_port;
      targetDomain.value = defaults.default_domain;
      siteUrl.value = defaults.default_site_url;
      handlePortChanged();
    } catch {
      // ignore
    }
  }

  await syncStatus();
  statusTimer = setInterval(syncStatus, 2500);
});

onUnmounted(() => {
  if (statusTimer) {
    clearInterval(statusTimer);
    statusTimer = null;
  }
});
</script>

<style scoped>
@keyframes indeterminate {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(300%);
  }
}

.animate-indeterminate {
  animation: indeterminate 1.5s infinite ease-in-out;
}
</style>

<template>
  <!-- 启动器主工作台视图容器 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题与状态统计栏 -->
    <header class="flex-shrink-0 px-8 pt-7 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <!-- 视图标题与简介 -->
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-blue-500/20 text-blue-400 border border-blue-500/30 flex items-center justify-center">
              <Rocket class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">快捷启动器</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            一键批量唤起工作环境软件、系统脚本与后台服务，支持控制台窗口彻底静默隐藏。
          </p>
        </div>

        <!-- 右侧全局操作区：新建与一键启动主按钮 -->
        <div class="flex items-center gap-3">
          <!-- 新建启动项按钮 -->
          <button
            @click="openCreateModal"
            class="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] active:bg-white/[0.15] border border-white/10 hover:border-white/20 transition-all"
          >
            <Plus class="w-4 h-4 text-gray-300" />
            <span>新建启动项</span>
          </button>

          <!-- 一键全部启动按钮 (核心电光蓝按钮) -->
          <button
            @click="handleLaunchAll"
            :disabled="isBatchLaunching || enabledCount === 0"
            class="group relative flex items-center gap-2 px-5 py-2 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-blue-600 via-blue-500 to-sky-500 hover:from-blue-500 hover:to-sky-400 active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none transition-all shadow-lg shadow-blue-500/25 border border-blue-400/30"
            :title="enabledCount === 0 ? '请先勾选需要启动的项目' : `一键按序拉起已勾选的 ${enabledCount} 个启动项`"
          >
            <Loader2 v-if="isBatchLaunching" class="w-4 h-4 animate-spin text-white" />
            <Sparkles v-else class="w-4 h-4 text-blue-200 group-hover:rotate-12 transition-transform" />
            <span>{{ isBatchLaunching ? '正在批量唤起...' : '一键全部启动' }}</span>
            <span
              v-if="enabledCount > 0 && !isBatchLaunching"
              class="ml-1 px-1.5 py-0.2 rounded-full text-[10px] font-mono bg-white/20 text-white"
            >
              {{ enabledCount }}
            </span>
          </button>
        </div>
      </div>

      <!-- 次级过滤与统计工具栏 -->
      <div class="mt-5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <!-- 搜索过滤框 -->
        <div class="relative w-full sm:w-72">
          <Search class="w-3.5 h-3.5 text-gray-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索启动项名称、程序路径或参数..."
            class="w-full pl-9 pr-8 py-1.5 rounded-lg bg-white/5 border border-white/10 text-xs text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors"
          />
          <button
            v-if="searchQuery"
            @click="searchQuery = ''"
            class="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 hover:text-white p-0.5"
            title="清空搜索"
          >
            <X class="w-3 h-3" />
          </button>
        </div>

        <!-- 统计状态胶囊徽章 -->
        <div class="flex items-center gap-2 text-xs">
          <div class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-white/[0.03] border border-white/10 text-gray-300">
            <Layers class="w-3.5 h-3.5 text-blue-400" />
            <span>共 <strong class="text-white font-mono">{{ totalCount }}</strong> 个项目</span>
            <span class="text-gray-600">·</span>
            <span>已选中 <strong class="text-emerald-400 font-mono">{{ enabledCount }}</strong> 项</span>
          </div>

          <!-- 快捷全选/反选开关 -->
          <button
            v-if="items.length > 0"
            @click="toggleSelectAll"
            class="text-[11px] text-gray-400 hover:text-blue-400 px-2 py-1 rounded hover:bg-white/5 transition-colors"
          >
            {{ allSelected ? '全部取消' : '一键全选' }}
          </button>
        </div>
      </div>
    </header>

    <!-- 卡片主体网格展示区 (自适应滚动) -->
    <main class="flex-1 min-h-0 overflow-y-auto px-8 py-6 win11-scrollbar">
      <!-- 初始加载中骨架或指示器 -->
      <div v-if="isLoading" class="h-64 flex flex-col items-center justify-center gap-3 text-gray-400">
        <Loader2 class="w-7 h-7 animate-spin text-blue-500" />
        <p class="text-xs">正在载入启动器配置...</p>
      </div>

      <!-- 空状态 1：完全无任何配置项 -->
      <div
        v-else-if="items.length === 0"
        class="h-96 flex flex-col items-center justify-center rounded-2xl border border-dashed border-white/10 bg-white/[0.01] p-8 text-center"
      >
        <div class="w-14 h-14 rounded-2xl bg-blue-500/10 border border-blue-500/20 text-blue-400 flex items-center justify-center mb-4 shadow-inner">
          <Rocket class="w-7 h-7" />
        </div>
        <h3 class="text-base font-semibold text-white">暂无自定义启动项</h3>
        <p class="text-xs text-gray-400 max-w-sm mt-1 mb-5 leading-relaxed">
          添加你日常高频使用的 IDE、终端、数据库或开发服务，享受一秒并发拉起的极速体验。
        </p>
        <button
          @click="openCreateModal"
          class="flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 active:bg-blue-700 transition-all shadow-md shadow-blue-500/20"
        >
          <Plus class="w-4 h-4" />
          <span>立即添加首个启动项</span>
        </button>
      </div>

      <!-- 空状态 2：搜索未找到匹配结果 -->
      <div
        v-else-if="filteredItems.length === 0"
        class="h-64 flex flex-col items-center justify-center rounded-xl border border-dashed border-white/5 bg-white/[0.01] p-6 text-center text-gray-400"
      >
        <Search class="w-8 h-8 text-gray-600 mb-2" />
        <p class="text-xs font-medium text-gray-300">未找到匹配的启动项</p>
        <p class="text-[11px] text-gray-500 mt-1">
          没有包含关键词 "<span class="text-blue-400">{{ searchQuery }}</span>" 的项目，请尝试更换检索词。
        </p>
        <button
          @click="searchQuery = ''"
          class="mt-3 text-xs text-blue-400 hover:underline"
        >
          清空搜索条件
        </button>
      </div>

      <!-- 正常列表：响应式 1 列或 2 列 Fluent 网格 -->
      <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-2 gap-4 pb-4">
        <LaunchItemCard
          v-for="item in filteredItems"
          :key="item.id"
          :item="item"
          :is-launching="launchingIds.has(item.id)"
          @toggle-enable="(val) => handleToggleEnable(item, val)"
          @launch="() => handleLaunchSingle(item)"
          @edit="() => openEditModal(item)"
          @delete="() => confirmDeleteItem(item)"
        />
      </div>
    </main>

    <!-- 新增 / 编辑启动项模态弹窗 -->
    <EditItemModal
      :show="showEditModal"
      :item="editingItem"
      @close="closeEditModal"
      @save="handleSaveItem"
    />

    <!-- 删除确认轻量对话框 -->
    <Transition name="fade">
      <div
        v-if="itemPendingDelete"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm select-none"
        @click.self="itemPendingDelete = null"
      >
        <div class="w-full max-w-sm bg-[#161b26] border border-white/10 rounded-xl p-5 shadow-2xl space-y-4">
          <div class="flex items-center gap-3 text-rose-400">
            <div class="w-9 h-9 rounded-lg bg-rose-500/10 border border-rose-500/20 flex items-center justify-center">
              <Trash2 class="w-4 h-4" />
            </div>
            <div>
              <h4 class="text-sm font-semibold text-white">确认移除启动项？</h4>
              <p class="text-xs text-gray-400 mt-0.5">此操作不会卸载本体程序文件</p>
            </div>
          </div>
          <div class="p-2.5 rounded-lg bg-white/5 text-xs text-gray-300 font-mono truncate">
            {{ itemPendingDelete.name }}
          </div>
          <div class="flex items-center justify-end gap-2 pt-1">
            <button
              @click="itemPendingDelete = null"
              class="px-3 py-1.5 rounded-lg text-xs text-gray-400 hover:text-white hover:bg-white/10 transition-colors"
            >
              取消
            </button>
            <button
              @click="executeDeleteItem"
              class="px-4 py-1.5 rounded-lg text-xs font-semibold text-white bg-rose-600 hover:bg-rose-500 transition-colors"
            >
              确认删除
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
/**
 * @file LauncherView.vue
 * @description 快速启动器主业务视图
 * 包含启动项卡片列表、搜索过滤、单项唤起、多项一键并发唤起及与 Rust 存储同步持久化
 */

import { ref, computed, onMounted } from 'vue';
import {
  Rocket,
  Plus,
  Search,
  Sparkles,
  Layers,
  X,
  Loader2,
  Trash2,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { LaunchItem, BatchLaunchResult } from '../types/module';
import { useToast } from '../composables/useToast';
import LaunchItemCard from '../components/launcher/LaunchItemCard.vue';
import EditItemModal from '../components/launcher/EditItemModal.vue';

const { toast } = useToast();

// 启动项列表响应式数据
const items = ref<LaunchItem[]>([]);

// 页面状态标记
const isLoading = ref<boolean>(false);
const isBatchLaunching = ref<boolean>(false);
const launchingIds = ref<Set<string>>(new Set());

// 搜索关键词
const searchQuery = ref<string>('');

// 模态弹窗控制
const showEditModal = ref<boolean>(false);
const editingItem = ref<LaunchItem | null>(null);

// 待确认删除的临时项
const itemPendingDelete = ref<LaunchItem | null>(null);

// ----------------------------------------------------
// 计算属性与过滤
// ----------------------------------------------------

/** 过滤后的启动项列表 (按搜索词匹配名称、路径或参数，增强空值安全防御) */
const filteredItems = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return items.value;
  return items.value.filter(
    (item) =>
      (item.name || '').toLowerCase().includes(q) ||
      (item.path || '').toLowerCase().includes(q) ||
      (item.args || '').toLowerCase().includes(q)
  );
});

/** 启动项总数 */
const totalCount = computed(() => items.value.length);

/** 当前启用的项数 */
const enabledCount = computed(() => items.value.filter((i) => i.enabled).length);

/** 是否全部处于启用状态 */
const allSelected = computed(() => {
  return items.value.length > 0 && items.value.every((i) => i.enabled);
});

// ----------------------------------------------------
// 后端通信与持久化调度
// ----------------------------------------------------

/**
 * 从后端加载启动项配置文件
 */
async function loadConfig() {
  isLoading.value = true;
  try {
    if (isTauri()) {
      const loaded = await invoke<LaunchItem[]>('load_launcher_config');
      items.value = loaded;
    } else {
      // 浏览器 Web 预览环境：使用本地存储模拟或提供默认演示配置
      const local = localStorage.getItem('omnibox_launcher_items');
      if (local) {
        items.value = JSON.parse(local);
      } else {
        items.value = [
          {
            id: 'demo-1',
            name: 'Windows 记事本',
            path: 'notepad.exe',
            args: '',
            work_dir: null,
            silent: false,
            enabled: true,
          },
          {
            id: 'demo-2',
            name: '系统计算器',
            path: 'calc.exe',
            args: '',
            work_dir: null,
            silent: false,
            enabled: true,
          },
          {
            id: 'demo-3',
            name: '后台批处理守护 (静默示例)',
            path: 'cmd.exe',
            args: '/c timeout /t 10',
            work_dir: null,
            silent: true,
            enabled: false,
          },
        ];
      }
    }
  } catch (err) {
    console.error('加载启动项配置失败:', err);
    toast.error('加载失败', `无法读取启动器配置文件: ${err}`);
  } finally {
    isLoading.value = false;
  }
}

/**
 * 将当前配置写回后端持久化存储
 */
async function saveConfig() {
  try {
    if (isTauri()) {
      await invoke('save_launcher_config', { items: items.value });
    } else {
      localStorage.setItem('omnibox_launcher_items', JSON.stringify(items.value));
    }
  } catch (err) {
    console.error('保存启动项配置失败:', err);
    toast.error('保存失败', `持久化启动配置出错: ${err}`);
  }
}

// ----------------------------------------------------
// 单项与批量执行业务流程
// ----------------------------------------------------

/**
 * 单项唤起执行
 */
async function handleLaunchSingle(item: LaunchItem) {
  if (launchingIds.value.has(item.id)) return;

  // 标记当前单项正在加载
  launchingIds.value.add(item.id);

  try {
    if (isTauri()) {
      const pid = await invoke<number>('execute_launch_item', { item });
      toast.success('启动成功', `${item.name} 已成功拉起 (PID: ${pid})`);
    } else {
      // 浏览器环境延时模拟执行
      await new Promise((resolve) => setTimeout(resolve, 500));
      const mockPid = Math.floor(1000 + Math.random() * 9000);
      toast.success('浏览器模拟启动', `${item.name} 运行就绪 (PID: ${mockPid})`);
    }
  } catch (err) {
    console.error(`拉起 ${item.name} 失败:`, err);
    toast.error('启动失败', `${item.name} 无法启动: ${err}`);
  } finally {
    launchingIds.value.delete(item.id);
  }
}

/**
 * 一键全部启动执行
 */
async function handleLaunchAll() {
  if (isBatchLaunching.value) return;

  if (enabledCount.value === 0) {
    toast.warning('提示', '未勾选任何启动项，请先在列表中勾选要运行的项目');
    return;
  }

  isBatchLaunching.value = true;

  try {
    if (isTauri()) {
      const results = await invoke<BatchLaunchResult[]>('execute_all_launch_items', {
        items: items.value,
      });

      // 统计批量执行结果
      const succeeded = results.filter((r) => r.success);
      const failed = results.filter(
        (r) => !r.success && r.error !== '启动项未启用，已跳过'
      );

      if (failed.length === 0) {
        toast.success(
          '一键启动完成',
          `已成功按序唤起全部 ${succeeded.length} 个应用程序`
        );
      } else if (succeeded.length > 0) {
        toast.warning(
          '部分启动成功',
          `成功唤起 ${succeeded.length} 项，失败 ${failed.length} 项 (如: ${failed[0].name} - ${failed[0].error})`
        );
      } else {
        toast.error(
          '一键启动失败',
          `所有已启用应用均未能成功启动，首个报错: ${failed[0]?.error || '未知错误'}`
        );
      }
    } else {
      // 浏览器模拟批量执行
      await new Promise((resolve) => setTimeout(resolve, 800));
      toast.success(
        '一键启动完成 (模拟)',
        `已成功模拟调度 ${enabledCount.value} 个启动项`
      );
    }
  } catch (err) {
    console.error('一键启动失败:', err);
    toast.error('执行异常', `批量启动过程出错: ${err}`);
  } finally {
    isBatchLaunching.value = false;
  }
}

// ----------------------------------------------------
// 用户交互与编辑操作
// ----------------------------------------------------

/**
 * 切换单个项的启用状态 (自动保存)
 */
async function handleToggleEnable(item: LaunchItem, enabled: boolean) {
  item.enabled = enabled;
  await saveConfig();
}

/**
 * 一键全选或全部反选
 */
async function toggleSelectAll() {
  const targetState = !allSelected.value;
  items.value.forEach((item) => {
    item.enabled = targetState;
  });
  await saveConfig();
  toast.info(
    '配置已更新',
    targetState ? `已启用全部 ${items.value.length} 个启动项` : '已全部排除一键启动'
  );
}

/**
 * 打开新建弹窗
 */
function openCreateModal() {
  editingItem.value = null;
  showEditModal.value = true;
}

/**
 * 打开编辑弹窗
 */
function openEditModal(item: LaunchItem) {
  // 浅拷贝对象传入弹窗
  editingItem.value = { ...item };
  showEditModal.value = true;
}

/**
 * 关闭弹窗
 */
function closeEditModal() {
  showEditModal.value = false;
  editingItem.value = null;
}

/**
 * 保存新建或修改的项并写回后端
 */
async function handleSaveItem(savedItem: LaunchItem) {
  const index = items.value.findIndex((i) => i.id === savedItem.id);
  if (index !== -1) {
    // 修改已有项
    items.value[index] = savedItem;
    toast.success('修改成功', `已更新 "${savedItem.name}" 配置`);
  } else {
    // 新增项添加到列表首部
    items.value.unshift(savedItem);
    toast.success('创建成功', `已添加新启动项 "${savedItem.name}"`);
  }

  closeEditModal();
  await saveConfig();
}

/**
 * 弹出删除确认框
 */
function confirmDeleteItem(item: LaunchItem) {
  itemPendingDelete.value = item;
}

/**
 * 执行删除
 */
async function executeDeleteItem() {
  if (!itemPendingDelete.value) return;
  const target = itemPendingDelete.value;
  items.value = items.value.filter((i) => i.id !== target.id);
  itemPendingDelete.value = null;

  await saveConfig();
  toast.info('已删除', `已移除启动项 "${target.name}"`);
}

// 页面挂载时自动载入持久化配置
onMounted(() => {
  loadConfig();
});
</script>

<style scoped>
/* 淡入淡出动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

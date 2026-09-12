<template>
  <!-- 启动项卡片容器：Fluent 磨砂卡片质感，支持状态透明度区分 -->
  <div
    class="group relative rounded-xl border p-4 transition-all duration-200 backdrop-blur-md flex flex-col justify-between gap-3"
    :class="[
      item.enabled
        ? 'bg-white/[0.04] border-white/10 hover:border-white/20 hover:bg-white/[0.06] shadow-sm hover:shadow-lg hover:shadow-black/30'
        : 'bg-white/[0.015] border-white/5 opacity-70 hover:opacity-90'
    ]"
  >
    <!-- 卡片头部：左侧复选框与标题，右侧静默模式徽章 -->
    <div class="flex items-start justify-between gap-2">
      <!-- 勾选与名称 -->
      <div class="flex items-center gap-3 min-w-0">
        <!-- 启用状态复选框 (控制是否参与一键启动) -->
        <label
          class="relative flex items-center justify-center w-5 h-5 rounded cursor-pointer transition-colors border select-none flex-shrink-0"
          :class="[
            item.enabled
              ? 'bg-blue-600 border-blue-500 text-white shadow-sm shadow-blue-500/30'
              : 'bg-white/5 border-white/20 hover:border-white/40 text-transparent'
          ]"
          :title="item.enabled ? '已启用：将参与一键启动' : '已停用：不参与一键启动'"
        >
          <input
            type="checkbox"
            class="sr-only"
            :checked="item.enabled"
            @change="handleToggleEnable"
          />
          <Check class="w-3.5 h-3.5 stroke-[3]" />
        </label>

        <!-- 项目名称与状态提示 -->
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <h3
              class="font-semibold text-sm truncate tracking-tight"
              :class="item.enabled ? 'text-white' : 'text-gray-400 line-through decoration-gray-500'"
              :title="item.name"
            >
              {{ item.name }}
            </h3>
          </div>
        </div>
      </div>

      <!-- 静默运行模式标识标签 -->
      <div class="flex items-center gap-1.5 flex-shrink-0">
        <span
          v-if="item.silent"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-purple-500/15 text-purple-300 border border-purple-500/30 shadow-xs"
          title="以静默无窗口模式在后台启动，隐藏控制台黑框与主窗体"
        >
          <EyeOff class="w-3 h-3" />
          <span>后台静默</span>
        </span>
        <span
          v-else
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-sky-500/10 text-sky-300 border border-sky-500/20"
          title="常规窗口模式启动"
        >
          <AppWindow class="w-3 h-3" />
          <span>常规窗口</span>
        </span>
      </div>
    </div>

    <!-- 卡片主体：路径与参数信息 -->
    <div class="space-y-1.5 min-w-0">
      <!-- 执行路径展示 (带省略和原生 Tooltip) -->
      <div
        class="flex items-center gap-1.5 text-xs text-gray-300 bg-black/25 px-2.5 py-1.5 rounded-lg border border-white/5 font-mono overflow-hidden"
        :title="'执行路径: ' + item.path"
      >
        <Terminal class="w-3.5 h-3.5 text-gray-400 flex-shrink-0" />
        <span class="truncate">{{ item.path }}</span>
      </div>

      <!-- 参数与工作目录徽章行 -->
      <div v-if="item.args || item.work_dir" class="flex flex-wrap items-center gap-1.5 pt-0.5 text-[11px]">
        <!-- 附加参数 -->
        <span
          v-if="item.args"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-white/5 text-gray-300 border border-white/10 font-mono truncate max-w-[240px]"
          :title="'命令行参数: ' + item.args"
        >
          <span class="text-gray-500 font-sans font-medium">参数:</span>
          <span class="truncate">{{ item.args }}</span>
        </span>

        <!-- 工作目录 -->
        <span
          v-if="item.work_dir"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-white/5 text-gray-400 border border-white/10 truncate max-w-[200px]"
          :title="'工作目录: ' + item.work_dir"
        >
          <Folder class="w-3 h-3 text-gray-500 flex-shrink-0" />
          <span class="truncate">{{ item.work_dir }}</span>
        </span>
      </div>
    </div>

    <!-- 卡片底部：状态说明与操作按钮 -->
    <div class="flex items-center justify-between pt-2 border-t border-white/5 gap-2">
      <!-- 左侧一键启动队列状态指示 -->
      <span class="text-[11px] text-gray-500 select-none">
        {{ item.enabled ? '已列入一键启动' : '已排除一键启动' }}
      </span>

      <!-- 右侧操作按钮组 -->
      <div class="flex items-center gap-1.5">
        <!-- 编辑按钮 -->
        <button
          @click="$emit('edit')"
          class="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-white/10 transition-colors"
          title="修改此项配置"
        >
          <Pencil class="w-3.5 h-3.5" />
        </button>

        <!-- 删除按钮 -->
        <button
          @click="$emit('delete')"
          class="p-1.5 rounded-lg text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
          title="从启动器列表中移除"
        >
          <Trash2 class="w-3.5 h-3.5" />
        </button>

        <!-- 单项启动按钮 -->
        <button
          @click="$emit('launch')"
          :disabled="isLaunching"
          class="flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-medium text-white bg-blue-600 hover:bg-blue-500 active:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-sm shadow-blue-500/20"
          :title="'立即单独运行 ' + item.name"
        >
          <Loader2 v-if="isLaunching" class="w-3.5 h-3.5 animate-spin" />
          <Play v-else class="w-3.5 h-3.5 fill-current" />
          <span>{{ isLaunching ? '拉起中' : '启动' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * @file LaunchItemCard.vue
 * @description 启动器单项卡片展示组件
 * 展示启动项名称、文件路径、参数、静默标识与单独启动交互
 */

import {
  Check,
  EyeOff,
  AppWindow,
  Terminal,
  Folder,
  Pencil,
  Trash2,
  Play,
  Loader2,
} from 'lucide-vue-next';
import type { LaunchItem } from '../../types/module';

defineProps<{
  /** 启动项数据模型 */
  item: LaunchItem;
  /** 当前单项是否正处于唤起加载状态 */
  isLaunching?: boolean;
}>();

const emit = defineEmits<{
  /** 切换一键启动启用状态 */
  (e: 'toggle-enable', enabled: boolean): void;
  /** 触发单项运行 */
  (e: 'launch'): void;
  /** 触发编辑配置 */
  (e: 'edit'): void;
  /** 触发删除配置 */
  (e: 'delete'): void;
}>();

function handleToggleEnable(evt: Event) {
  const target = evt.target as HTMLInputElement;
  emit('toggle-enable', target.checked);
}
</script>

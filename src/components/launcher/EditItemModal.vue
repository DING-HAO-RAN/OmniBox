<template>
  <!-- 模态弹窗遮罩层：Fluent 磨砂半透明背景 -->
  <Transition name="modal-fade">
    <div
      v-if="show"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm select-none"
      @click.self="handleClose"
      @keydown.esc="handleClose"
    >
      <!-- 模态框窗口卡片 -->
      <div
        class="w-full max-w-lg bg-[#161b26] border border-white/10 rounded-2xl shadow-2xl shadow-black/60 overflow-hidden flex flex-col animate-scale-in"
        role="dialog"
        aria-modal="true"
      >
        <!-- 弹窗头部 -->
        <div class="px-6 py-4 border-b border-white/10 flex items-center justify-between bg-white/[0.02]">
          <div class="flex items-center gap-2.5">
            <div class="w-8 h-8 rounded-lg bg-blue-500/20 text-blue-400 border border-blue-500/30 flex items-center justify-center">
              <Rocket class="w-4 h-4" />
            </div>
            <div>
              <h2 class="text-sm font-bold text-white tracking-tight">
                {{ isEditMode ? '编辑启动项配置' : '新建自定义启动项' }}
              </h2>
              <p class="text-xs text-gray-400">配置 Windows 应用程序、脚本或后台守护项</p>
            </div>
          </div>
          <button
            @click="handleClose"
            class="p-1 rounded-lg text-gray-400 hover:text-white hover:bg-white/10 transition-colors"
            title="关闭弹窗 (Esc)"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- 弹窗表单主体 (支持内容过多时内部滚动) -->
        <form @submit.prevent="handleSubmit" class="p-6 space-y-4 overflow-y-auto max-h-[72vh] win11-scrollbar">
          <!-- 目标名称 (必填) -->
          <div class="space-y-1.5">
            <label class="block text-xs font-semibold text-gray-300">
              目标名称 <span class="text-rose-400">*</span>
            </label>
            <input
              v-model.trim="form.name"
              type="text"
              placeholder="例如: VS Code / 本地开发服务"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border text-xs text-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 transition-colors"
              :class="errors.name ? 'border-rose-500/80 bg-rose-500/5' : 'border-white/10 focus:border-blue-500'"
            />
            <p v-if="errors.name" class="text-[11px] text-rose-400">{{ errors.name }}</p>
          </div>

          <!-- 执行路径 (必填) -->
          <div class="space-y-1.5">
            <label class="block text-xs font-semibold text-gray-300">
              执行路径 / 命令 <span class="text-rose-400">*</span>
            </label>
            <input
              v-model.trim="form.path"
              type="text"
              placeholder="例如: notepad.exe / code / C:\Tools\my-app.exe"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border text-xs text-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 transition-colors font-mono"
              :class="errors.path ? 'border-rose-500/80 bg-rose-500/5' : 'border-white/10 focus:border-blue-500'"
            />
            <p v-if="errors.path" class="text-[11px] text-rose-400">{{ errors.path }}</p>
            <p class="text-[11px] text-gray-500">
              支持系统可执行程序全路径、PATH 系统环境变量内的命令名或批处理脚本文件 (.bat/.cmd)
            </p>
          </div>

          <!-- 附加命令行参数 (选填) -->
          <div class="space-y-1.5">
            <label class="block text-xs font-semibold text-gray-300 flex items-center justify-between">
              <span>附加命令行参数</span>
              <span class="text-[10px] text-gray-500 font-normal">选填</span>
            </label>
            <input
              v-model.trim="form.args"
              type="text"
              placeholder="例如: --port 3000 --debug / /min"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-xs text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors font-mono"
            />
          </div>

          <!-- 工作目录 (选填) -->
          <div class="space-y-1.5">
            <label class="block text-xs font-semibold text-gray-300 flex items-center justify-between">
              <span>指定工作目录 (WorkDir)</span>
              <span class="text-[10px] text-gray-500 font-normal">选填</span>
            </label>
            <input
              v-model.trim="form.work_dir"
              type="text"
              placeholder="例如: D:\Projects\MyProject (留空则继承系统默认)"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-xs text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors font-mono"
            />
          </div>

          <!-- 开关设置区域 -->
          <div class="pt-2 space-y-3 border-t border-white/5">
            <!-- 静默无窗口启动开关 -->
            <div class="flex items-center justify-between p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <div class="space-y-0.5 pr-4">
                <div class="flex items-center gap-1.5">
                  <EyeOff class="w-3.5 h-3.5 text-purple-400" />
                  <span class="text-xs font-medium text-gray-200">后台静默启动</span>
                </div>
                <p class="text-[11px] text-gray-400 leading-relaxed">
                  通过 Windows 隐藏窗口标志创建进程，彻底消除命令行黑框闪烁，适用于后台守护服务。
                </p>
              </div>
              <button
                type="button"
                role="switch"
                :aria-checked="form.silent"
                @click="form.silent = !form.silent"
                class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none"
                :class="form.silent ? 'bg-purple-600' : 'bg-gray-700'"
              >
                <span
                  class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out"
                  :class="form.silent ? 'translate-x-4' : 'translate-x-0'"
                />
              </button>
            </div>

            <!-- 参与一键启动开关 -->
            <div class="flex items-center justify-between p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <div class="space-y-0.5 pr-4">
                <div class="flex items-center gap-1.5">
                  <Play class="w-3.5 h-3.5 text-blue-400" />
                  <span class="text-xs font-medium text-gray-200">参与一键批量启动</span>
                </div>
                <p class="text-[11px] text-gray-400 leading-relaxed">
                  开启后，点击主界面的「一键全部启动」将按序调度唤起此应用。
                </p>
              </div>
              <button
                type="button"
                role="switch"
                :aria-checked="form.enabled"
                @click="form.enabled = !form.enabled"
                class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none"
                :class="form.enabled ? 'bg-blue-600' : 'bg-gray-700'"
              >
                <span
                  class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out"
                  :class="form.enabled ? 'translate-x-4' : 'translate-x-0'"
                />
              </button>
            </div>
          </div>
        </form>

        <!-- 弹窗底部操作栏 -->
        <div class="px-6 py-3.5 border-t border-white/10 bg-white/[0.01] flex items-center justify-end gap-3">
          <button
            type="button"
            @click="handleClose"
            class="px-4 py-1.5 rounded-lg text-xs font-medium text-gray-300 hover:text-white hover:bg-white/10 transition-colors"
          >
            取消
          </button>
          <button
            type="button"
            @click="handleSubmit"
            class="px-5 py-1.5 rounded-lg text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 active:bg-blue-700 transition-all shadow-md shadow-blue-500/20"
          >
            {{ isEditMode ? '保存修改' : '立即创建' }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
/**
 * @file EditItemModal.vue
 * @description 启动项新增/编辑模态弹窗组件
 * 支持设定程序名称、运行路径、启动参数、工作目录、后台静默模式与一键启动激活状态
 */

import { computed, watch, reactive, onMounted, onUnmounted } from 'vue';
import { Rocket, X, EyeOff, Play } from 'lucide-vue-next';
import type { LaunchItem } from '../../types/module';

const props = defineProps<{
  /** 是否展示弹窗 */
  show: boolean;
  /** 若传入待编辑的对象则为修改模式，若为 null/undefined 则为新建模式 */
  item?: LaunchItem | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', item: LaunchItem): void;
}>();

// 是否为编辑现有启动项
const isEditMode = computed(() => !!props.item);

// 全局 Esc 快捷键关闭监听
function handleGlobalKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.show) {
    handleClose();
  }
}

onMounted(() => {
  if (props.show) {
    window.addEventListener('keydown', handleGlobalKeydown);
  }
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleGlobalKeydown);
});

// 表单响应式数据
const form = reactive<{
  id: string;
  name: string;
  path: string;
  args: string;
  work_dir: string;
  silent: boolean;
  enabled: boolean;
}>({
  id: '',
  name: '',
  path: '',
  args: '',
  work_dir: '',
  silent: false,
  enabled: true,
});

// 表单错误校验记录
const errors = reactive<{
  name?: string;
  path?: string;
}>({});

// 监听弹窗打开状态，及时填充或重置表单内容及事件绑定
watch(
  () => props.show,
  (visible) => {
    if (visible) {
      window.addEventListener('keydown', handleGlobalKeydown);
      errors.name = undefined;
      errors.path = undefined;
      if (props.item) {
        form.id = props.item.id;
        form.name = props.item.name;
        form.path = props.item.path;
        form.args = props.item.args || '';
        form.work_dir = props.item.work_dir || '';
        form.silent = props.item.silent;
        form.enabled = props.item.enabled;
      } else {
        // 新增初始化默认值
        form.id = `launch-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
        form.name = '';
        form.path = '';
        form.args = '';
        form.work_dir = '';
        form.silent = false;
        form.enabled = true;
      }
    } else {
      window.removeEventListener('keydown', handleGlobalKeydown);
    }
  },
  { immediate: true }
);

/**
 * 关闭弹窗处理
 */
function handleClose() {
  emit('close');
}

/**
 * 校验并提交表单
 */
function handleSubmit() {
  errors.name = undefined;
  errors.path = undefined;

  let isValid = true;
  if (!form.name.trim()) {
    errors.name = '请输入启动项显示名称';
    isValid = false;
  }
  if (!form.path.trim()) {
    errors.path = '请输入目标程序的可执行路径或命令';
    isValid = false;
  }

  if (!isValid) return;

  const resultItem: LaunchItem = {
    id: form.id,
    name: form.name.trim(),
    path: form.path.trim(),
    args: form.args.trim(),
    work_dir: form.work_dir.trim() ? form.work_dir.trim() : null,
    silent: form.silent,
    enabled: form.enabled,
  };

  emit('save', resultItem);
}
</script>

<style scoped>
/* 模态框淡入淡出动画 */
.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}

/* 缩放滑入动画 */
@keyframes scaleIn {
  from {
    transform: scale(0.96);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

.animate-scale-in {
  animation: scaleIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}
</style>

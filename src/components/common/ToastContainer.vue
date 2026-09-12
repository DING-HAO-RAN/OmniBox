<template>
  <!-- 全局 Toast 提示容器：固定于视窗右上角，层级最高 -->
  <aside
    class="fixed top-12 right-4 z-50 flex flex-col gap-2.5 max-w-sm w-full pointer-events-none"
    aria-live="polite"
    aria-atomic="true"
  >
    <TransitionGroup
      name="toast-slide"
      tag="div"
      class="flex flex-col gap-2.5 w-full"
    >
      <div
        v-for="item in toasts"
        :key="item.id"
        class="pointer-events-auto flex items-start gap-3 p-3.5 rounded-xl border shadow-xl backdrop-blur-xl transition-all duration-200"
        :class="getToastStyle(item.type)"
        role="alert"
      >
        <!-- 状态指示图标 -->
        <div class="flex-shrink-0 mt-0.5">
          <component :is="getIconComponent(item.type)" class="w-4 h-4" />
        </div>

        <!-- 文本正文区域 -->
        <div class="flex-1 min-w-0">
          <p class="text-xs font-semibold text-gray-100 leading-tight">
            {{ item.title }}
          </p>
          <p v-if="item.message" class="text-[11px] text-gray-300/80 mt-0.5 leading-relaxed break-words">
            {{ item.message }}
          </p>
        </div>

        <!-- 手动关闭操作按钮 -->
        <button
          @click="removeToast(item.id)"
          class="flex-shrink-0 p-1 text-gray-400 hover:text-white rounded-md hover:bg-white/10 transition-colors"
          title="关闭提示"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>
    </TransitionGroup>
  </aside>
</template>

<script setup lang="ts">
/**
 * @file ToastContainer.vue
 * @description Windows 11 Fluent 风格全局 Toast 弹窗容器
 */

import { CheckCircle2, AlertTriangle, XCircle, Info, X } from 'lucide-vue-next';
import { useToast } from '../../composables/useToast';
import type { ToastType } from '../../types/module';

const { toasts, removeToast } = useToast();

/**
 * 根据消息类型获取对应的视觉样式类名
 */
function getToastStyle(type: ToastType): string {
  switch (type) {
    case 'success':
      return 'bg-emerald-950/80 border-emerald-500/30 text-emerald-400 shadow-emerald-950/30';
    case 'warning':
      return 'bg-amber-950/80 border-amber-500/30 text-amber-400 shadow-amber-950/30';
    case 'error':
      return 'bg-rose-950/80 border-rose-500/30 text-rose-400 shadow-rose-950/30';
    case 'info':
    default:
      return 'bg-slate-900/85 border-blue-500/30 text-sky-400 shadow-blue-950/30';
  }
}

/**
 * 根据消息类型获取对应的 Lucide 图标组件
 */
function getIconComponent(type: ToastType) {
  switch (type) {
    case 'success':
      return CheckCircle2;
    case 'warning':
      return AlertTriangle;
    case 'error':
      return XCircle;
    case 'info':
    default:
      return Info;
  }
}
</script>

<style scoped>
/* Toast 进出场平滑弹动动画 */
.toast-slide-enter-active,
.toast-slide-leave-active {
  transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.toast-slide-enter-from {
  opacity: 0;
  transform: translateX(30px) scale(0.95);
}

.toast-slide-leave-to {
  opacity: 0;
  transform: translateX(40px) scale(0.92);
}
</style>

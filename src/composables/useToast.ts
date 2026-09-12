/**
 * @file useToast.ts
 * @description 全局响应式 Toast 消息通知状态管理
 * 提供非阻塞、自动延时销毁的轻量级桌面风格通知机制
 */

import { ref } from 'vue';
import type { ToastMessage, ToastType } from '../types/module';

// 全局响应式消息队列
const toasts = ref<ToastMessage[]>([]);

/**
 * 触发一条全局 Toast 消息
 * @param options Toast 构造选项
 * @returns 自动生成的消自 ID
 */
export function showToast(options: {
  title: string;
  message?: string;
  type?: ToastType;
  duration?: number;
}): string {
  const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
  const duration = options.duration ?? 3200;
  const newToast: ToastMessage = {
    id,
    title: options.title,
    message: options.message,
    type: options.type ?? 'info',
    duration,
  };

  // 添加到消息队首
  toasts.value.push(newToast);

  // 定时自动清除
  if (duration > 0) {
    setTimeout(() => {
      removeToast(id);
    }, duration);
  }

  return id;
}

/**
 * 手动移除指定 ID 的 Toast
 * @param id 消息 ID
 */
export function removeToast(id: string): void {
  const index = toasts.value.findIndex((t) => t.id === id);
  if (index !== -1) {
    toasts.value.splice(index, 1);
  }
}

/**
 * 快捷封装调用
 */
export const toast = {
  info: (title: string, message?: string, duration?: number) =>
    showToast({ title, message, type: 'info', duration }),
  success: (title: string, message?: string, duration?: number) =>
    showToast({ title, message, type: 'success', duration }),
  warning: (title: string, message?: string, duration?: number) =>
    showToast({ title, message, type: 'warning', duration }),
  error: (title: string, message?: string, duration?: number) =>
    showToast({ title, message, type: 'error', duration }),
};

/**
 * 导出 Composable 供组件消费
 */
export function useToast() {
  return {
    toasts,
    showToast,
    removeToast,
    toast,
  };
}

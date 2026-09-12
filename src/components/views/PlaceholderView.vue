<template>
  <!-- 模块占位视图组件：用于尚未挂载或正在装载的插件 -->
  <div class="h-full flex flex-col items-center justify-center p-8 text-center">
    <div class="w-16 h-16 rounded-2xl bg-white/[0.04] border border-white/10 flex items-center justify-center text-blue-400 mb-4 shadow-xl">
      <Construction class="w-8 h-8" />
    </div>
    <h2 class="text-xl font-bold text-white mb-2">{{ title }}</h2>
    <p class="text-sm text-gray-400 max-w-md mb-6 leading-relaxed">
      {{ description || '该功能模块正在接入中，敬请期待。' }}
    </p>
    <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-blue-500/10 border border-blue-500/20 text-xs text-blue-400">
      <span class="w-2 h-2 rounded-full bg-blue-400 animate-ping"></span>
      <span>模块插槽已就绪</span>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * @file PlaceholderView.vue
 * @description 工具模块占位展示组件
 */

import { Construction } from 'lucide-vue-next';

defineProps<{
  title?: string;
  description?: string;
}>();
</script>

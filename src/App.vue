<template>
  <!-- 根视图挂载容器 -->
  <div id="app-root" class="w-screen h-screen overflow-hidden select-none bg-[#0d1117]">
    <!-- 全局 Fluent 风格桌面外壳 -->
    <AppShell />

    <!-- 全局浮动 Toast 消息通知中心 -->
    <ToastContainer />
  </div>
</template>

<script setup lang="ts">
/**
 * @file App.vue
 * @description 应用程序根组件
 * 组合桌面核心外壳 (AppShell) 与全局即时反馈通知 (ToastContainer)
 */

import AppShell from './components/layout/AppShell.vue';
import ToastContainer from './components/common/ToastContainer.vue';
</script>

<template>
  <!-- 桌面外壳主架构容器：高度占满视窗，禁止外部滚动 -->
  <div class="h-screen w-screen flex flex-col overflow-hidden bg-[#0d1117] text-white">
    <!-- 顶部统一自定义标题栏 -->
    <HeaderBar />

    <!-- 核心工作区：左侧边栏导航 + 右侧动态模块视窗 -->
    <div class="flex-1 flex min-h-0 overflow-hidden relative">
      <!-- 磨砂侧边栏 -->
      <Sidebar />

      <!-- 主工作区内容插槽与视图宿主 -->
      <main class="flex-1 h-full min-w-0 overflow-hidden relative bg-[#0e131b]">
        <!-- 页面切换平滑过渡动效：标准 Vue 3 Transition 模板实现 -->
        <Transition name="fade-slide" mode="out-in">
          <div
            v-if="activeTool"
            :key="activeTool.id"
            class="h-full w-full overflow-hidden"
          >
            <component :is="activeTool.component" />
          </div>
          <div
            v-else
            key="empty-state"
            class="h-full flex items-center justify-center text-gray-500"
          >
            未选择任何工具模块
          </div>
        </Transition>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * @file AppShell.vue
 * @description Windows 11 Fluent 桌面客户端核心外壳系统
 * 集成 HeaderBar、Sidebar 与标准 Transition 视图动态切换插槽
 */

import HeaderBar from './HeaderBar.vue';
import Sidebar from './Sidebar.vue';
import { activeTool } from '../../registry';
</script>

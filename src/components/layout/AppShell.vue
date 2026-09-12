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
        <!-- 页面切换平滑过渡动效 -->
        <RouterViewContainer />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * @file AppShell.vue
 * @description Windows 11 Fluent 桌面客户端核心外壳系统
 * 集成 HeaderBar、Sidebar 与可插拔的模块化视图切换器
 */

import { defineComponent, h } from 'vue';
import HeaderBar from './HeaderBar.vue';
import Sidebar from './Sidebar.vue';
import { activeTool } from '../../registry';

/**
 * 内部模块视图过渡切换器组件
 */
const RouterViewContainer = defineComponent({
  name: 'RouterViewContainer',
  setup() {
    return () => {
      const tool = activeTool.value;
      if (!tool) {
        return h('div', { class: 'h-full flex items-center justify-center text-gray-500' }, '未选择任何工具模块');
      }

      // 渲染带过渡动画的动态插槽组件
      return h(
        'transition',
        {
          name: 'fade-slide',
          mode: 'out-in',
        },
        () => [
          h(
            'div',
            {
              key: tool.id,
              class: 'h-full w-full overflow-hidden',
            },
            [h(tool.component)]
          ),
        ]
      );
    };
  },
});
</script>

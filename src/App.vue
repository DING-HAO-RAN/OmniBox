<template>
  <!-- 主视窗容器：Windows 11 Fluent 风格居中布局 -->
  <main class="h-screen w-screen flex flex-col items-center justify-center bg-[#0d1117] text-white p-6 select-none">
    <!-- 亚克力磨砂拟态卡片 -->
    <div class="p-8 rounded-2xl border border-white/10 bg-white/[0.03] backdrop-blur-xl shadow-2xl flex flex-col items-center max-w-md text-center transition-all duration-300 hover:border-white/20">
      <!-- 标志性应用图标容器 -->
      <div class="w-16 h-16 rounded-2xl bg-blue-600/20 border border-blue-500/30 flex items-center justify-center mb-4 text-blue-400 shadow-inner">
        <Layers class="w-8 h-8" />
      </div>

      <!-- 标题与简要描述 -->
      <h1 class="text-2xl font-bold tracking-tight mb-2">OmniBox 万能工具箱</h1>
      <p class="text-sm text-gray-400 mb-6 leading-relaxed">
        Windows 现代化极简工具箱 · 超轻量低内存占用
      </p>

      <!-- 状态标签指示器 -->
      <div class="flex items-center gap-2.5 text-xs font-medium text-emerald-400 bg-emerald-500/10 px-3.5 py-1.5 rounded-full border border-emerald-500/20">
        <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
        <span>脚手架初始化就绪 (Tauri v2 + Vue 3)</span>
      </div>
    </div>
  </main>
</template>

<script setup lang="ts">
// 引入 Lucide 现代化图标
import { Layers } from "lucide-vue-next";
</script>

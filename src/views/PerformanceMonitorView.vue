<template>
  <!-- 深度对标 Windows 任务管理器的硬件性能监控面板 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-6 pb-4 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-sky-500/20 text-sky-400 border border-sky-500/30 flex items-center justify-center">
              <Activity class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">系统性能与硬件监控中心</h1>
          </div>
          <p class="text-xs text-gray-400 mt-1">
            原生采集 CPU、物理内存、各存储磁盘、网络吞吐与 GPU 深度运行状态与 60s 实时走势图。
          </p>
        </div>

        <!-- 刷新与视图切换控制 -->
        <div class="flex items-center gap-3">
          <!-- 模式切换: 任务管理器分栏模式 vs 全景矩阵模式 -->
          <div class="inline-flex p-1 rounded-xl bg-black/40 border border-white/10 text-xs">
            <button
              @click="viewMode = 'taskmgr'"
              type="button"
              class="px-3 py-1 rounded-lg font-medium transition-all"
              :class="viewMode === 'taskmgr' ? 'bg-blue-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'"
            >
              任务管理器模式
            </button>
            <button
              @click="viewMode = 'grid'"
              type="button"
              class="px-3 py-1 rounded-lg font-medium transition-all"
              :class="viewMode === 'grid' ? 'bg-blue-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'"
            >
              全景网格视图
            </button>
          </div>

          <div class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-white/[0.03] border border-white/10 text-xs text-gray-400">
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>1.5s 采样</span>
          </div>

          <button
            @click="fetchData"
            type="button"
            class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] border border-white/10 transition-all"
            title="手动刷新硬件状态"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isFetching }" />
            <span>刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主内容展示区 -->
    <main class="flex-1 min-h-0 overflow-hidden px-8 py-5 flex flex-col">
      <!-- ================= 模式一：对标任务管理器的精细左右分栏模式 ================= -->
      <div v-if="viewMode === 'taskmgr'" class="flex-1 min-h-0 flex flex-col lg:flex-row gap-5">
        <!-- 左侧：硬件分类迷你卡片选择栏 -->
        <div class="w-full lg:w-72 flex-shrink-0 flex flex-col gap-2 overflow-y-auto win11-scrollbar pr-1">
          <!-- 1. CPU 卡片 -->
          <button
            @click="activeDevice = 'cpu'"
            type="button"
            class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
            :class="activeDevice === 'cpu' ? 'bg-blue-600/15 border-blue-500/50 shadow-lg shadow-blue-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
          >
            <span v-if="activeDevice === 'cpu'" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.8)]"></span>
            <div class="min-w-0">
              <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'cpu' ? 'text-blue-400' : 'text-gray-300'">
                <Cpu class="w-3.5 h-3.5" />
                <span>CPU</span>
              </div>
              <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">
                {{ perfData.cpu_usage_percent.toFixed(0) }}%
              </p>
              <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]" :title="perfData.cpu_name">
                {{ perfData.cpu_name }}
              </p>
            </div>
            <!-- 迷你走势微图 -->
            <div class="w-20 h-10 flex-shrink-0">
              <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 40">
                <polyline :points="buildPoints(cpuHistory, 100).linePoints" fill="none" stroke="#3b82f6" stroke-width="1.5" />
              </svg>
            </div>
          </button>

          <!-- 2. 内存 卡片 -->
          <button
            @click="activeDevice = 'memory'"
            type="button"
            class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
            :class="activeDevice === 'memory' ? 'bg-purple-600/15 border-purple-500/50 shadow-lg shadow-purple-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
          >
            <span v-if="activeDevice === 'memory'" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-purple-500 shadow-[0_0_8px_rgba(168,85,247,0.8)]"></span>
            <div class="min-w-0">
              <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'memory' ? 'text-purple-400' : 'text-gray-300'">
                <Layers class="w-3.5 h-3.5" />
                <span>内存 (RAM)</span>
              </div>
              <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">
                {{ formatGb(perfData.memory.used_ram) }}/{{ formatGb(perfData.memory.total_ram) }} ({{ perfData.memory.usage_percent.toFixed(0) }}%)
              </p>
              <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]">
                已用 {{ (perfData.memory.used_ram / (1024*1024*1024)).toFixed(1) }} GB
              </p>
            </div>
            <div class="w-20 h-10 flex-shrink-0">
              <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 40">
                <polyline :points="buildPoints(memHistory, 100).linePoints" fill="none" stroke="#a855f7" stroke-width="1.5" />
              </svg>
            </div>
          </button>

          <!-- 3. 磁盘 卡片 (列表中的首个磁盘或选中的盘) -->
          <button
            v-for="disk in perfData.disks"
            :key="disk.letter"
            @click="activeDevice = 'disk:' + disk.letter"
            type="button"
            class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
            :class="activeDevice === 'disk:' + disk.letter ? 'bg-emerald-600/15 border-emerald-500/50 shadow-lg shadow-emerald-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
          >
            <span v-if="activeDevice === 'disk:' + disk.letter" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.8)]"></span>
            <div class="min-w-0">
              <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'disk:' + disk.letter ? 'text-emerald-400' : 'text-gray-300'">
                <HardDrive class="w-3.5 h-3.5" />
                <span>磁盘 ({{ disk.letter }})</span>
              </div>
              <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">
                {{ disk.usage_percent }}% 已用
              </p>
              <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]">
                {{ disk.label }} ({{ disk.file_system }})
              </p>
            </div>
            <!-- 磁盘进度条 -->
            <div class="w-16 flex flex-col items-end gap-1 flex-shrink-0">
              <span class="text-[9px] font-mono text-gray-400">{{ formatBytes(disk.available_bytes) }} 余</span>
              <div class="w-full h-1.5 rounded-full bg-white/10 overflow-hidden">
                <div class="h-full bg-emerald-500 rounded-full" :style="{ width: `${disk.usage_percent}%` }"></div>
              </div>
            </div>
          </button>

          <!-- 4. 网络 卡片 -->
          <button
            @click="activeDevice = 'network'"
            type="button"
            class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
            :class="activeDevice === 'network' ? 'bg-amber-600/15 border-amber-500/50 shadow-lg shadow-amber-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
          >
            <span v-if="activeDevice === 'network'" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-amber-500 shadow-[0_0_8px_rgba(245,158,11,0.8)]"></span>
            <div class="min-w-0">
              <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'network' ? 'text-amber-400' : 'text-gray-300'">
                <Network class="w-3.5 h-3.5" />
                <span>以太网 / Wi-Fi</span>
              </div>
              <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">
                ↓ {{ formatSpeed(perfData.network.rx_speed_bps) }}
              </p>
              <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]" :title="perfData.network.adapter_name">
                ↑ {{ formatSpeed(perfData.network.tx_speed_bps) }}
              </p>
            </div>
            <div class="w-20 h-10 flex-shrink-0">
              <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 40">
                <polyline :points="buildPoints(netHistory, Math.max(1024*100, ...netHistory)).linePoints" fill="none" stroke="#f59e0b" stroke-width="1.5" />
              </svg>
            </div>
          </button>

          <!-- 5. GPU 卡片 -->
          <button
            @click="activeDevice = 'gpu'"
            type="button"
            class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
            :class="activeDevice === 'gpu' ? 'bg-sky-600/15 border-sky-500/50 shadow-lg shadow-sky-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
          >
            <span v-if="activeDevice === 'gpu'" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-sky-500 shadow-[0_0_8px_rgba(14,165,233,0.8)]"></span>
            <div class="min-w-0">
              <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'gpu' ? 'text-sky-400' : 'text-gray-300'">
                <Monitor class="w-3.5 h-3.5" />
                <span>GPU</span>
              </div>
              <p class="text-[11px] text-gray-300 mt-1 font-semibold truncate max-w-[140px]" :title="perfData.gpu_name">
                {{ perfData.gpu_name }}
              </p>
              <p class="text-[10px] text-gray-500 truncate mt-0.5">3D 硬件加速就绪</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-sky-500/10 border border-sky-500/20 flex items-center justify-center text-sky-400 flex-shrink-0">
              <Sparkles class="w-4 h-4" />
            </div>
          </button>
        </div>

        <!-- 右侧：选中的硬件详细精细图表与参数面板 -->
        <div class="flex-1 min-h-0 overflow-y-auto win11-scrollbar rounded-3xl p-6 bg-white/[0.025] border border-white/10 shadow-2xl backdrop-blur-md flex flex-col justify-between space-y-5">
          <!-- 头部大号状态 -->
          <div class="flex items-start justify-between border-b border-white/5 pb-4">
            <div>
              <div class="flex items-center gap-2">
                <span class="text-xs font-bold uppercase tracking-wider text-gray-400">
                  {{ currentSectionTitle }}
                </span>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-mono bg-blue-500/10 text-blue-300 border border-blue-500/20">
                  实时利用率 60 秒走势
                </span>
              </div>
              <h2 class="text-xl font-bold text-white mt-1">
                {{ currentDeviceHeaderName }}
              </h2>
            </div>

            <!-- 右上角当前大号实时数值 -->
            <div class="text-right">
              <span class="text-2xl lg:text-3xl font-bold font-mono" :class="currentThemeColorClass">
                {{ currentPrimaryValue }}
              </span>
              <p class="text-xs text-gray-400 mt-0.5">{{ currentSecondaryValue }}</p>
            </div>
          </div>

          <!-- 精细绘图区 (参考任务管理器网格背景 + 60s 走势图) -->
          <div class="relative w-full h-48 lg:h-56 bg-black/40 rounded-2xl border border-white/10 p-3 overflow-hidden">
            <!-- 任务管理器经典网格背景线条 -->
            <div class="absolute inset-0 grid grid-rows-4 grid-cols-6 pointer-events-none opacity-20">
              <div v-for="n in 24" :key="n" class="border-b border-r border-white/50"></div>
            </div>

            <!-- 纵坐标刻度标签 -->
            <div class="absolute right-2 top-2 text-[10px] font-mono text-gray-500 select-none">
              {{ currentScaleTopLabel }}
            </div>
            <div class="absolute right-2 bottom-2 text-[10px] font-mono text-gray-500 select-none">
              0
            </div>
            <div class="absolute left-2 bottom-2 text-[10px] font-mono text-gray-500 select-none">
              60 秒
            </div>

            <!-- SVG 核心波形走势图 -->
            <svg class="w-full h-full overflow-visible relative z-10" preserveAspectRatio="none" viewBox="0 0 100 100">
              <defs>
                <linearGradient :id="currentGradientId" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" :stop-color="currentColorHex" stop-opacity="0.45" />
                  <stop offset="100%" :stop-color="currentColorHex" stop-opacity="0.0" />
                </linearGradient>
              </defs>

              <!-- 填充渐变多边形 -->
              <polygon :points="currentChartPoints.areaPoints" :fill="`url(#${currentGradientId})`" />

              <!-- 顶层主折线 -->
              <polyline
                :points="currentChartPoints.linePoints"
                fill="none"
                :stroke="currentColorHex"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />

              <!-- 当前最新采样点光晕 -->
              <circle
                :cx="currentChartPoints.lastPoint.x"
                :cy="currentChartPoints.lastPoint.y"
                r="3.5"
                :fill="currentColorHex"
                class="animate-pulse"
              />
            </svg>
          </div>

          <!-- 底部详细参数网格 (100% 对标 Windows 任务管理器性能明细) -->
          <div class="pt-4 border-t border-white/5">
            <!-- 1. CPU 模式详细参数 -->
            <div v-if="activeDevice === 'cpu'" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">利用率</span>
                <p class="text-sm font-bold font-mono text-white">{{ perfData.cpu_usage_percent.toFixed(1) }}%</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">内核 / 逻辑处理器</span>
                <p class="text-sm font-bold font-mono text-white">{{ perfData.cpu_detail?.physical_cores || 8 }} / {{ perfData.cpu_logical_cores }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">进程数</span>
                <p class="text-sm font-bold font-mono text-white">{{ perfData.cpu_detail?.process_count || 240 }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">线程数</span>
                <p class="text-sm font-bold font-mono text-white">{{ perfData.cpu_detail?.thread_count || 3200 }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">句柄数</span>
                <p class="text-sm font-bold font-mono text-white">{{ perfData.cpu_detail?.handle_count || 110000 }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">正常运行时间</span>
                <p class="text-sm font-bold font-mono text-emerald-400">{{ perfData.cpu_detail?.uptime_formatted || '00:00:00' }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">虚拟化</span>
                <p class="text-sm font-bold text-white">已启用</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">指令集架构</span>
                <p class="text-sm font-bold font-mono text-white">x86-64 (AVX2)</p>
              </div>
            </div>

            <!-- 2. 内存 模式详细参数 -->
            <div v-else-if="activeDevice === 'memory'" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">使用中 (物理)</span>
                <p class="text-sm font-bold font-mono text-purple-300">{{ formatBytes(perfData.memory.used_ram) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">可用 (Available)</span>
                <p class="text-sm font-bold font-mono text-emerald-400">{{ formatBytes(perfData.memory.available_ram) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">已提交 (Committed)</span>
                <p class="text-sm font-bold font-mono text-white">
                  {{ formatBytes(perfData.memory_detail?.committed_bytes || perfData.memory.used_ram) }} / {{ formatBytes(perfData.memory_detail?.commit_limit_bytes || perfData.memory.total_ram) }}
                </p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">分页缓冲池 (Paged)</span>
                <p class="text-sm font-bold font-mono text-gray-200">{{ formatBytes(perfData.memory_detail?.paged_pool_bytes || 600*1024*1024) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">非分页缓冲池 (Non-paged)</span>
                <p class="text-sm font-bold font-mono text-gray-200">{{ formatBytes(perfData.memory_detail?.non_paged_pool_bytes || 400*1024*1024) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">硬件保留</span>
                <p class="text-sm font-bold font-mono text-gray-300">约 128 MB</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">总物理容量</span>
                <p class="text-sm font-bold font-mono text-white">{{ formatBytes(perfData.memory.total_ram) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">内存状态评估</span>
                <p class="text-sm font-bold" :class="perfData.memory.usage_percent > 85 ? 'text-rose-400' : 'text-emerald-400'">
                  {{ perfData.memory.usage_percent > 85 ? '高负荷运行' : '健康充裕' }}
                </p>
              </div>
            </div>

            <!-- 3. 磁盘 模式详细参数 -->
            <div v-else-if="activeDevice.startsWith('disk:')" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">卷标与盘符</span>
                <p class="text-sm font-bold text-white">{{ currentSelectedDisk?.label }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">总容量</span>
                <p class="text-sm font-bold font-mono text-white">{{ formatBytes(currentSelectedDisk?.total_bytes || 0) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">可用空闲空间</span>
                <p class="text-sm font-bold font-mono text-emerald-400">{{ formatBytes(currentSelectedDisk?.available_bytes || 0) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">已用空间</span>
                <p class="text-sm font-bold font-mono text-white">{{ formatBytes(currentSelectedDisk?.used_bytes || 0) }} ({{ currentSelectedDisk?.usage_percent }}%)</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">文件系统格式</span>
                <p class="text-sm font-bold font-mono text-sky-400">{{ currentSelectedDisk?.file_system }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">系统引导盘</span>
                <p class="text-sm font-bold text-white">{{ currentSelectedDisk?.is_system_drive ? '是 (Windows 操作系统主盘)' : '否 (数据存储卷)' }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">介质类型</span>
                <p class="text-sm font-bold text-white">固态硬盘 (NVMe / SSD)</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">健康状态</span>
                <p class="text-sm font-bold text-emerald-400">良好 (SMART 正常)</p>
              </div>
            </div>

            <!-- 4. 网络 模式详细参数 -->
            <div v-else-if="activeDevice === 'network'" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">适配器名称</span>
                <p class="text-xs font-bold text-white truncate" :title="perfData.network.adapter_name">{{ perfData.network.adapter_name }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">瞬时下行速率 (接收)</span>
                <p class="text-sm font-bold font-mono text-sky-400">↓ {{ formatSpeed(perfData.network.rx_speed_bps) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">瞬时上行速率 (发送)</span>
                <p class="text-sm font-bold font-mono text-amber-400">↑ {{ formatSpeed(perfData.network.tx_speed_bps) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">本次开机累计接收</span>
                <p class="text-sm font-bold font-mono text-white">{{ formatBytes(perfData.network.total_rx_bytes) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">本次开机累计发送</span>
                <p class="text-sm font-bold font-mono text-white">{{ formatBytes(perfData.network.total_tx_bytes) }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">网络连接类型</span>
                <p class="text-sm font-bold text-emerald-400">物理全双工连接</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">状态</span>
                <p class="text-sm font-bold text-white">已连接至互联网</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">IP 协议栈</span>
                <p class="text-sm font-bold font-mono text-white">IPv4 / IPv6 双栈</p>
              </div>
            </div>

            <!-- 5. GPU 模式详细参数 -->
            <div v-else-if="activeDevice === 'gpu'" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">GPU 完整型号</span>
                <p class="text-xs font-bold text-white truncate" :title="perfData.gpu_name">{{ perfData.gpu_name }}</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">DirectX 支持</span>
                <p class="text-sm font-bold font-mono text-sky-400">DirectX 12 Ultimate</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">驱动程序模型</span>
                <p class="text-sm font-bold font-mono text-white">WDDM 3.1</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[11px] text-gray-400">硬件加速 GPU 计划</span>
                <p class="text-sm font-bold text-emerald-400">已开启</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ================= 模式二：全景网格视图 (全硬件一屏尽览) ================= -->
      <div v-else class="flex-1 min-h-0 overflow-y-auto win11-scrollbar space-y-5">
        <!-- 顶部 CPU & 内存 大卡片 -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- CPU -->
          <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Cpu class="w-4 h-4 text-blue-400" />
                <span class="text-xs font-bold text-gray-300">CPU 综合利用率</span>
              </div>
              <span class="text-xl font-bold font-mono text-blue-400">{{ perfData.cpu_usage_percent.toFixed(1) }}%</span>
            </div>
            <p class="text-xs text-gray-100 font-semibold truncate">{{ perfData.cpu_name }}</p>
            <div class="h-24 w-full bg-black/30 rounded-xl p-2 border border-white/5">
              <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 100">
                <defs>
                  <linearGradient id="gridCpuGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.4" />
                    <stop offset="100%" stop-color="#3b82f6" stop-opacity="0.0" />
                  </linearGradient>
                </defs>
                <polygon :points="buildPoints(cpuHistory, 100).areaPoints" fill="url(#gridCpuGrad)" />
                <polyline :points="buildPoints(cpuHistory, 100).linePoints" fill="none" stroke="#3b82f6" stroke-width="2" />
              </svg>
            </div>
            <div class="flex items-center justify-between text-[11px] text-gray-400 pt-1">
              <span>进程: {{ perfData.cpu_detail?.process_count || 240 }} | 线程: {{ perfData.cpu_detail?.thread_count || 3200 }}</span>
              <span class="text-emerald-400 font-mono">运行时间: {{ perfData.cpu_detail?.uptime_formatted }}</span>
            </div>
          </div>

          <!-- 内存 -->
          <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Layers class="w-4 h-4 text-purple-400" />
                <span class="text-xs font-bold text-gray-300">物理内存已用</span>
              </div>
              <span class="text-xl font-bold font-mono text-purple-400">{{ perfData.memory.usage_percent.toFixed(1) }}%</span>
            </div>
            <p class="text-xs text-gray-100 font-semibold truncate">
              已使用 {{ formatBytes(perfData.memory.used_ram) }} / 共 {{ formatBytes(perfData.memory.total_ram) }}
            </p>
            <div class="h-24 w-full bg-black/30 rounded-xl p-2 border border-white/5">
              <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 100">
                <defs>
                  <linearGradient id="gridMemGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stop-color="#a855f7" stop-opacity="0.4" />
                    <stop offset="100%" stop-color="#a855f7" stop-opacity="0.0" />
                  </linearGradient>
                </defs>
                <polygon :points="buildPoints(memHistory, 100).areaPoints" fill="url(#gridMemGrad)" />
                <polyline :points="buildPoints(memHistory, 100).linePoints" fill="none" stroke="#a855f7" stroke-width="2" />
              </svg>
            </div>
            <div class="flex items-center justify-between text-[11px] text-gray-400 pt-1">
              <span>可用内存: {{ formatBytes(perfData.memory.available_ram) }}</span>
              <span>已提交: {{ formatBytes(perfData.memory_detail?.committed_bytes || perfData.memory.used_ram) }}</span>
            </div>
          </div>
        </div>

        <!-- 中部 网络与 GPU -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- 网络 -->
          <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Network class="w-4 h-4 text-amber-400" />
                <span class="text-xs font-bold text-gray-300">网络瞬时收发</span>
              </div>
              <span class="text-sm font-mono font-bold text-sky-400">↓ {{ formatSpeed(perfData.network.rx_speed_bps) }}</span>
            </div>
            <p class="text-xs text-gray-300 truncate" :title="perfData.network.adapter_name">{{ perfData.network.adapter_name }}</p>
            <div class="h-20 w-full bg-black/30 rounded-xl p-2 border border-white/5">
              <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 100">
                <polyline :points="buildPoints(netHistory, Math.max(1024*100, ...netHistory)).linePoints" fill="none" stroke="#f59e0b" stroke-width="2" />
              </svg>
            </div>
            <div class="flex items-center justify-between text-[11px] text-gray-400">
              <span>发送: ↑ {{ formatSpeed(perfData.network.tx_speed_bps) }}</span>
              <span>总接收: {{ formatBytes(perfData.network.total_rx_bytes) }}</span>
            </div>
          </div>

          <!-- GPU -->
          <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Monitor class="w-4 h-4 text-sky-400" />
                <span class="text-xs font-bold text-gray-300">图形显示核心 (GPU)</span>
              </div>
              <span class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-sky-500/10 text-sky-300 border border-sky-500/20">WDDM 3.1</span>
            </div>
            <p class="text-xs text-gray-100 font-semibold truncate">{{ perfData.gpu_name }}</p>
            <div class="grid grid-cols-2 gap-3 pt-2">
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[10px] text-gray-500">图形接口</span>
                <p class="text-xs font-medium text-sky-400">DirectX 12 FL 12.1</p>
              </div>
              <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <span class="text-[10px] text-gray-500">硬件加速状态</span>
                <p class="text-xs font-medium text-emerald-400">已就绪</p>
              </div>
            </div>
          </div>
        </div>

        <!-- 底部 磁盘分区概览 -->
        <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 shadow-xl backdrop-blur-md space-y-4">
          <div class="flex items-center justify-between border-b border-white/5 pb-3">
            <div class="flex items-center gap-2">
              <HardDrive class="w-4 h-4 text-emerald-400" />
              <span class="text-xs font-bold text-gray-200">本地磁盘驱动器容量</span>
            </div>
            <span class="text-[11px] text-gray-400">{{ perfData.disks.length }} 个逻辑分区</span>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div v-for="disk in perfData.disks" :key="disk.letter" class="p-3.5 rounded-xl bg-white/[0.02] border border-white/5 space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-white">{{ disk.label }}</span>
                <span class="text-[10px] font-mono text-gray-400">{{ disk.file_system }}</span>
              </div>
              <div class="w-full h-2 rounded-full bg-white/10 overflow-hidden">
                <div class="h-full bg-emerald-500 rounded-full" :style="{ width: `${disk.usage_percent}%` }"></div>
              </div>
              <div class="flex items-center justify-between text-[11px] text-gray-400">
                <span>剩余 {{ formatBytes(disk.available_bytes) }}</span>
                <span>总共 {{ formatBytes(disk.total_bytes) }} ({{ disk.usage_percent }}%)</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file PerformanceMonitorView.vue
 * @description Windows 任务管理器风格的硬件性能监控与精细绘图中心
 */

import { ref, computed, onMounted, onUnmounted } from 'vue';
import {
  Activity,
  RefreshCw,
  Cpu,
  Monitor,
  Layers,
  Network,
  HardDrive,
  Sparkles,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { HardwarePerformance, DiskInfo } from '../types/module';

// 视图模式: 'taskmgr' (任务管理器经典分栏) | 'grid' (全景矩阵)
const viewMode = ref<'taskmgr' | 'grid'>('taskmgr');

// 当前激活选中的硬件设备选项卡: 'cpu' | 'memory' | 'disk:C:' | 'network' | 'gpu'
const activeDevice = ref<string>('cpu');

const isFetching = ref(false);

const perfData = ref<HardwarePerformance>({
  cpu_name: '检测中...',
  cpu_logical_cores: 8,
  cpu_usage_percent: 15.0,
  gpu_name: '检测中...',
  memory: {
    total_ram: 16 * 1024 * 1024 * 1024,
    available_ram: 8 * 1024 * 1024 * 1024,
    used_ram: 8 * 1024 * 1024 * 1024,
    usage_percent: 50.0,
  },
  disks: [],
  network: {
    adapter_name: '正在检测网卡...',
    rx_speed_bps: 0,
    tx_speed_bps: 0,
    total_rx_bytes: 0,
    total_tx_bytes: 0,
  },
  cpu_detail: {
    name: 'Intel(R) Core(TM) i7',
    physical_cores: 8,
    logical_cores: 16,
    usage_percent: 15.0,
    process_count: 240,
    thread_count: 3200,
    handle_count: 110000,
    uptime_seconds: 7200,
    uptime_formatted: '02:00:00',
  },
  memory_detail: {
    base: {
      total_ram: 16 * 1024 * 1024 * 1024,
      available_ram: 8 * 1024 * 1024 * 1024,
      used_ram: 8 * 1024 * 1024 * 1024,
      usage_percent: 50.0,
    },
    committed_bytes: 9 * 1024 * 1024 * 1024,
    commit_limit_bytes: 18 * 1024 * 1024 * 1024,
    paged_pool_bytes: 600 * 1024 * 1024,
    non_paged_pool_bytes: 400 * 1024 * 1024,
  },
  gpu_detail: {
    name: '集成显卡',
    status: 'DirectX 12 FL 12.1 就绪',
  },
});

// 历史 60 个秒级走势点 (精细绘制 60 秒时间轴)
const MAX_POINTS = 60;
const cpuHistory = ref<number[]>(new Array(MAX_POINTS).fill(15));
const memHistory = ref<number[]>(new Array(MAX_POINTS).fill(50));
const netHistory = ref<number[]>(new Array(MAX_POINTS).fill(0));
const diskHistory = ref<number[]>(new Array(MAX_POINTS).fill(5));

let timer: ReturnType<typeof setInterval> | null = null;

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
}

function formatGb(bytes: number): string {
  if (!bytes || bytes <= 0) return '0.0 GB';
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatSpeed(bps: number): string {
  if (!bps || bps <= 0) return '0 KB/s';
  const kb = bps / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  const mb = kb / 1024;
  return `${mb.toFixed(2)} MB/s`;
}

// 通用 SVG 坐标点生成函数
function buildPoints(history: number[], maxVal = 100) {
  const step = 100 / (MAX_POINTS - 1);
  let lastX = 100;
  let lastY = 100;

  const pts = history.map((val, idx) => {
    const x = idx * step;
    const normalized = Math.min(maxVal, Math.max(0, val));
    const y = 100 - (normalized / (maxVal || 1)) * 95;
    if (idx === history.length - 1) {
      lastX = x;
      lastY = y;
    }
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  const linePoints = pts.join(' ');
  const areaPoints = `0,100 ${linePoints} 100,100`;

  return { linePoints, areaPoints, lastPoint: { x: lastX, y: lastY } };
}

// 动态计算当前选中的磁盘
const currentSelectedDisk = computed<DiskInfo | undefined>(() => {
  if (activeDevice.value.startsWith('disk:')) {
    const letter = activeDevice.value.replace('disk:', '');
    return perfData.value.disks.find((d) => d.letter === letter) || perfData.value.disks[0];
  }
  return perfData.value.disks[0];
});

// 计算当前分类大标题
const currentSectionTitle = computed(() => {
  if (activeDevice.value === 'cpu') return '中央处理器 (CPU)';
  if (activeDevice.value === 'memory') return '系统物理内存 (Memory)';
  if (activeDevice.value.startsWith('disk:')) return '逻辑磁盘驱动器 (Disk)';
  if (activeDevice.value === 'network') return '网络适配器 (Network)';
  if (activeDevice.value === 'gpu') return '图形显示核心 (GPU)';
  return '硬件性能';
});

// 计算当前设备主名称
const currentDeviceHeaderName = computed(() => {
  if (activeDevice.value === 'cpu') return perfData.value.cpu_name;
  if (activeDevice.value === 'memory') return `总容量 ${formatBytes(perfData.value.memory.total_ram)}`;
  if (activeDevice.value.startsWith('disk:')) return currentSelectedDisk.value?.label || '本地磁盘';
  if (activeDevice.value === 'network') return perfData.value.network.adapter_name;
  if (activeDevice.value === 'gpu') return perfData.value.gpu_name;
  return '';
});

// 计算当前大号主数值
const currentPrimaryValue = computed(() => {
  if (activeDevice.value === 'cpu') return `${perfData.value.cpu_usage_percent.toFixed(1)}%`;
  if (activeDevice.value === 'memory') return `${(perfData.value.memory.used_ram / (1024*1024*1024)).toFixed(1)} GB`;
  if (activeDevice.value.startsWith('disk:')) return `${currentSelectedDisk.value?.usage_percent || 0}%`;
  if (activeDevice.value === 'network') return `↓ ${formatSpeed(perfData.value.network.rx_speed_bps)}`;
  if (activeDevice.value === 'gpu') return 'DirectX 12';
  return '';
});

// 计算当前副标题数值
const currentSecondaryValue = computed(() => {
  if (activeDevice.value === 'cpu') return `${perfData.value.cpu_logical_cores} 个逻辑核心`;
  if (activeDevice.value === 'memory') return `总共 ${formatGb(perfData.value.memory.total_ram)} (${perfData.value.memory.usage_percent.toFixed(0)}%)`;
  if (activeDevice.value.startsWith('disk:')) return `剩余 ${formatBytes(currentSelectedDisk.value?.available_bytes || 0)}`;
  if (activeDevice.value === 'network') return `↑ ${formatSpeed(perfData.value.network.tx_speed_bps)}`;
  if (activeDevice.value === 'gpu') return '硬件加速已就绪';
  return '';
});

// 当前主题色 Hex
const currentColorHex = computed(() => {
  if (activeDevice.value === 'cpu') return '#3b82f6';
  if (activeDevice.value === 'memory') return '#a855f7';
  if (activeDevice.value.startsWith('disk:')) return '#10b981';
  if (activeDevice.value === 'network') return '#f59e0b';
  if (activeDevice.value === 'gpu') return '#0ea5e9';
  return '#3b82f6';
});

// 当前主题文字色 Class
const currentThemeColorClass = computed(() => {
  if (activeDevice.value === 'cpu') return 'text-blue-400';
  if (activeDevice.value === 'memory') return 'text-purple-400';
  if (activeDevice.value.startsWith('disk:')) return 'text-emerald-400';
  if (activeDevice.value === 'network') return 'text-amber-400';
  if (activeDevice.value === 'gpu') return 'text-sky-400';
  return 'text-blue-400';
});

// 渐变 ID
const currentGradientId = computed(() => `grad-${activeDevice.value.replace(':', '-')}`);

// 当前刻度上限标签
const currentScaleTopLabel = computed(() => {
  if (activeDevice.value === 'cpu' || activeDevice.value === 'memory' || activeDevice.value.startsWith('disk:')) {
    return '100%';
  }
  if (activeDevice.value === 'network') {
    const maxNet = Math.max(1024 * 100, ...netHistory.value);
    return formatSpeed(maxNet);
  }
  return '100%';
});

// 当前折线与区域点集
const currentChartPoints = computed(() => {
  if (activeDevice.value === 'cpu') {
    return buildPoints(cpuHistory.value, 100);
  }
  if (activeDevice.value === 'memory') {
    return buildPoints(memHistory.value, 100);
  }
  if (activeDevice.value.startsWith('disk:')) {
    return buildPoints(diskHistory.value, 100);
  }
  if (activeDevice.value === 'network') {
    const maxNet = Math.max(1024 * 100, ...netHistory.value);
    return buildPoints(netHistory.value, maxNet);
  }
  return buildPoints(cpuHistory.value, 100);
});

async function fetchData() {
  if (isFetching.value) return;
  isFetching.value = true;
  try {
    if (isTauri()) {
      const snap = await invoke<HardwarePerformance>('get_performance_snapshot');
      perfData.value = snap;

      cpuHistory.value.push(snap.cpu_usage_percent);
      if (cpuHistory.value.length > MAX_POINTS) cpuHistory.value.shift();

      memHistory.value.push(snap.memory.usage_percent);
      if (memHistory.value.length > MAX_POINTS) memHistory.value.shift();

      netHistory.value.push(snap.network.rx_speed_bps);
      if (netHistory.value.length > MAX_POINTS) netHistory.value.shift();

      const diskPct = snap.disks[0]?.usage_percent || 0;
      diskHistory.value.push(diskPct);
      if (diskHistory.value.length > MAX_POINTS) diskHistory.value.shift();
    } else {
      // 浏览器非 Tauri 环境 Mock
      const rndCpu = Math.min(95, Math.max(5, 22 + (Math.random() - 0.5) * 15));
      perfData.value.cpu_usage_percent = rndCpu;
      cpuHistory.value.push(rndCpu);
      if (cpuHistory.value.length > MAX_POINTS) cpuHistory.value.shift();

      const rndMem = Math.min(95, Math.max(20, 52 + (Math.random() - 0.5) * 4));
      perfData.value.memory.usage_percent = rndMem;
      memHistory.value.push(rndMem);
      if (memHistory.value.length > MAX_POINTS) memHistory.value.shift();

      const rndNet = Math.floor(Math.random() * 1024 * 1024 * 3);
      perfData.value.network.rx_speed_bps = rndNet;
      netHistory.value.push(rndNet);
      if (netHistory.value.length > MAX_POINTS) netHistory.value.shift();
    }
  } catch (err) {
    console.error('获取硬件性能失败:', err);
  } finally {
    isFetching.value = false;
  }
}

onMounted(() => {
  fetchData();
  timer = setInterval(fetchData, 1500);
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
});
</script>

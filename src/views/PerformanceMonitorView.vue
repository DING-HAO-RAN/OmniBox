<template>
  <!-- 深度对标 Windows 任务管理器与 AIDA64 的全能硬件性能与系统全景中心 -->
  <div class="h-full flex flex-col min-h-0 bg-transparent text-white overflow-hidden select-none">
    <!-- 顶部工作台标题栏 -->
    <header class="flex-shrink-0 px-8 pt-5 pb-3 border-b border-white/5 bg-white/[0.01]">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-sky-500/20 text-sky-400 border border-sky-500/30 flex items-center justify-center">
              <Activity class="w-4 h-4" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-white">系统性能与硬件全景监控中心</h1>
          </div>
          <p class="text-xs text-gray-400 mt-0.5">
            原生采集 CPU 指令集与缓存、多 GPU 显存、物理 DIMM、NVMe SMART 寿命、网络 Wi-Fi、外设、安全与诊断日志。
          </p>
        </div>

        <!-- 顶部主标签页切换 -->
        <div class="flex items-center gap-2.5">
          <div class="inline-flex p-1 rounded-xl bg-black/40 border border-white/10 text-xs">
            <button
              v-for="tab in mainTabs"
              :key="tab.id"
              @click="activeMainTab = tab.id"
              type="button"
              class="px-3 py-1 rounded-lg font-medium transition-all flex items-center gap-1.5"
              :class="activeMainTab === tab.id ? 'bg-blue-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'"
            >
              <component :is="tab.icon" class="w-3.5 h-3.5" />
              <span>{{ tab.title }}</span>
            </button>
          </div>

          <button
            @click="handleManualRefresh"
            type="button"
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium text-gray-200 bg-white/[0.05] hover:bg-white/[0.1] border border-white/10 transition-all"
            title="刷新系统数据"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isFetching }" />
            <span>刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- 主展示区 -->
    <main class="flex-1 min-h-0 overflow-hidden px-8 py-5 flex flex-col">
      <!-- ================= 标签 1：实时性能走势 (对标任务管理器 60s 走势图) ================= -->
      <div v-if="activeMainTab === 'charts'" class="flex-1 min-h-0 flex flex-col">
        <!-- 顶部子模式切换 -->
        <div class="flex items-center justify-between pb-3 mb-1 border-b border-white/5">
          <span class="text-xs text-gray-400">实时采样周期: 1.5s · 保留 60s 历史窗口</span>
          <div class="inline-flex p-0.5 rounded-lg bg-white/[0.03] border border-white/10 text-[11px]">
            <button
              @click="chartSubMode = 'taskmgr'"
              type="button"
              class="px-2.5 py-0.5 rounded font-medium transition-all"
              :class="chartSubMode === 'taskmgr' ? 'bg-blue-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'"
            >
              分栏精细模式
            </button>
            <button
              @click="chartSubMode = 'grid'"
              type="button"
              class="px-2.5 py-0.5 rounded font-medium transition-all"
              :class="chartSubMode === 'grid' ? 'bg-blue-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200'"
            >
              全景网格视图
            </button>
          </div>
        </div>

        <!-- 分栏精细模式 -->
        <div v-if="chartSubMode === 'taskmgr'" class="flex-1 min-h-0 flex flex-col lg:flex-row gap-5 overflow-hidden">
          <!-- 左侧：硬件卡片列表 -->
          <div class="w-full lg:w-72 flex-shrink-0 flex flex-col gap-2 overflow-y-auto win11-scrollbar pr-1">
            <!-- CPU -->
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
                  <span>CPU 处理器</span>
                </div>
                <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">{{ formatMetricWithUnit(perfData.cpu.total_usage_percent, '%', 0) }}</p>
                <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]">{{ metricText(fullReport?.cpu_static.name) }}</p>
              </div>
              <div class="w-20 h-10 flex-shrink-0">
                <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 40">
                  <polyline
                     v-for="(points, index) in buildPoints(cpuHistory, 100).lineSegments"
                     :key="`cpu-line-${index}`"
                     :points="points"
                     fill="none"
                     stroke="#3b82f6"
                     stroke-width="1.5"
                   />
                </svg>
              </div>
            </button>

            <!-- 内存 -->
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
                  <span>物理内存 (RAM)</span>
                </div>
                <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">
                  {{ formatGb(metricNumber(perfData.memory.used_physical_bytes)) }}/{{ formatGb(metricNumber(perfData.memory.total_physical_bytes)) }} ({{ formatMetricWithUnit(perfData.memory.usage_percent, '%', 0) }})
                </p>
                <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]">已用 {{ formatGb(metricNumber(perfData.memory.used_physical_bytes)) }}</p>
              </div>
              <div class="w-20 h-10 flex-shrink-0">
                <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 40">
                  <polyline
                     v-for="(points, index) in buildPoints(memHistory, 100).lineSegments"
                     :key="`memory-line-${index}`"
                     :points="points"
                     fill="none"
                     stroke="#a855f7"
                     stroke-width="1.5"
                   />
                </svg>
              </div>
            </button>

            <!-- 磁盘 -->
            <button
              v-for="disk in perfData.disks"
              :key="disk.id"
              @click="activeDevice = 'disk:' + disk.id"
              type="button"
              class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
              :class="activeDevice === 'disk:' + disk.id ? 'bg-emerald-600/15 border-emerald-500/50 shadow-lg shadow-emerald-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
            >
              <span v-if="activeDevice === 'disk:' + disk.id" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.8)]"></span>
              <div class="min-w-0">
                <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'disk:' + disk.id ? 'text-emerald-400' : 'text-gray-300'">
                  <HardDrive class="w-3.5 h-3.5" />
                  <span>磁盘 ({{ metricText(disk.drive_letter) }})</span>
                </div>
                <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">{{ formatMetricWithUnit(disk.usage_percent, '%') }} 已用</p>
                <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]">{{ metricText(disk.label) }} ({{ metricText(disk.file_system) }})</p>
              </div>
              <div class="w-16 flex flex-col items-end gap-1 flex-shrink-0">
                <span class="text-[9px] font-mono text-gray-400">{{ formatBytes(metricNumber(disk.available_bytes)) }} 余</span>
                <div class="w-full h-1.5 rounded-full bg-white/10 overflow-hidden">
                  <div class="h-full bg-emerald-500 rounded-full" :style="{ width: metricPercentWidth(disk.usage_percent) }"></div>
                </div>
              </div>
            </button>

            <!-- 网络适配器：每个真实适配器使用稳定 id 展示 -->
            <button
              v-for="adapter in perfData.network"
              :key="adapter.id"
              @click="activeDevice = 'network:' + adapter.id"
              type="button"
              class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
              :class="activeDevice === 'network:' + adapter.id ? 'bg-amber-600/15 border-amber-500/50 shadow-lg shadow-amber-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
            >
              <span v-if="activeDevice === 'network:' + adapter.id" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-amber-500"></span>
              <div class="min-w-0">
                <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'network:' + adapter.id ? 'text-amber-400' : 'text-gray-300'">
                  <Network class="w-3.5 h-3.5" />
                  <span>{{ metricText(adapter.name) }}</span>
                </div>
                <p class="text-[11px] font-mono text-gray-400 mt-1 font-semibold">↓ {{ formatSpeed(adapter.rx_bytes_per_sec) }}</p>
                <p class="text-[10px] text-gray-500 truncate mt-0.5 max-w-[130px]">↑ {{ formatSpeed(adapter.tx_bytes_per_sec) }}</p>
              </div>
            </button>

            <!-- GPU：多 GPU 逐项展示稳定 id -->
            <button
              v-for="gpu in perfData.gpus"
              :key="gpu.id"
              @click="activeDevice = 'gpu:' + gpu.id"
              type="button"
              class="w-full text-left rounded-2xl p-3.5 border transition-all flex items-center justify-between gap-3 relative group"
              :class="activeDevice === 'gpu:' + gpu.id ? 'bg-sky-600/15 border-sky-500/50 shadow-lg shadow-sky-500/10' : 'bg-white/[0.025] hover:bg-white/[0.05] border-white/10'"
            >
              <span v-if="activeDevice === 'gpu:' + gpu.id" class="absolute left-1 top-1/2 -translate-y-1/2 h-5 w-1 rounded-full bg-sky-500"></span>
              <div class="min-w-0">
                <div class="flex items-center gap-1.5 text-xs font-bold" :class="activeDevice === 'gpu:' + gpu.id ? 'text-sky-400' : 'text-gray-300'">
                  <Monitor class="w-3.5 h-3.5" />
                  <span>GPU 图形卡</span>
                </div>
                <p class="text-[11px] text-gray-300 mt-1 font-semibold truncate max-w-[140px]">{{ metricText(gpu.name) }}</p>
                <p class="text-[10px] text-gray-500 truncate mt-0.5">{{ formatMetricWithUnit(gpu.utilization_percent, '%', 0) }} 利用率</p>
              </div>
              <Sparkles class="w-4 h-4 text-sky-400 flex-shrink-0" />
            </button>
          </div>

          <!-- 右侧：选中的硬件详细精细图表与参数面板 -->
          <div class="flex-1 min-h-0 overflow-y-auto win11-scrollbar rounded-3xl p-6 bg-white/[0.025] border border-white/10 shadow-2xl backdrop-blur-md flex flex-col justify-between space-y-5">
            <!-- 头部状态 -->
            <div class="flex items-start justify-between border-b border-white/5 pb-4">
              <div>
                <div class="flex items-center gap-2">
                  <span class="text-xs font-bold uppercase tracking-wider text-gray-400">{{ currentSectionTitle }}</span>
                  <span class="px-2 py-0.5 rounded-full text-[10px] font-mono bg-blue-500/10 text-blue-300 border border-blue-500/20">
                    实时 60 秒平滑走势
                  </span>
                </div>
                <h2 class="text-xl font-bold text-white mt-1">{{ currentDeviceHeaderName }}</h2>
              </div>
              <div class="text-right">
                <span class="text-2xl lg:text-3xl font-bold font-mono" :class="currentThemeColorClass">{{ currentPrimaryValue }}</span>
                <p class="text-xs text-gray-400 mt-0.5">{{ currentSecondaryValue }}</p>
              </div>
            </div>

            <!-- 精细网格走势图 -->
            <div class="relative w-full h-44 lg:h-52 bg-black/40 rounded-2xl border border-white/10 p-3 overflow-hidden">
              <div class="absolute inset-0 grid grid-rows-4 grid-cols-6 pointer-events-none opacity-20">
                <div v-for="n in 24" :key="n" class="border-b border-r border-white/50"></div>
              </div>
              <div class="absolute right-2 top-2 text-[10px] font-mono text-gray-500 select-none">{{ currentScaleTopLabel }}</div>
              <div class="absolute right-2 bottom-2 text-[10px] font-mono text-gray-500 select-none">0</div>
              <div class="absolute left-2 bottom-2 text-[10px] font-mono text-gray-500 select-none">60 秒</div>

              <svg class="w-full h-full overflow-visible relative z-10" preserveAspectRatio="none" viewBox="0 0 100 100">
                <defs>
                  <linearGradient :id="currentGradientId" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" :stop-color="currentColorHex" stop-opacity="0.45" />
                    <stop offset="100%" :stop-color="currentColorHex" stop-opacity="0.0" />
                  </linearGradient>
                </defs>
                <polygon
                  v-for="(points, index) in currentChartPoints.areaSegments"
                  :key="`area-${index}`"
                  :points="points"
                  :fill="`url(#${currentGradientId})`"
                />
                <polyline
                  v-for="(points, index) in currentChartPoints.lineSegments"
                  :key="`line-${index}`"
                  :points="points"
                  fill="none"
                  :stroke="currentColorHex"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <circle
                  v-if="currentChartPoints.lastPoint !== null"
                  :cx="currentChartPoints.lastPoint.x"
                  :cy="currentChartPoints.lastPoint.y"
                  r="3.5"
                  :fill="currentColorHex"
                  class="animate-pulse"
                />
              </svg>
            </div>

            <!-- 底部关键参数网格 -->
            <div class="pt-4 border-t border-white/5">
              <!-- CPU 参数 -->
              <div v-if="activeDevice === 'cpu'" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">利用率</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatMetricWithUnit(perfData.cpu.total_usage_percent, '%') }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">物理核 / 线程数</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatMetricNumber(fullReport?.cpu_static.physical_cores, 0) }} / {{ formatMetricNumber(fullReport?.cpu_static.logical_processors, 0) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">活跃进程数</span>
                  <p class="text-sm font-bold font-mono text-white">{{ fullReport?.processes?.total_processes ?? '—' }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">运行时间 (Uptime)</span>
                  <p class="text-sm font-bold font-mono text-emerald-400">{{ metricText(fullReport?.computer.uptime_formatted) }}</p>
                </div>
              </div>

              <!-- 内存参数 -->
              <div v-else-if="activeDevice === 'memory'" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">已用物理内存</span>
                  <p class="text-sm font-bold font-mono text-purple-300">{{ formatBytes(metricNumber(perfData.memory.used_physical_bytes)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">可用空闲内存</span>
                  <p class="text-sm font-bold font-mono text-emerald-400">{{ formatBytes(metricNumber(perfData.memory.available_physical_bytes)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">已提交 (Committed)</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatBytes(metricNumber(perfData.memory.committed_bytes)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">总物理容量</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatBytes(metricNumber(perfData.memory.total_physical_bytes)) }}</p>
                </div>
              </div>

              <!-- 磁盘参数 -->
              <div v-else-if="activeDevice.startsWith('disk:')" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">总容量</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatBytes(metricNumber(currentSelectedDisk?.total_bytes)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">可用剩余</span>
                  <p class="text-sm font-bold font-mono text-emerald-400">{{ formatBytes(metricNumber(currentSelectedDisk?.available_bytes)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">文件系统</span>
                  <p class="text-sm font-bold font-mono text-sky-400">{{ metricText(currentSelectedDisk?.file_system) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">引导盘</span>
                  <p class="text-sm font-bold text-white">—</p>
                </div>
              </div>

              <!-- 网络参数 -->
              <div v-else-if="activeDevice.startsWith('network:')" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">瞬时下行速率</span>
                  <p class="text-sm font-bold font-mono text-sky-400">↓ {{ formatSpeed(currentSelectedNetwork?.rx_bytes_per_sec) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">瞬时上行速率</span>
                  <p class="text-sm font-bold font-mono text-amber-400">↑ {{ formatSpeed(currentSelectedNetwork?.tx_bytes_per_sec) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">累计接收流量</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatBytes(metricNumber(currentSelectedNetwork?.rx_bytes_total)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">累计发送流量</span>
                  <p class="text-sm font-bold font-mono text-white">{{ formatBytes(metricNumber(currentSelectedNetwork?.tx_bytes_total)) }}</p>
                </div>
              </div>

              <!-- GPU 参数 -->
              <div v-else-if="activeDevice.startsWith('gpu:')" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">图形接口</span>
                  <p class="text-sm font-bold text-sky-400">—</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">驱动架构</span>
                  <p class="text-sm font-bold text-white">{{ metricText(currentSelectedGpu?.driver_version) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">显存机制</span>
                  <p class="text-sm font-bold text-emerald-400">独立 {{ formatBytes(metricNumber(currentSelectedGpu?.dedicated_vram_bytes)) }} / 共享 {{ formatBytes(metricNumber(currentSelectedGpu?.shared_vram_bytes)) }}</p>
                </div>
                <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                  <span class="text-[11px] text-gray-400">硬件加速</span>
                  <p class="text-sm font-bold text-white">{{ formatMetricWithUnit(currentSelectedGpu?.utilization_percent, '%', 0) }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 全景网格视图 -->
        <div v-else class="flex-1 min-h-0 overflow-y-auto win11-scrollbar space-y-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- CPU -->
            <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-gray-300">CPU 处理器</span>
                <span class="text-lg font-bold font-mono text-blue-400">{{ formatMetricWithUnit(perfData.cpu.total_usage_percent, '%') }}</span>
              </div>
              <p class="text-xs text-white font-semibold truncate">{{ metricText(fullReport?.cpu_static.name) }}</p>
              <div class="h-20 w-full bg-black/30 rounded-xl p-2 border border-white/5">
                <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 100">
                  <polyline
                     v-for="(points, index) in buildPoints(cpuHistory, 100).lineSegments"
                     :key="`cpu-grid-line-${index}`"
                     :points="points"
                     fill="none"
                     stroke="#3b82f6"
                     stroke-width="2"
                   />
                </svg>
              </div>
            </div>

            <!-- 内存 -->
            <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-gray-300">物理内存</span>
                <span class="text-lg font-bold font-mono text-purple-400">{{ formatMetricWithUnit(perfData.memory.usage_percent, '%') }}</span>
              </div>
              <p class="text-xs text-white font-semibold truncate">{{ formatBytes(metricNumber(perfData.memory.used_physical_bytes)) }} / {{ formatBytes(metricNumber(perfData.memory.total_physical_bytes)) }}</p>
              <div class="h-20 w-full bg-black/30 rounded-xl p-2 border border-white/5">
                <svg class="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 100">
                  <polyline
                     v-for="(points, index) in buildPoints(memHistory, 100).lineSegments"
                     :key="`memory-grid-line-${index}`"
                     :points="points"
                     fill="none"
                     stroke="#a855f7"
                     stroke-width="2"
                   />
                </svg>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ================= 标签 2：硬件全景规格 (CPUID、NVMe SMART、DIMM插槽、外设等) ================= -->
      <div v-else-if="activeMainTab === 'specs'" class="flex-1 min-h-0 overflow-y-auto win11-scrollbar space-y-5 pr-1">
        <!-- 1. 处理器 CPUID 与缓存拓扑 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-4">
          <div class="flex items-center justify-between border-b border-white/5 pb-2.5">
            <div class="flex items-center gap-2">
              <Cpu class="w-4 h-4 text-blue-400" />
              <h3 class="text-sm font-bold text-white">处理器指令集架构与拓扑 (CPUID)</h3>
            </div>
            <span class="text-xs font-mono text-blue-400" :title="metricMeta(fullReport?.cpu_static?.name)">{{ metricText(fullReport?.cpu_static?.name) }}</span>
          </div>

          <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">厂商 / 微架构</span>
              <p class="font-bold text-white mt-0.5" :title="metricMeta(fullReport?.cpu_static?.vendor)">{{ metricText(fullReport?.cpu_static?.vendor) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">家族 / 型号 / 步进</span>
              <p
                class="font-bold font-mono text-white mt-0.5"
                :title="`${metricMeta(fullReport?.cpu_static?.family)} | ${metricMeta(fullReport?.cpu_static?.model)} | ${metricMeta(fullReport?.cpu_static?.stepping)}`"
              >Family {{ formatMetricNumber(fullReport?.cpu_static?.family, 0) }} · Model {{ formatMetricNumber(fullReport?.cpu_static?.model, 0) }} · Stepping {{ formatMetricNumber(fullReport?.cpu_static?.stepping, 0) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">L1 数据 / 指令缓存</span>
              <p
                class="font-bold font-mono text-white mt-0.5"
                :title="`${metricMeta(fullReport?.cpu_static?.l1_data_cache_kb)} | ${metricMeta(fullReport?.cpu_static?.l1_inst_cache_kb)}`"
              >{{ formatMetricWithUnit(fullReport?.cpu_static?.l1_data_cache_kb, ' KB', 0) }} / {{ formatMetricWithUnit(fullReport?.cpu_static?.l1_inst_cache_kb, ' KB', 0) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">L2 / L3 三级缓存</span>
              <p
                class="font-bold font-mono text-white mt-0.5"
                :title="`${metricMeta(fullReport?.cpu_static?.l2_cache_kb)} | ${metricMeta(fullReport?.cpu_static?.l3_cache_kb)}`"
              >{{ formatMetricMb(fullReport?.cpu_static?.l2_cache_kb) }} / {{ formatMetricMb(fullReport?.cpu_static?.l3_cache_kb) }}</p>
            </div>
          </div>

          <!-- 指令集徽章 -->
          <div class="space-y-1.5 pt-1">
            <span class="text-[11px] text-gray-400">支持的硬件指令集扩展:</span>
            <div class="flex flex-wrap gap-1.5">
              <span
                v-for="feat in (fullReport?.cpu_static?.features ?? [])"
                :key="feat"
                class="px-2 py-0.5 rounded bg-blue-500/10 text-blue-300 border border-blue-500/20 text-[10px] font-mono font-medium"
              >
                {{ feat }}
              </span>
            </div>
          </div>
        </section>

        <!-- 2. 主板与 BIOS / UEFI -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-4">
          <div class="flex items-center justify-between border-b border-white/5 pb-2.5">
            <div class="flex items-center gap-2">
              <Layers class="w-4 h-4 text-purple-400" />
              <h3 class="text-sm font-bold text-white">主板与 BIOS / UEFI 固件</h3>
            </div>
            <span class="text-xs text-purple-300" :title="metricMeta(fullReport?.motherboard?.product)">{{ metricText(fullReport?.motherboard?.product) }}</span>
          </div>

          <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">主板制造商</span>
              <p class="font-bold text-white mt-0.5" :title="metricMeta(fullReport?.motherboard?.manufacturer)">{{ metricText(fullReport?.motherboard?.manufacturer) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">主板型号</span>
              <p class="font-bold text-white mt-0.5" :title="metricMeta(fullReport?.motherboard?.product)">{{ metricText(fullReport?.motherboard?.product) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">BIOS 厂商 & 版本</span>
              <p
                class="font-bold font-mono text-white mt-0.5"
                :title="`${metricMeta(fullReport?.bios?.vendor)} | ${metricMeta(fullReport?.bios?.version)}`"
              >{{ metricText(fullReport?.bios?.vendor) }} {{ metricText(fullReport?.bios?.version) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">固件引导模式</span>
              <p class="font-bold text-emerald-400 mt-0.5" :title="metricMeta(fullReport?.bios?.firmware_mode)">{{ metricText(fullReport?.bios?.firmware_mode) }}</p>
            </div>
          </div>
        </section>

        <!-- 3. NVMe 物理磁盘与 SMART 健康度 (寿命/TBW/温度) -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-4">
          <div class="flex items-center justify-between border-b border-white/5 pb-2.5">
            <div class="flex items-center gap-2">
              <HardDrive class="w-4 h-4 text-emerald-400" />
              <h3 class="text-sm font-bold text-white">NVMe / SATA 物理磁盘硬件与 SMART 健康度</h3>
            </div>
            <span class="text-xs text-emerald-400">共 {{ fullReportCount(fullReport?.storage?.physical_disks, 'storage') }} 块物理驱动器</span>
          </div>

          <div class="space-y-3">
            <div
              v-for="disk in (fullReport?.storage?.physical_disks ?? [])"
              :key="disk.id"
              class="p-4 rounded-xl bg-white/[0.02] border border-white/5 space-y-3"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span
                    class="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-300 border border-emerald-500/20 font-bold"
                    :title="metricMeta(disk.bus_type)"
                  >
                    {{ metricText(disk.bus_type) }}
                  </span>
                  <span class="text-xs font-bold text-white" :title="metricMeta(disk.model)">{{ metricText(disk.model) }}</span>
                </div>
                <span class="text-xs font-mono text-gray-400" :title="metricMeta(disk.serial_number)">SN: {{ metricText(disk.serial_number) }}</span>
              </div>

              <!-- SMART 健康度核心指标网格 -->
              <div v-if="disk.smart_health" class="grid grid-cols-2 sm:grid-cols-5 gap-2.5 text-xs pt-1">
                <div class="p-2.5 rounded-lg bg-black/30 border border-white/5">
                  <span class="text-[10px] text-gray-400">复合温度</span>
                  <p class="text-xs font-bold font-mono text-emerald-400" :title="metricMeta(disk.smart_health.temperature_c)">{{ formatMetricWithUnit(disk.smart_health.temperature_c, ' °C') }}</p>
                </div>
                <div class="p-2.5 rounded-lg bg-black/30 border border-white/5">
                  <span class="text-[10px] text-gray-400">寿命已用 (Percentage Used)</span>
                  <p class="text-xs font-bold font-mono text-white" :title="metricMeta(disk.smart_health.percentage_used)">{{ formatMetricWithUnit(disk.smart_health.percentage_used, '%') }}</p>
                </div>
                <div class="p-2.5 rounded-lg bg-black/30 border border-white/5">
                  <span class="text-[10px] text-gray-400">累计写入量 (TBW)</span>
                  <p class="text-xs font-bold font-mono text-sky-400" :title="metricMeta(disk.smart_health.data_units_written_tb)">{{ formatMetricWithUnit(disk.smart_health.data_units_written_tb, ' TB') }}</p>
                </div>
                <div class="p-2.5 rounded-lg bg-black/30 border border-white/5">
                  <span class="text-[10px] text-gray-400">通电时间</span>
                  <p class="text-xs font-bold font-mono text-white" :title="metricMeta(disk.smart_health.power_on_hours)">{{ formatMetricWithUnit(disk.smart_health.power_on_hours, ' 小时', 0) }}</p>
                </div>
                <div class="p-2.5 rounded-lg bg-black/30 border border-white/5">
                  <span class="text-[10px] text-gray-400">异常断电</span>
                  <p class="text-xs font-bold font-mono text-amber-400" :title="metricMeta(disk.smart_health.unsafe_shutdowns)">{{ formatMetricWithUnit(disk.smart_health.unsafe_shutdowns, ' 次', 0) }}</p>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 4. 物理内存条 DIMM 插槽分配 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-4">
          <div class="flex items-center justify-between border-b border-white/5 pb-2.5">
            <div class="flex items-center gap-2">
              <Layers class="w-4 h-4 text-purple-400" />
              <h3 class="text-sm font-bold text-white">物理内存条 (DIMM) 插槽与规格</h3>
            </div>
            <span class="text-xs text-purple-300">已插入 {{ fullReportCount(fullReport?.memory?.dimms, 'dimm') }} 根</span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div
              v-for="dimm in (fullReport?.memory?.dimms ?? [])"
              :key="dimm.slot"
              class="p-3.5 rounded-xl bg-white/[0.02] border border-white/5 space-y-2 text-xs"
            >
              <div class="flex items-center justify-between">
                <span class="font-bold text-white">{{ dimm.slot }}</span>
                <span class="px-2 py-0.5 rounded bg-purple-500/10 text-purple-300 border border-purple-500/20 font-mono text-[10px]">{{ dimm.memory_type }} - {{ dimm.speed_mhz }} MHz</span>
              </div>
              <div class="flex items-center justify-between text-gray-400 text-[11px]">
                <span>容量: {{ (dimm.capacity_bytes / (1024*1024*1024)).toFixed(0) }} GB</span>
                <span>电压: {{ dimm.configured_voltage }}V</span>
              </div>
              <p class="text-[10px] text-gray-500 truncate">型号: {{ dimm.part_number }} · {{ dimm.manufacturer }}</p>
            </div>
          </div>
        </section>

        <!-- 5. 显示器、音频与外设 -->
        <section class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- 显示器 -->
          <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-3">
            <div class="flex items-center justify-between border-b border-white/5 pb-2">
              <div class="flex items-center gap-2">
                <Monitor class="w-4 h-4 text-sky-400" />
                <h3 class="text-xs font-bold text-white">显示设备</h3>
              </div>
              <span class="text-[11px] text-gray-400">{{ fullReportCount(fullReport?.media?.displays, 'media') }} 台显示器</span>
            </div>
            <div v-for="disp in (fullReport?.media?.displays ?? [])" :key="disp.id" class="p-3 rounded-xl bg-white/[0.02] border border-white/5 text-xs space-y-1">
              <div class="flex items-center justify-between font-bold text-white">
                <span>{{ disp.friendly_name }}</span>
                <span class="text-sky-400 font-mono">{{ disp.width }} × {{ disp.height }} @ {{ disp.refresh_rate_hz }}Hz</span>
              </div>
              <div class="flex items-center justify-between text-[11px] text-gray-400">
                <span>位深: {{ disp.bits_per_pixel }} Bit · {{ disp.orientation }}</span>
                <span :class="disp.is_primary ? 'text-emerald-400' : 'text-gray-500'">{{ disp.is_primary ? '主显示器' : '辅助屏' }}</span>
              </div>
            </div>
          </div>

          <!-- 电池与电源 -->
          <div class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-3">
            <div class="flex items-center justify-between border-b border-white/5 pb-2">
              <div class="flex items-center gap-2">
                <BatteryCharging class="w-4 h-4 text-emerald-400" />
                <h3 class="text-xs font-bold text-white">电池与电源状态</h3>
              </div>
              <span class="text-[11px] text-emerald-400" :title="metricMeta(fullReport?.battery_power?.has_battery)">{{ metricBoolean(fullReport?.battery_power?.has_battery) === null ? '—' : metricBoolean(fullReport?.battery_power?.has_battery) ? '便携设备' : '台式机' }}</span>
            </div>
            <div class="p-3.5 rounded-xl bg-white/[0.02] border border-white/5 text-xs space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-gray-400">供电模式:</span>
                <span class="font-bold text-white" :title="metricMeta(fullReport?.battery_power?.charging_status)">{{ metricText(fullReport?.battery_power?.charging_status) }}</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-gray-400">当前电源计划:</span>
                <span class="font-bold text-emerald-400" :title="metricMeta(fullReport?.battery_power?.power_scheme)">{{ metricText(fullReport?.battery_power?.power_scheme) }}</span>
              </div>
            </div>
          </div>
        </section>
      </div>

      <!-- ================= 标签 3：系统环境与工具链 (OS、开发环境、安全防护) ================= -->
      <div v-else-if="activeMainTab === 'env'" class="flex-1 min-h-0 overflow-y-auto win11-scrollbar space-y-5 pr-1">
        <!-- Windows 详细版本与构建号 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-3">
          <h3 class="text-sm font-bold text-white flex items-center gap-2 border-b border-white/5 pb-2">
            <Layers class="w-4 h-4 text-blue-400" />
            <span>Windows 操作系统详细版本</span>
          </h3>
          <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">产品版本</span>
              <p class="font-bold text-white mt-0.5" :title="metricMeta(fullReport?.os?.name)">{{ metricText(fullReport?.os?.name) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">内部版本号 (Build.UBR)</span>
              <p class="font-bold font-mono text-white mt-0.5" :title="metricMeta(fullReport?.os?.build_number)">{{ metricText(fullReport?.os?.build_number) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">版本阶段 (Version)</span>
              <p class="font-bold font-mono text-sky-400 mt-0.5" :title="metricMeta(fullReport?.os?.display_version)">{{ metricText(fullReport?.os?.display_version) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">系统架构</span>
              <p class="font-bold text-white mt-0.5" :title="metricMeta(fullReport?.os?.architecture)">{{ metricText(fullReport?.os?.architecture) }}</p>
            </div>
          </div>
        </section>

        <!-- 开发者工具链检测 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-3">
          <h3 class="text-sm font-bold text-white flex items-center gap-2 border-b border-white/5 pb-2">
            <Terminal class="w-4 h-4 text-emerald-400" />
            <span>开发者工具链与运行环境探测</span>
          </h3>
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            <div
              v-for="tool in (fullReport?.dev_env?.tools ?? [])"
              :key="tool.name"
              class="p-3 rounded-xl bg-white/[0.02] border border-white/5 flex items-center justify-between text-xs"
            >
              <div class="min-w-0 pr-2">
                <span class="font-bold text-white block">{{ tool.name }}</span>
                <span class="text-[11px] text-gray-400 truncate block mt-0.5" :title="metricMeta(tool.version)">{{ metricText(tool.version) }}</span>
              </div>
              <span
                class="px-2 py-0.5 rounded text-[10px] font-medium flex-shrink-0"
                :class="metricBoolean(tool.installed) === true ? 'bg-emerald-500/10 text-emerald-300 border border-emerald-500/20' : 'bg-gray-500/10 text-gray-400 border border-gray-500/20'"
              >
                <span :title="metricMeta(tool.installed)">{{ metricBoolean(tool.installed) === null ? '—' : metricBoolean(tool.installed) ? '已就绪' : '未检测到' }}</span>
              </span>
            </div>
          </div>
        </section>

        <!-- 安全中心防护状态 -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-3">
          <h3 class="text-sm font-bold text-white flex items-center gap-2 border-b border-white/5 pb-2">
            <ShieldCheck class="w-4 h-4 text-amber-400" />
            <span>安全中心与硬件安全防护状态</span>
          </h3>
          <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">安全启动 (Secure Boot)</span>
              <p class="font-bold text-emerald-400 mt-0.5" :title="metricMeta(fullReport?.windows_env?.security_status?.secure_boot_enabled)">{{ metricBooleanText(fullReport?.windows_env?.security_status?.secure_boot_enabled) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">TPM 芯片规格</span>
              <p class="font-bold font-mono text-white mt-0.5" :title="metricMeta(fullReport?.windows_env?.security_status?.tpm_version)">{{ metricText(fullReport?.windows_env?.security_status?.tpm_version) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">Windows Defender 实时扫描</span>
              <p class="font-bold text-emerald-400 mt-0.5" :title="metricMeta(fullReport?.windows_env?.security_status?.defender_realtime_protection)">{{ metricBooleanText(fullReport?.windows_env?.security_status?.defender_realtime_protection) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">Windows 防火墙</span>
              <p class="font-bold text-emerald-400 mt-0.5" :title="metricMeta(fullReport?.windows_env?.security_status?.firewall_public_enabled)">{{ metricBooleanText(fullReport?.windows_env?.security_status?.firewall_public_enabled) }}</p>
            </div>
          </div>
        </section>
      </div>

      <!-- ================= 标签 4：诊断日志与一键导出 ================= -->
      <div v-else-if="activeMainTab === 'export'" class="flex-1 min-h-0 overflow-y-auto win11-scrollbar space-y-5 pr-1">
        <!-- 故障转储与 WHEA -->
        <section class="rounded-2xl p-5 bg-white/[0.025] border border-white/10 space-y-3">
          <h3 class="text-sm font-bold text-white flex items-center gap-2 border-b border-white/5 pb-2">
            <FileText class="w-4 h-4 text-sky-400" />
            <span>系统健康度与故障事件统计</span>
          </h3>
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">Minidump 崩溃转储文件</span>
              <p class="font-bold font-mono text-emerald-400 mt-0.5">{{ formatMetricWithUnit(fullReport?.diagnostics?.minidump_count, ' 个', 0) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">异常关机记录</span>
              <p class="font-bold font-mono text-white mt-0.5">{{ formatMetricWithUnit(fullReport?.diagnostics?.unexpected_shutdowns_count, ' 次', 0) }}</p>
            </div>
            <div class="p-3 rounded-xl bg-white/[0.02] border border-white/5">
              <span class="text-[11px] text-gray-400">总体健康评估</span>
              <p class="font-bold text-emerald-400 mt-0.5">{{ metricText(fullReport?.diagnostics?.overall_health_assessment) }}</p>
            </div>
          </div>
        </section>

        <!-- 一键导出诊断报告操作卡片 -->
        <section class="rounded-2xl p-6 bg-gradient-to-r from-blue-900/20 to-purple-900/20 border border-white/10 space-y-4">
          <div>
            <h3 class="text-sm font-bold text-white flex items-center gap-2">
              <Download class="w-4 h-4 text-blue-400" />
              <span>导出全维度系统与硬件诊断分析报告 (JSON)</span>
            </h3>
            <p class="text-xs text-gray-300 mt-1">
              一键生成结构化的计算机、硬件规格、NVMe 寿命、网络适配器与事件诊断快照，支持自动脱敏保护个人隐私。
            </p>
          </div>

          <div class="flex items-center gap-3">
            <label class="flex items-center gap-2 cursor-pointer text-xs text-gray-300">
              <input type="checkbox" v-model="sanitizeExport" class="rounded bg-black/40 border-white/20 accent-blue-500" />
              <span>自动脱敏隐私信息 (用户名、IP、MAC、序列号等)</span>
            </label>
          </div>

          <div class="flex items-center gap-3 pt-2">
            <button
              @click="handleCopyReport"
              type="button"
              class="px-4 py-2 rounded-xl text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 active:scale-95 transition-all flex items-center gap-1.5 shadow-lg shadow-blue-500/20"
            >
              <Copy class="w-3.5 h-3.5" />
              <span>复制诊断报告 JSON 到剪贴板</span>
            </button>

            <button
              @click="handleExportReport"
              type="button"
              class="px-4 py-2 rounded-xl text-xs font-semibold text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/30 active:scale-95 transition-all flex items-center gap-1.5"
            >
              <FileDown class="w-3.5 h-3.5" />
              <span>保存为本地诊断文件</span>
            </button>
          </div>
        </section>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
/**
 * @file PerformanceMonitorView.vue
 * @description Windows 任务管理器风格的硬件性能监控与系统全景诊断中心
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
  Terminal,
  ShieldCheck,
  BatteryCharging,
  FileText,
  Download,
  Copy,
  FileDown,
  LayoutGrid,
} from 'lucide-vue-next';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { HardwarePerformance, MetricValue, RuntimeDiskInfo, RuntimeNetworkInfo, GpuDevice, SystemFullReport, SystemMemoryInfo } from '../types/module';
import {
  appendMetricGap,
  appendMetricPoint,
  collectionCount,
  metricMeta,
  metricNumber,
  metricText,
  sanitizePreviewReport,
} from '../lib/metric';
import { useToast } from '../composables/useToast';

const { toast } = useToast();

const mainTabs = [
  { id: 'charts', title: '实时性能走势', icon: Activity },
  { id: 'specs', title: '硬件全景规格', icon: LayoutGrid },
  { id: 'env', title: '系统与工具链', icon: Terminal },
  { id: 'export', title: '健康诊断与导出', icon: FileText },
];

const activeMainTab = ref('charts');
const chartSubMode = ref<'taskmgr' | 'grid'>('taskmgr');
const activeDevice = ref<string>('cpu');
const sanitizeExport = ref(true);

const isFetching = ref(false);

const fullReport = ref<SystemFullReport | null>(null);

function providerStatusFor(sourcePart: string) {
  const normalizedSource = sourcePart.toLowerCase();
  return fullReport.value?.provider_status.find((status) => status.source.toLowerCase().includes(normalizedSource));
}

function fullReportCount(items: readonly unknown[] | undefined, sourcePart: string): string {
  return collectionCount(items, providerStatusFor(sourcePart), fullReport.value !== null);
}

function metric<T>(value: T | null, unit: string, quality: MetricValue<T>['quality'] = 'Unavailable'): MetricValue<T> {
  return { value, unit, quality, source: quality === 'Estimated' ? 'browser-preview' : 'frontend-empty', timestamp: Date.now(), error: null };
}

function emptyMemory(): SystemMemoryInfo {
  return {
    total_physical_bytes: metric<number>(null, 'Bytes'), available_physical_bytes: metric<number>(null, 'Bytes'), used_physical_bytes: metric<number>(null, 'Bytes'), usage_percent: metric<number>(null, '%'),
    total_page_file_bytes: metric<number>(null, 'Bytes'), available_page_file_bytes: metric<number>(null, 'Bytes'), total_virtual_bytes: metric<number>(null, 'Bytes'), available_virtual_bytes: metric<number>(null, 'Bytes'), committed_bytes: metric<number>(null, 'Bytes'), commit_limit_bytes: metric<number>(null, 'Bytes'), paged_pool_bytes: metric<number>(null, 'Bytes'), non_paged_pool_bytes: metric<number>(null, 'Bytes'), hardware_reserved_bytes: metric<number>(null, 'Bytes'), dimms: [], provider_status: { quality: 'Unavailable', source: 'frontend-empty', timestamp: Date.now(), item_count: 0, truncated: false, error: null },
  };
}

function emptyPerformance(): HardwarePerformance {
  const cpuMetric = () => metric<number>(null, '%');
  return { timestamp: Date.now(), cpu: { total_usage_percent: cpuMetric(), user_usage_percent: cpuMetric(), kernel_usage_percent: cpuMetric(), idle_percent: cpuMetric(), base_frequency_mhz: metric<number>(null, 'MHz'), current_frequency_mhz: metric<number>(null, 'MHz'), package_temperature_c: metric<number>(null, '°C'), package_power_watts: metric<number>(null, 'W') }, memory: emptyMemory(), gpus: [], disks: [], network: [], provider_status: [] };
}

function previewPerformance(): HardwarePerformance {
  const total = 16 * 1024 * 1024 * 1024;
  const used = 8 * 1024 * 1024 * 1024;
  const now = Date.now();
  return {
    timestamp: now,
    cpu: { total_usage_percent: metric(22, '%', 'Estimated'), user_usage_percent: metric(12, '%', 'Estimated'), kernel_usage_percent: metric(10, '%', 'Estimated'), idle_percent: metric(78, '%', 'Estimated'), base_frequency_mhz: metric(2400, 'MHz', 'Estimated'), current_frequency_mhz: metric(2400, 'MHz', 'Estimated'), package_temperature_c: metric<number>(null, '°C', 'Unsupported'), package_power_watts: metric<number>(null, 'W', 'Unsupported') },
    memory: { ...emptyMemory(), total_physical_bytes: metric(total, 'Bytes', 'Estimated'), available_physical_bytes: metric(total - used, 'Bytes', 'Estimated'), used_physical_bytes: metric(used, 'Bytes', 'Estimated'), usage_percent: metric(50, '%', 'Estimated') },
    gpus: [], disks: [], network: [], provider_status: [],
  };
}

const perfData = ref<HardwarePerformance>(emptyPerformance());
const MAX_POINTS = 60;
const cpuHistory = ref<Array<number | null>>([]);
const memHistory = ref<Array<number | null>>([]);
const networkHistoryById = ref<Record<string, Array<number | null>>>({});
const diskHistoryById = ref<Record<string, Array<number | null>>>({});
const gpuHistoryById = ref<Record<string, Array<number | null>>>({});

let timer: ReturnType<typeof setInterval> | null = null;

function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes)) return '—';
  if (bytes === 0) return '0 B';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
}

function formatMetricMb(metricValue: MetricValue<number> | null | undefined): string {
  const value = metricNumber(metricValue);
  return value === null ? '—' : `${(value / 1024).toFixed(1)} MB`;
}

function formatGb(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes)) return '—';
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatMetricNumber(metricValue: MetricValue<number> | null | undefined, digits = 1): string {
  const value = metricNumber(metricValue);
  return value === null ? '—' : value.toFixed(digits);
}

// 只有数值有效时才拼接单位，避免出现“—%”等误导性文本。
function formatMetricWithUnit(metricValue: MetricValue<number> | null | undefined, unit: string, digits = 1): string {
  const value = formatMetricNumber(metricValue, digits);
  return value === '—' ? value : `${value}${unit}`;
}

function metricBoolean(value: MetricValue<boolean> | null | undefined): boolean | null {
  if (!value || (value.quality !== 'Good' && value.quality !== 'Estimated') || value.value === null) return null;
  return value.value;
}

function metricBooleanText(value: MetricValue<boolean> | null | undefined): string {
  const state = metricBoolean(value);
  return state === null ? '—' : state ? '已开启' : '未开启';
}

function metricPercentWidth(value: MetricValue<number> | null | undefined): string {
  const percent = metricNumber(value);
  return percent === null ? '0%' : `${Math.min(100, Math.max(0, percent))}%`;
}

function formatSpeed(bps: MetricValue<number> | null | undefined): string {
  const value = metricNumber(bps);
  if (value === null) return '—';
  if (value === 0) return '0 KB/s';
  const kb = value / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  return `${(kb / 1024).toFixed(2)} MB/s`;
}

function appendOptionalMetricPoint(history: Array<number | null>, value: MetricValue<number> | null | undefined): void {
  if (value) {
    appendMetricPoint(history, value, MAX_POINTS);
    return;
  }
  appendMetricGap(history, MAX_POINTS);
}

function appendGapsToDeviceHistories(historyById: Record<string, Array<number | null>>): void {
  for (const history of Object.values(historyById)) appendMetricGap(history, MAX_POINTS);
}

function syncDeviceHistories<T extends { id: string }>(
  historyById: Record<string, Array<number | null>>,
  devices: T[],
  metricFor: (device: T) => MetricValue<number> | null | undefined,
): void {
  const activeIds = new Set(devices.map((device) => device.id));
  for (const id of Object.keys(historyById)) {
    if (!activeIds.has(id)) delete historyById[id];
  }

  for (const device of devices) {
    const history = historyById[device.id] ?? (historyById[device.id] = []);
    appendOptionalMetricPoint(history, metricFor(device));
  }
}

function buildPoints(history: Array<number | null>, maxVal = 100) {
  const step = 100 / Math.max(1, MAX_POINTS - 1);
  const safeMax = Number.isFinite(maxVal) && maxVal > 0 ? maxVal : 1;
  const segments: Array<Array<{ x: number; y: number }>> = [];
  let segment: Array<{ x: number; y: number }> = [];

  for (let index = 0; index < history.length; index += 1) {
    const value = history[index];
    if (value === null || !Number.isFinite(value)) {
      if (segment.length > 0) segments.push(segment);
      segment = [];
      continue;
    }

    const normalized = Math.min(safeMax, Math.max(0, value));
    segment.push({ x: index * step, y: 100 - (normalized / safeMax) * 95 });
  }
  if (segment.length > 0) segments.push(segment);

  const lineSegments = segments.map((points) => points.map(({ x, y }) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' '));
  const areaSegments = segments.map((points) => {
    const line = points.map(({ x, y }) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' ');
    return `${points[0].x.toFixed(1)},100 ${line} ${points[points.length - 1].x.toFixed(1)},100`;
  });

  const latestIndex = history.length - 1;
  const latestValue = latestIndex >= 0 ? history[latestIndex] : null;
  const lastPoint = latestValue === null || latestValue === undefined || !Number.isFinite(latestValue)
    ? null
    : (() => {
        const normalized = Math.min(safeMax, Math.max(0, latestValue));
        return { x: latestIndex * step, y: 100 - (normalized / safeMax) * 95 };
      })();

  return { lineSegments, areaSegments, lastPoint };
}

const currentSelectedDisk = computed<RuntimeDiskInfo | undefined>(() => {
  if (!activeDevice.value.startsWith('disk:')) return undefined;
  const id = activeDevice.value.slice('disk:'.length);
  return perfData.value.disks.find((disk) => disk.id === id);
});

const currentSelectedNetwork = computed<RuntimeNetworkInfo | undefined>(() => {
  if (!activeDevice.value.startsWith('network:')) return undefined;
  return perfData.value.network.find((adapter) => adapter.id === activeDevice.value.slice('network:'.length));
});

const currentSelectedGpu = computed<GpuDevice | undefined>(() => {
  if (!activeDevice.value.startsWith('gpu:')) return undefined;
  return perfData.value.gpus.find((gpu) => gpu.id === activeDevice.value.slice('gpu:'.length));
});

function resetRemovedSelection(): void {
  if (activeDevice.value.startsWith('disk:') && !currentSelectedDisk.value) activeDevice.value = 'cpu';
  if (activeDevice.value.startsWith('network:') && !currentSelectedNetwork.value) activeDevice.value = 'cpu';
  if (activeDevice.value.startsWith('gpu:') && !currentSelectedGpu.value) activeDevice.value = 'cpu';
}

const currentSectionTitle = computed(() => {
  if (activeDevice.value === 'cpu') return '中央处理器 (CPU)';
  if (activeDevice.value === 'memory') return '系统物理内存 (Memory)';
  if (activeDevice.value.startsWith('disk:')) return '逻辑磁盘驱动器 (Disk)';
  if (activeDevice.value.startsWith('network:')) return '网络适配器 (Network)';
  if (activeDevice.value.startsWith('gpu:')) return '图形显示核心 (GPU)';
  return '硬件性能';
});

const currentDeviceHeaderName = computed(() => {
  if (activeDevice.value === 'cpu') return metricText(fullReport.value?.cpu_static.name);
  if (activeDevice.value === 'memory') return `总容量 ${formatBytes(metricNumber(perfData.value.memory.total_physical_bytes))}`;
  if (activeDevice.value.startsWith('disk:')) return metricText(currentSelectedDisk.value?.label);
  if (activeDevice.value.startsWith('network:')) return metricText(currentSelectedNetwork.value?.name);
  if (activeDevice.value.startsWith('gpu:')) return metricText(currentSelectedGpu.value?.name);
  return '—';
});

const currentPrimaryValue = computed(() => {
  if (activeDevice.value === 'cpu') return formatMetricWithUnit(perfData.value.cpu.total_usage_percent, '%');
  if (activeDevice.value === 'memory') return formatBytes(metricNumber(perfData.value.memory.used_physical_bytes));
  if (activeDevice.value.startsWith('disk:')) return formatMetricWithUnit(currentSelectedDisk.value?.usage_percent, '%');
  if (activeDevice.value.startsWith('network:')) return `↓ ${formatSpeed(currentSelectedNetwork.value?.rx_bytes_per_sec)}`;
  if (activeDevice.value.startsWith('gpu:')) return formatMetricWithUnit(currentSelectedGpu.value?.utilization_percent, '%');
  return '—';
});

const currentSecondaryValue = computed(() => {
  if (activeDevice.value === 'cpu') return '实时 CPU 总利用率';
  if (activeDevice.value === 'memory') return `总容量 ${formatBytes(metricNumber(perfData.value.memory.total_physical_bytes))} (${formatMetricWithUnit(perfData.value.memory.usage_percent, '%', 0)})`;
  if (activeDevice.value.startsWith('disk:')) return `剩余 ${formatBytes(metricNumber(currentSelectedDisk.value?.available_bytes))}`;
  if (activeDevice.value.startsWith('network:')) return `↑ ${formatSpeed(currentSelectedNetwork.value?.tx_bytes_per_sec)}`;
  if (activeDevice.value.startsWith('gpu:')) return 'GPU 利用率';
  return '—';
});

const currentColorHex = computed(() => {
  if (activeDevice.value === 'cpu') return '#3b82f6';
  if (activeDevice.value === 'memory') return '#a855f7';
  if (activeDevice.value.startsWith('disk:')) return '#10b981';
  if (activeDevice.value.startsWith('network:')) return '#f59e0b';
  if (activeDevice.value.startsWith('gpu:')) return '#0ea5e9';
  return '#3b82f6';
});

const currentThemeColorClass = computed(() => {
  if (activeDevice.value === 'cpu') return 'text-blue-400';
  if (activeDevice.value === 'memory') return 'text-purple-400';
  if (activeDevice.value.startsWith('disk:')) return 'text-emerald-400';
  if (activeDevice.value.startsWith('network:')) return 'text-amber-400';
  if (activeDevice.value.startsWith('gpu:')) return 'text-sky-400';
  return 'text-blue-400';
});

const currentGradientId = computed(() => `grad-${activeDevice.value.replace(':', '-')}`);

function historyMaximum(history: Array<number | null>, baseline: number): number {
  const values = history.filter((value): value is number => value !== null && Number.isFinite(value));
  return Math.max(baseline, ...values);
}

const currentNetworkHistory = computed(() => {
  if (!activeDevice.value.startsWith('network:')) return [];
  return networkHistoryById.value[activeDevice.value.slice('network:'.length)] ?? [];
});

const currentDiskHistory = computed(() => {
  if (!activeDevice.value.startsWith('disk:')) return [];
  return diskHistoryById.value[activeDevice.value.slice('disk:'.length)] ?? [];
});

const currentGpuHistory = computed(() => {
  if (!activeDevice.value.startsWith('gpu:')) return [];
  return gpuHistoryById.value[activeDevice.value.slice('gpu:'.length)] ?? [];
});

const networkScale = computed(() => historyMaximum(currentNetworkHistory.value, 1024 * 100));

const currentScaleTopLabel = computed(() => {
  if (activeDevice.value === 'cpu' || activeDevice.value === 'memory' || activeDevice.value.startsWith('disk:')) return '100%';
  if (activeDevice.value.startsWith('network:')) return formatSpeed(metric(networkScale.value, 'Bytes/s', 'Estimated'));
  return '100%';
});

const currentChartPoints = computed(() => {
  if (activeDevice.value === 'cpu') return buildPoints(cpuHistory.value, 100);
  if (activeDevice.value === 'memory') return buildPoints(memHistory.value, 100);
  if (activeDevice.value.startsWith('disk:')) return buildPoints(currentDiskHistory.value, 100);
  if (activeDevice.value.startsWith('network:')) {
    return buildPoints(currentNetworkHistory.value, networkScale.value);
  }
  if (activeDevice.value.startsWith('gpu:')) return buildPoints(currentGpuHistory.value, 100);
  return buildPoints(cpuHistory.value, 100);
});

async function fetchData() {
  if (isFetching.value) return;
  isFetching.value = true;
  try {
    // 真实 Tauri 只消费 IPC 快照；浏览器预览才使用明确隔离的 mock。
    const snap = isTauri()
      ? await invoke<HardwarePerformance>('get_performance_snapshot')
      : previewPerformance();
    perfData.value = snap;
    appendOptionalMetricPoint(cpuHistory.value, snap.cpu.total_usage_percent);
    appendOptionalMetricPoint(memHistory.value, snap.memory.usage_percent);
    syncDeviceHistories(networkHistoryById.value, snap.network, (adapter) => adapter.rx_bytes_per_sec);
    syncDeviceHistories(diskHistoryById.value, snap.disks, (disk) => disk.usage_percent);
    syncDeviceHistories(gpuHistoryById.value, snap.gpus, (gpu) => gpu.utilization_percent);
    resetRemovedSelection();
  } catch (err) {
    // IPC 失败也要推进时间轴，避免把上一采样点伪装成连续实时数据。
    appendMetricGap(cpuHistory.value, MAX_POINTS);
    appendMetricGap(memHistory.value, MAX_POINTS);
    appendGapsToDeviceHistories(networkHistoryById.value);
    appendGapsToDeviceHistories(diskHistoryById.value);
    appendGapsToDeviceHistories(gpuHistoryById.value);
    console.error('获取硬件性能失败:', err);
  } finally {
    isFetching.value = false;
  }
}

async function fetchFullReport() {
  try {
    if (isTauri()) {
      fullReport.value = await invoke<SystemFullReport>('get_system_full_report');
    }
  } catch (err) {
    console.error('获取全量报告失败:', err);
  }
}

async function handleManualRefresh() {
  await Promise.all([fetchData(), fetchFullReport()]);
  toast.success('硬件与性能数据已更新', '全量硬件指标已完成最新采样');
}

async function handleCopyReport() {
  try {
    let json = '';
    if (isTauri()) {
      json = await invoke<string>('export_system_report', { sanitize: sanitizeExport.value });
    } else {
      const previewData = sanitizeExport.value ? sanitizePreviewReport(perfData.value) : perfData.value;
      json = JSON.stringify(previewData, null, 2);
    }
    await navigator.clipboard.writeText(json);
    toast.success('已复制诊断报告', sanitizeExport.value ? '已自动脱敏隐私并复制到剪贴板' : '原始完整报告已复制到剪贴板');
  } catch (err) {
    toast.error('复制报告失败', String(err));
  }
}

async function handleExportReport() {
  try {
    let json = '';
    if (isTauri()) {
      json = await invoke<string>('export_system_report', { sanitize: sanitizeExport.value });
    } else {
      const previewData = sanitizeExport.value ? sanitizePreviewReport(perfData.value) : perfData.value;
      json = JSON.stringify(previewData, null, 2);
    }
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `OmniBox-SystemReport-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success('报告导出完成', '已下载至本地文件');
  } catch (err) {
    toast.error('导出失败', String(err));
  }
}

onMounted(() => {
  fetchData();
  fetchFullReport();
  timer = setInterval(fetchData, 1500);
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
});
</script>

/**
 * @file module.ts
 * @description 前端核心类型定义与模块化插件接口规范
 * 包含与 Rust Tauri 后端交互的数据结构以及前端工具插槽扩展接口
 */

import type { Component } from 'vue';

/**
 * 工具分类类别定义
 * - 'system': 系统运维与底层优化 (如内存清理、硬件监控)
 * - 'efficiency': 日常高效工作辅助 (如一键启动器、快捷剪贴板)
 * - 'dev': 开发者工具与调试助手 (如格式化转换、网络排障)
 */
export type ToolCategory = 'system' | 'efficiency' | 'dev';

/**
 * 模块化工具插件接口规范
 * 每一个扩展工具均需实现此接口以接入全局注册中心
 */
export interface ToolModule {
  /** 工具唯一标识符 (例如: 'launcher', 'memory', 'overview') */
  id: string;
  /** 工具在界面中显示的标题名称 */
  title: string;
  /** 工具简要功能说明与定位 */
  description: string;
  /** Lucide 图标标识名称 (如 'Cpu', 'Rocket', 'LayoutDashboard') */
  iconName: string;
  /** 工具所属类别 */
  category: ToolCategory;
  /** 工具的主视图 Vue 组件 */
  component: Component;
  /** 侧边栏与列表展示排序权重 (越小越靠前，默认 99) */
  order?: number;
}

/**
 * 旧版清理前后内存状态（仅保留给兼容的清理领域类型）。
 * 实时 IPC `get_memory_status` 返回 `SystemMemoryInfo`。
 */
export interface MemoryStatus {
  total_ram: number;
  available_ram: number;
  used_ram: number;
  usage_percent: number;
}

/**
 * 内存工作集深度清理结果 (对齐 Rust 端 `CleanResult` 结构体)
 */
export interface CleanResult {
  /** 释放的物理内存字节数 */
  freed_bytes: number;
  /** 释放的内存大小 (MB) */
  freed_mb: number;
  /** 成功修剪工作集的进程数量 */
  processes_trimmed: number;
  /** 清理前内存占用百分比 (0.0 ~ 100.0) */
  before_usage_percent: number;
  /** 清理后内存占用百分比 (0.0 ~ 100.0) */
  after_usage_percent: number;
  /** 释放的备用列表/待机缓存 (Standby Cache) 字节数 */
  standby_freed_bytes: number;
  /** 清理模式描述 */
  clean_mode: string;
  /** 执行时是否具备管理员权限 */
  is_admin: boolean;
}

/**
 * 应用程序通用设置
 */
export interface AppSettings {
  /** 是否以管理员权限自启/首选运行 */
  run_as_admin_default: boolean;
  /** 界面语言 ("zh-CN" | "en-US") */
  language: string;
  /** 是否开机自动启动 */
  auto_start: boolean;
  /** 开机自启时是否默认静默最小化 */
  start_minimized: boolean;
  /** 关闭窗口时最小化到托盘还是退出 */
  close_to_tray: boolean;
  /** 是否启用内存智能自动清理 */
  auto_clean_memory: boolean;
  /** 自动清理内存的占用率阈值 (例如 80 代表 80%) */
  auto_clean_threshold: number;
}

/**
 * 深度隐藏文件/文件夹条目
 */
export interface CloakedItem {
  /** 唯一标识 ID */
  id: string;
  /** 显示名称 */
  name: string;
  /** 文件或文件夹完整绝对路径 */
  path: string;
  /** 是否为目录 */
  is_dir: boolean;
  /** 添加时间戳 (毫秒) */
  added_at: number;
  /** 当前是否处于超级隐藏状态 */
  is_cloaked: boolean;
  /** 备注说明 */
  note: string;
}

/**
 * 自定义启动项配置数据结构 (对齐 Rust 端 `LaunchItem` 结构体)
 */
export interface LaunchItem {
  /** 启动项唯一标识 ID */
  id: string;
  /** 启动项显示名称 */
  name: string;
  /** 目标可执行程序或脚本文件路径 */
  path: string;
  /** 附加命令行启动参数 */
  args: string;
  /** 指定执行工作目录 (可选) */
  work_dir?: string | null;
  /** 是否以静默无窗口模式执行 */
  silent: boolean;
  /** 是否处于启用激活状态 */
  enabled: boolean;
}

/**
 * 批量启动项单项执行结果 (对齐 Rust 端 `BatchLaunchResult` 结构体)
 */
export interface BatchLaunchResult {
  /** 启动项唯一标识 ID */
  id: string;
  /** 启动项显示名称 */
  name: string;
  /** 是否成功启动运行 */
  success: boolean;
  /** 启动成功时的操作系统进程 ID (PID) */
  pid?: number | null;
  /** 启动失败时的详细错误描述 */
  error?: string | null;
}

/**
 * 全局 Toast 提示消息类型
 */
export type ToastType = 'info' | 'success' | 'warning' | 'error';

/**
 * 全局 Toast 提示数据对象
 */
export interface ToastMessage {
  /** 消息唯一 ID */
  id: string;
  /** 消息类型 */
  type: ToastType;
  /** 标题文本 */
  title: string;
  /** 详细消息正文 (可选) */
  message?: string;
  /** 自动消失停留毫秒数 (默认 3000ms) */
  duration?: number;
}

/**
 * 磁盘分区信息 (对齐 Rust 端 `DiskInfo`)
 */
export interface DiskInfo {
  letter: string;
  label: string;
  file_system: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  is_system_drive?: boolean;
}

/**
 * 网络速率快照 (对齐 Rust 端 `NetworkSpeedInfo`)
 */
export interface NetworkSpeedInfo {
  adapter_name: string;
  rx_speed_bps: number;
  tx_speed_bps: number;
  total_rx_bytes: number;
  total_tx_bytes: number;
}

/**
 * CPU 详细硬件与调度指标 (对标任务管理器)
 */
export interface CpuDetailedInfo {
  name: string;
  physical_cores: number;
  logical_cores: number;
  usage_percent: number;
  process_count: number;
  thread_count: number;
  handle_count: number;
  uptime_seconds: number;
  uptime_formatted: string;
}

/**
 * 内存详细硬件与虚拟内存指标 (对标任务管理器)
 */
export interface MemoryDetailedInfo {
  base: MemoryStatus;
  committed_bytes: number;
  commit_limit_bytes: number;
  paged_pool_bytes: number;
  non_paged_pool_bytes: number;
}

/**
 * GPU 详细信息
 */
export interface GpuDetailedInfo {
  name: string;
  status: string;
}

/**
 * 逻辑卷实时性能投影 (对齐 Rust 端 `RuntimeDiskInfo`)
 */
export interface RuntimeDiskInfo {
  id: string;
  drive_letter: MetricValue<string>;
  label: MetricValue<string>;
  file_system: MetricValue<string>;
  total_bytes: MetricValue<number>;
  available_bytes: MetricValue<number>;
  used_bytes: MetricValue<number>;
  usage_percent: MetricValue<number>;
  active_percent: MetricValue<number>;
  read_bytes_per_sec: MetricValue<number>;
  write_bytes_per_sec: MetricValue<number>;
  queue_length: MetricValue<number>;
}

/**
 * 网络适配器实时吞吐投影 (对齐 Rust 端 `RuntimeNetworkInfo`)
 */
export interface RuntimeNetworkInfo {
  id: string;
  name: MetricValue<string>;
  rx_bytes_total: MetricValue<number>;
  tx_bytes_total: MetricValue<number>;
  rx_bytes_per_sec: MetricValue<number>;
  tx_bytes_per_sec: MetricValue<number>;
}

/**
 * 全局硬件性能快照 (对齐 Rust 端 `HardwarePerformance`)
 */
export interface HardwarePerformance {
  timestamp: number;
  cpu: CpuRuntimeInfo;
  memory: SystemMemoryInfo;
  gpus: GpuDevice[];
  disks: RuntimeDiskInfo[];
  network: RuntimeNetworkInfo[];
  provider_status: CollectionStatus[];
}

/**
 * 统一度量指标状态与数据源
 */
export type MetricQuality =
  | 'Good'
  | 'Estimated'
  | 'Stale'
  | 'Unsupported'
  | 'Unavailable'
  | 'PermissionDenied'
  | 'DriverMissing'
  | 'ApiUnavailable'
  | 'ReadError'
  | 'Invalid'
  | 'Unknown';

export interface MetricValue<T> {
  value: T | null;
  unit: string;
  quality: MetricQuality;
  source: string;
  timestamp: number;
  error?: string | null;
}

export interface CollectionStatus {
  quality: MetricQuality;
  source: string;
  timestamp: number;
  item_count?: number | null;
  truncated: boolean;
  error?: string | null;
}

export interface ComputerInfo {
  hostname: MetricValue<string>;
  dns_hostname: MetricValue<string>;
  manufacturer: MetricValue<string>;
  model: MetricValue<string>;
  system_family: MetricValue<string>;
  serial_number: MetricValue<string>;
  uuid: MetricValue<string>;
  domain: MetricValue<string>;
  workgroup: MetricValue<string>;
  system_type: MetricValue<string>;
  current_user: MetricValue<string>;
  uptime_seconds: MetricValue<number>;
  uptime_formatted: MetricValue<string>;
}

export interface MotherboardInfo {
  manufacturer: MetricValue<string>;
  product: MetricValue<string>;
  version: MetricValue<string>;
  serial_number: MetricValue<string>;
  chipset: MetricValue<string>;
}

export interface BiosInfo {
  vendor: MetricValue<string>;
  version: MetricValue<string>;
  release_date: MetricValue<string>;
  smbios_version: MetricValue<string>;
  firmware_mode: MetricValue<string>;
}

export interface WindowsOsInfo {
  name: MetricValue<string>;
  edition: MetricValue<string>;
  display_version: MetricValue<string>;
  build_number: MetricValue<string>;
  ubr: MetricValue<number>;
  architecture: MetricValue<string>;
  install_date: MetricValue<string>;
  windows_directory: MetricValue<string>;
  system_directory: MetricValue<string>;
  system_drive: MetricValue<string>;
  locale: MetricValue<string>;
  timezone: MetricValue<string>;
}

export interface CpuStaticInfo {
  name: MetricValue<string>;
  vendor: MetricValue<string>;
  brand: MetricValue<string>;
  family: MetricValue<number>;
  model: MetricValue<number>;
  stepping: MetricValue<number>;
  physical_cores: MetricValue<number>;
  logical_processors: MetricValue<number>;
  l1_data_cache_kb: MetricValue<number>;
  l1_inst_cache_kb: MetricValue<number>;
  l2_cache_kb: MetricValue<number>;
  l3_cache_kb: MetricValue<number>;
  features: string[];
  virtualization: MetricValue<string>;
}

export interface CpuRuntimeInfo {
  total_usage_percent: MetricValue<number>;
  user_usage_percent: MetricValue<number>;
  kernel_usage_percent: MetricValue<number>;
  idle_percent: MetricValue<number>;
  base_frequency_mhz: MetricValue<number>;
  current_frequency_mhz: MetricValue<number>;
  package_temperature_c: MetricValue<number>;
  package_power_watts: MetricValue<number>;
}

export interface GpuDevice {
  id: string;
  name: MetricValue<string>;
  vendor: MetricValue<string>;
  vendor_id: MetricValue<number>;
  device_id: MetricValue<number>;
  dedicated_vram_bytes: MetricValue<number>;
  shared_vram_bytes: MetricValue<number>;
  driver_version: MetricValue<string>;
  utilization_percent: MetricValue<number>;
  memory_used_bytes: MetricValue<number>;
  temperature_c: MetricValue<number>;
  power_watts: MetricValue<number>;
  fan_speed_percent: MetricValue<number>;
  is_primary: boolean;
}

export interface DimmModule {
  slot: string;
  capacity_bytes: number;
  speed_mhz: number;
  memory_type: string;
  manufacturer: string;
  part_number: string;
  serial_number: string;
  configured_voltage: number;
}

export interface SystemMemoryInfo {
  total_physical_bytes: MetricValue<number>;
  available_physical_bytes: MetricValue<number>;
  used_physical_bytes: MetricValue<number>;
  usage_percent: MetricValue<number>;
  total_page_file_bytes: MetricValue<number>;
  available_page_file_bytes: MetricValue<number>;
  total_virtual_bytes: MetricValue<number>;
  available_virtual_bytes: MetricValue<number>;
  committed_bytes: MetricValue<number>;
  commit_limit_bytes: MetricValue<number>;
  paged_pool_bytes: MetricValue<number>;
  non_paged_pool_bytes: MetricValue<number>;
  hardware_reserved_bytes: MetricValue<number>;
  dimms: DimmModule[];
  provider_status: CollectionStatus;
}

export interface NvmeHealthInfo {
  temperature_c: MetricValue<number>;
  percentage_used: MetricValue<number>;
  available_spare_percent: MetricValue<number>;
  spare_threshold_percent: MetricValue<number>;
  data_units_read_tb: MetricValue<number>;
  data_units_written_tb: MetricValue<number>;
  power_on_hours: MetricValue<number>;
  power_cycles: MetricValue<number>;
  unsafe_shutdowns: MetricValue<number>;
  media_errors: MetricValue<number>;
  critical_warning: MetricValue<number>;
}

export interface PhysicalDiskInfo {
  id: string;
  index: number;
  vendor: MetricValue<string>;
  model: MetricValue<string>;
  serial_number: MetricValue<string>;
  firmware_revision: MetricValue<string>;
  bus_type: MetricValue<string>;
  media_type: MetricValue<string>;
  capacity_bytes: MetricValue<number>;
  sector_size_bytes: MetricValue<number>;
  is_nvme: boolean;
  smart_health?: NvmeHealthInfo | null;
}

export interface VolumeInfo {
  drive_letter: string;
  label: string;
  file_system: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  is_read_only: boolean;
  bitlocker_status: MetricValue<string>;
}

export interface StorageSnapshot {
  physical_disks: PhysicalDiskInfo[];
  volumes: VolumeInfo[];
}

export interface NetworkAdapterInfo {
  index: number;
  name: string;
  alias: string;
  description: string;
  mac_address: string;
  is_physical: boolean;
  oper_status: string;
  link_speed_bps: MetricValue<number>;
  mtu: MetricValue<number>;
  ipv4_addresses: string[];
  ipv6_addresses: string[];
  gateway: string;
  dns_servers: string[];
  dhcp_enabled: MetricValue<boolean>;
  rx_speed_bps: MetricValue<number>;
  tx_speed_bps: MetricValue<number>;
  total_rx_bytes: MetricValue<number>;
  total_tx_bytes: MetricValue<number>;
}

export interface WiFiConnectionInfo {
  is_connected: MetricValue<boolean>;
  ssid: MetricValue<string>;
  bssid: MetricValue<string>;
  signal_quality_percent: MetricValue<number>;
  rssi_dbm: MetricValue<number>;
  channel: MetricValue<number>;
  radio_frequency_ghz: MetricValue<number>;
  security_cipher: MetricValue<string>;
}

export interface NetworkConnectionsSummary {
  tcp_established_count: MetricValue<number>;
  tcp_listening_count: MetricValue<number>;
  tcp_time_wait_count: MetricValue<number>;
  tcp_total_connections: MetricValue<number>;
  udp_endpoints_count: MetricValue<number>;
}

export interface NetworkSnapshot {
  adapters: NetworkAdapterInfo[];
  wifi_info: WiFiConnectionInfo;
  connections_summary: NetworkConnectionsSummary;
}

export interface DisplayDevice {
  id: string;
  name: string;
  friendly_name: string;
  is_primary: boolean;
  width: number;
  height: number;
  refresh_rate_hz: number;
  bits_per_pixel: number;
  orientation: string;
  position_x: number;
  position_y: number;
  hdr_supported: MetricValue<boolean>;
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  is_output: boolean;
  is_default: boolean;
  volume_percent: number;
  is_muted: boolean;
  state: string;
}

export interface MediaDevicesSnapshot {
  displays: DisplayDevice[];
  audio_devices: AudioDeviceInfo[];
}

export interface PnpDeviceEntry {
  device_name: string;
  friendly_name: string;
  manufacturer: string;
  device_class: string;
  hardware_id: string;
  instance_id: string;
  vendor_id?: string | null;
  product_id?: string | null;
  bus_type: string;
}

export interface DevicesSnapshot {
  usb_devices: PnpDeviceEntry[];
  pci_devices: PnpDeviceEntry[];
  other_pnp_devices: PnpDeviceEntry[];
}

export interface BatteryPowerSnapshot {
  has_battery: MetricValue<boolean>;
  ac_connected: MetricValue<boolean>;
  battery_percent: MetricValue<number>;
  charging_status: MetricValue<string>;
  estimated_runtime_minutes: MetricValue<number>;
  power_scheme: MetricValue<string>;
}

export interface ProcessSummaryItem {
  pid: number;
  ppid: number;
  name: string;
  threads: number;
  memory_working_set_bytes: MetricValue<number>;
}

export interface ProcessSnapshot {
  total_processes: number;
  total_threads: number;
  top_memory_processes: ProcessSummaryItem[];
}

export interface StartupEntry {
  name: string;
  command: string;
  source: string;
  enabled: boolean;
}

export interface InstalledAppEntry {
  name: string;
  version: MetricValue<string>;
  publisher: MetricValue<string>;
  install_date: MetricValue<string>;
  install_location: MetricValue<string>;
}

export interface SecurityStatusInfo {
  secure_boot_enabled: MetricValue<boolean>;
  tpm_present: MetricValue<boolean>;
  tpm_version: MetricValue<string>;
  defender_enabled: MetricValue<boolean>;
  defender_realtime_protection: MetricValue<boolean>;
  firewall_domain_enabled: MetricValue<boolean>;
  firewall_private_enabled: MetricValue<boolean>;
  firewall_public_enabled: MetricValue<boolean>;
}

export interface ServiceSummaryItem {
  name: string;
  display_name: string;
  status: string;
}

export interface WindowsEnvSnapshot {
  startup_items: StartupEntry[];
  installed_apps_sample: InstalledAppEntry[];
  security_status: SecurityStatusInfo;
  active_services_sample: ServiceSummaryItem[];
}

export interface DevToolEntry {
  name: string;
  installed: MetricValue<boolean>;
  version: MetricValue<string>;
  path: MetricValue<string>;
}

export interface DevEnvironmentSnapshot {
  tools: DevToolEntry[];
}

export interface CrashDumpEntry {
  file_name: string;
  file_size_bytes: number;
  created_at: string;
}

export interface HardwareEventSummary {
  provider: string;
  event_id: number;
  severity: string;
  description: string;
}

export interface DiagnosticsSnapshot {
  minidump_count: MetricValue<number>;
  recent_crash_dumps: CrashDumpEntry[];
  minidump_truncated: boolean;
  unexpected_shutdowns_count: MetricValue<number>;
  whea_hardware_events: HardwareEventSummary[];
  whea_hardware_events_status: CollectionStatus;
  overall_health_assessment: MetricValue<string>;
}

export interface SystemFullReport {
  computer: ComputerInfo;
  motherboard: MotherboardInfo;
  bios: BiosInfo;
  os: WindowsOsInfo;
  cpu_static: CpuStaticInfo;
  cpu_runtime: CpuRuntimeInfo;
  gpus: GpuDevice[];
  memory: SystemMemoryInfo;
  storage: StorageSnapshot;
  network: NetworkSnapshot;
  media: MediaDevicesSnapshot;
  devices: DevicesSnapshot;
  battery_power: BatteryPowerSnapshot;
  processes: ProcessSnapshot;
  windows_env: WindowsEnvSnapshot;
  dev_env: DevEnvironmentSnapshot;
  diagnostics: DiagnosticsSnapshot;
  timestamp: number;
  provider_status: CollectionStatus[];
}

/**
 * 正在运行的 CDP 浏览器状态
 */
export interface RunningBrowserInfo {
  browser: string;
  endpoint: string;
  port: number;
  profile_dir: string;
  pid: number;
  browser_version: string;
  is_running: boolean;
}

/**
 * 会话迁移器默认配置
 */
export interface SessionMigratorDefaults {
  default_chrome_profile: string;
  default_edge_profile: string;
  sessions_dir: string;
  default_domain: string;
  default_site_url: string;
  default_port: number;
}

/**
 * 单条 Cookie 记录结构
 */
export interface CookieRecord {
  domain: string;
  path: string;
  name: string;
  value: string;
  expirationDate?: number | null;
  secure: boolean;
  httpOnly: boolean;
  sameSite?: string | null;
}

/**
 * 导出环境元数据
 */
export interface ExportMetadata {
  exportedAt: string;
  operatingSystem: string;
  browser: string;
  browserVersion: string;
  userAgent: string;
  acceptLanguage: string;
  timezone: string;
  screenResolution: string;
}

/**
 * 会话打包 Bundle 结构体 (bsm/session-bundle/v1)
 */
export interface SessionBundle {
  schema: string;
  targetDomain: string;
  siteUrl: string;
  metadata: ExportMetadata;
  cookies: CookieRecord[];
}

/**
 * 系统内置工具 (对齐 Rust 端 `SystemToolItem`)
 */
export interface SystemToolItem {
  id: string;
  name: string;
  description: string;
  category: string;
  command: string;
  icon_name: string;
}

/**
 * 系统调优选项 (对齐 Rust 端 `SystemTweakItem`)
 */
export interface SystemTweakItem {
  id: string;
  title: string;
  description: string;
  impact: string;
  is_disabled: boolean;
  requires_admin: boolean;
}

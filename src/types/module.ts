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
 * 系统物理内存实时运行状态 (对齐 Rust 端 `MemoryStatus` 结构体)
 */
export interface MemoryStatus {
  /** 物理总内存 (字节) */
  total_ram: number;
  /** 可用物理内存 (字节) */
  available_ram: number;
  /** 已占用物理内存 (字节) */
  used_ram: number;
  /** 内存占用百分比 (0.0 ~ 100.0) */
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
  standby_freed_bytes?: number;
  /** 清理模式描述 */
  clean_mode?: string;
  /** 执行时是否具备管理员权限 */
  is_admin?: boolean;
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

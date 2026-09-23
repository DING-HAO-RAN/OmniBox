// OmniBox 核心库入口

pub mod commands;
pub mod markdown_pdf;
pub mod session_migrator;
pub mod storage;
pub mod system;
pub mod downloader;

/// 启动 Tauri 桌面应用程序运行时
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let download_manager = std::sync::Arc::new(
        downloader::DownloadManager::new().expect("初始化多线程下载管理器失败"),
    );

    tauri::Builder::default()
        .manage(download_manager)
        .invoke_handler(tauri::generate_handler![
            commands::get_memory_status,
            commands::clean_system_memory,
            commands::execute_launch_item,
            commands::execute_all_launch_items,
            commands::load_launcher_config,
            commands::save_launcher_config,
            commands::read_markdown_file,
            commands::save_markdown_file,
            commands::read_markdown_image,
            commands::choose_markdown_file,
            commands::choose_markdown_output,
            commands::choose_pdf_output,
            markdown_pdf::export_markdown_pdf,
            // 窗口系统控制
            commands::app_minimize_window,
            commands::app_toggle_maximize_window,
            commands::app_close_window,
            commands::app_is_maximized,
            // 系统状态与通用设置
            commands::get_admin_status,
            commands::request_restart_as_admin,
            commands::get_app_settings,
            commands::update_app_settings,
            // 深度隐藏文件功能
            commands::cloak_file_or_dir,
            commands::uncloak_file_or_dir,
            commands::recloak_file_or_dir,
            commands::load_cloaked_list,
            commands::remove_cloaked_record,
            commands::send_cloaked_to_launcher,
            commands::choose_any_file,
            // 硬件性能监控
            commands::get_performance_snapshot,
            commands::get_system_full_report,
            commands::export_system_report,
            // 原生系统工具箱
            commands::get_system_tools,
            commands::launch_system_tool_cmd,
            // 系统特性一键优化禁用
            commands::get_system_tweaks,
            commands::apply_system_tweak,
            // 系统限制一键全量解除 (USB 与网络使用限制)
            commands::get_restriction_status,
            commands::unrestrict_usb_all,
            commands::unrestrict_network_all,
            commands::unrestrict_everything,
            // 浏览器会话迁移器 (Bilibili/通用会话登录器)
            commands::session_migrator_get_defaults,
            commands::session_migrator_launch_cdp,
            commands::session_migrator_stop_cdp,
            commands::session_migrator_get_cdp_status,
            commands::session_migrator_create_launcher,
            commands::session_migrator_choose_profile_dir,
            commands::session_migrator_open_sessions_dir,
            commands::session_migrator_choose_file,
            commands::session_migrator_write_file,
            commands::session_migrator_read_file,
            commands::session_migrator_compute_sha256,
            // 高速多线程下载器 (TurboDownloader)
            downloader::commands::downloader_create_task,
            downloader::commands::downloader_list_tasks,
            downloader::commands::downloader_pause_task,
            downloader::commands::downloader_resume_task,
            downloader::commands::downloader_delete_task,
            downloader::commands::downloader_probe_url,
            downloader::commands::downloader_choose_dir,
            downloader::commands::downloader_open_file,
            downloader::commands::downloader_open_folder,
        ])
        .setup(|_app| {
            // 初始化阶段逻辑钩子，后续任务可在此注册插件或系统托盘
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 OmniBox 应用程序时发生错误");
}

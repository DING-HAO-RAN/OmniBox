// OmniBox 核心库入口

pub mod commands;
pub mod storage;
pub mod system;

/// 启动 Tauri 桌面应用程序运行时
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_memory_status,
            commands::clean_system_memory,
            commands::execute_launch_item,
            commands::execute_all_launch_items,
            commands::load_launcher_config,
            commands::save_launcher_config,
        ])
        .setup(|_app| {
            // 初始化阶段逻辑钩子，后续任务可在此注册插件或系统托盘
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 OmniBox 应用程序时发生错误");
}

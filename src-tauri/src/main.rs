// 在 Windows Release 构建下隐藏默认的控制台黑窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 应用程序主执行入口
fn main() {
    omnibox_lib::run();
}

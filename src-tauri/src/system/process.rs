//! Windows 进程创建与启动管理模块
//!
//! 提供普通模式与静默模式 (`CREATE_NO_WINDOW`) 启动外部进程的能力，
//! 支持自定义工作目录与命令行参数。

use crate::system::types::LaunchItem;
use std::mem::size_of;
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, CREATE_NO_WINDOW, PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

/// 启动指定配置的外部进程，并返回其系统进程 ID (PID)
///
/// # 参数
/// - `item`: 待启动进程的配置项（包含路径、参数、工作目录及静默标志）
///
/// # 行为
/// - 若 `item.silent` 为 true：
///   - 附加 `CREATE_NO_WINDOW` 创建标志；
///   - 设置窗口启动状态为 `SW_HIDE`；
/// - 若 `item.silent` 为 false：
///   - 以普通模式启动，设置窗口启动状态为 `SW_SHOWNORMAL`；
/// - 成功创建进程后，立即释放进程及主线程句柄以防止资源泄漏，并返回 PID。
pub fn launch_process(item: &LaunchItem) -> Result<u32, String> {
    let trimmed_path = item.path.trim();
    if trimmed_path.is_empty() {
        return Err("可执行文件路径不能为空".to_string());
    }

    // 格式化命令行：对路径增加双引号包围以安全支持包含空格的路径，随后拼接参数
    let mut command_line_str = if trimmed_path.starts_with('"') && trimmed_path.ends_with('"') {
        trimmed_path.to_string()
    } else {
        format!("\"{}\"", trimmed_path)
    };

    let trimmed_args = item.args.trim();
    if !trimmed_args.is_empty() {
        command_line_str.push(' ');
        command_line_str.push_str(trimmed_args);
    }

    // 将命令行转为以 null 结尾的 UTF-16 宽字符序列
    // 注：Win32 CreateProcessW 可能会就地修改命令行缓冲区，因此必须为可变切片
    let mut command_line_wide: Vec<u16> = command_line_str
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // 解析工作目录为宽字符格式（如果指定）
    let work_dir_wide: Option<Vec<u16>> = item
        .work_dir
        .as_ref()
        .filter(|dir| !dir.trim().is_empty())
        .map(|dir| dir.encode_utf16().chain(std::iter::once(0)).collect());

    let work_dir_ptr = match &work_dir_wide {
        Some(w) => w.as_ptr(),
        None => ptr::null(),
    };

    unsafe {
        let mut startup_info: STARTUPINFOW = std::mem::zeroed();
        startup_info.cb = size_of::<STARTUPINFOW>() as u32;

        let creation_flags;
        if item.silent {
            // 静默模式：创建无控制台窗口进程，且隐藏窗口
            creation_flags = CREATE_NO_WINDOW;
            startup_info.dwFlags = STARTF_USESHOWWINDOW;
            startup_info.wShowWindow = SW_HIDE as u16;
        } else {
            // 普通模式：正常显示窗口
            creation_flags = 0;
            startup_info.dwFlags = STARTF_USESHOWWINDOW;
            startup_info.wShowWindow = SW_SHOWNORMAL as u16;
        }

        let mut process_info: PROCESS_INFORMATION = std::mem::zeroed();

        let success = CreateProcessW(
            ptr::null(),                    // 应用程序名称（传 NULL 时由命令行首段解析）
            command_line_wide.as_mut_ptr(), // 命令行字符串
            ptr::null(),                    // 进程安全属性
            ptr::null(),                    // 线程安全属性
            0,                              // 句柄继承选项 (false)
            creation_flags,                 // 进程创建标志
            ptr::null(),                    // 继承环境块
            work_dir_ptr,                   // 当前工作目录
            &startup_info,                  // 启动参数信息
            &mut process_info,              // 接收创建后的进程信息
        );

        if success == 0 {
            let error_code = GetLastError();
            return Err(format!(
                "启动进程失败: \"{}\", Windows 错误码: {}",
                item.path, error_code
            ));
        }

        let pid = process_info.dwProcessId;

        // 关闭句柄以避免内存/句柄资源泄漏；关闭句柄不影响子进程独立运行
        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);

        Ok(pid)
    }
}

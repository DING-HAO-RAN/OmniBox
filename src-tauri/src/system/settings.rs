//! 系统配置与 Windows 集成控制模块
//!
//! 负责开机自启动注册表配置、提升管理员权限重启、系统状态查询等。

use std::env;
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HINSTANCE, HWND};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_REG_NAME: &str = "OmniBox";

/// 查询当前应用程序是否已加入 Windows 开机自启
pub fn get_auto_start_status() -> Result<bool, String> {
    let subkey_wide: Vec<u16> = RUN_KEY_PATH
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name_wide: Vec<u16> = APP_REG_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = std::ptr::null_mut();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey_wide.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        if status != 0 {
            return Ok(false);
        }

        let mut data_type = 0u32;
        let mut data_len = 0u32;
        let query_res = RegQueryValueExW(
            hkey,
            value_name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut data_len,
        );

        RegCloseKey(hkey);
        Ok(query_res == 0)
    }
}

/// 设置或取消 Windows 开机自启状态
pub fn set_auto_start_status(enable: bool) -> Result<(), String> {
    let subkey_wide: Vec<u16> = RUN_KEY_PATH
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name_wide: Vec<u16> = APP_REG_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = std::ptr::null_mut();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey_wide.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if status != 0 {
            return Err("打开注册表开机启动键失败".to_string());
        }

        if enable {
            let current_exe = env::current_exe()
                .map_err(|e| format!("获取当前可执行文件路径失败: {e}"))?;
            let exe_str = current_exe.to_string_lossy();
            let quoted_exe = format!("\"{}\"", exe_str);
            let exe_wide: Vec<u16> = quoted_exe
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let byte_len = (exe_wide.len() * 2) as u32;
            let set_res = RegSetValueExW(
                hkey,
                value_name_wide.as_ptr(),
                0,
                REG_SZ,
                exe_wide.as_ptr() as *const u8,
                byte_len,
            );
            RegCloseKey(hkey);

            if set_res != 0 {
                return Err("写入开机自启注册表失败".to_string());
            }
        } else {
            let _ = RegDeleteValueW(hkey, value_name_wide.as_ptr());
            RegCloseKey(hkey);
        }

        Ok(())
    }
}

/// 以 Windows 管理员身份重新启动当前程序
pub fn restart_as_admin() -> Result<(), String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("获取当前程序路径失败: {e}"))?;
    let exe_wide: Vec<u16> = current_exe
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let verb_wide: Vec<u16> = "runas"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut sei = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: 0x00000040, // SEE_MASK_NOCLOSEPROCESS
            hwnd: 0 as HWND,
            lpVerb: verb_wide.as_ptr(),
            lpFile: exe_wide.as_ptr(),
            lpParameters: std::ptr::null(),
            lpDirectory: std::ptr::null(),
            nShow: SW_SHOWNORMAL as i32,
            hInstApp: 0 as HINSTANCE,
            lpIDList: std::ptr::null_mut(),
            lpClass: std::ptr::null(),
            hkeyClass: std::ptr::null_mut(),
            dwHotKey: 0,
            Anonymous: std::mem::zeroed(),
            hProcess: std::ptr::null_mut(),
        };

        if ShellExecuteExW(&mut sei) == FALSE {
            return Err("以管理员身份启动程序失败，用户可能取消了 UAC 授权".to_string());
        }

        if !sei.hProcess.is_null() {
            CloseHandle(sei.hProcess);
        }

        // 提权成功后延时退出当前进程
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            std::process::exit(0);
        });

        Ok(())
    }
}

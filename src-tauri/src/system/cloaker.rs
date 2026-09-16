//! Windows 深度隐藏文件/文件夹模块 (File Cloaker)
//!
//! 利用 Windows 系统级受保护属性 (`FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_HIDDEN`)，
//! 使选定的文件或文件夹在文件资源管理器中实现即使勾选“显示隐藏文件”也完全不可见的“超级隐身”效果。

use std::path::Path;
use windows_sys::Win32::Storage::FileSystem::{
    GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_SYSTEM, INVALID_FILE_ATTRIBUTES,
};

/// 检查路径是否存在
pub fn path_exists(path_str: &str) -> bool {
    Path::new(path_str).exists()
}

/// 检查文件或目录当前是否处于系统级深度隐形状态 (System + Hidden)
pub fn is_path_cloaked(path_str: &str) -> Result<bool, String> {
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let attrs = GetFileAttributesW(wide_path.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(format!("无法读取路径属性或路径不存在: {path_str}"));
        }

        let is_sys = (attrs & FILE_ATTRIBUTE_SYSTEM) != 0;
        let is_hidden = (attrs & FILE_ATTRIBUTE_HIDDEN) != 0;
        Ok(is_sys && is_hidden)
    }
}

/// 对指定文件或目录施加系统级深度隐藏 (Super Hidden)
pub fn cloak_path(path_str: &str) -> Result<(), String> {
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let attrs = GetFileAttributesW(wide_path.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(format!("无法找到目标路径: {path_str}"));
        }

        // 添加系统属性和隐藏属性
        let new_attrs = attrs | FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_HIDDEN;
        if SetFileAttributesW(wide_path.as_ptr(), new_attrs) == 0 {
            return Err(format!("设置深度隐藏属性失败，请检查文件权限: {path_str}"));
        }

        Ok(())
    }
}

/// 解除文件或目录的深度隐藏状态，恢复正常可见
pub fn uncloak_path(path_str: &str) -> Result<(), String> {
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let attrs = GetFileAttributesW(wide_path.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(format!("无法找到目标路径: {path_str}"));
        }

        // 移除系统属性和隐藏属性
        let mut new_attrs = attrs & !FILE_ATTRIBUTE_SYSTEM & !FILE_ATTRIBUTE_HIDDEN;
        if new_attrs == 0 {
            new_attrs = FILE_ATTRIBUTE_NORMAL;
        }

        if SetFileAttributesW(wide_path.as_ptr(), new_attrs) == 0 {
            return Err(format!("恢复文件可见属性失败，请检查文件权限: {path_str}"));
        }

        Ok(())
    }
}

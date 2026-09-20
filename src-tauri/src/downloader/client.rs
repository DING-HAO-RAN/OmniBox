// Windows WinHTTP 网络探测与分块切分客户端
// 包含 URL 解析、文件名提取、分块切分算法以及 WinHTTP API 安全封装

use super::types::{DownloadChunk, UrlMeta};
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_ACCEPT_RANGES, WINHTTP_QUERY_CONTENT_DISPOSITION, WINHTTP_QUERY_CONTENT_LENGTH,
    WINHTTP_QUERY_CUSTOM, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

/// WinHTTP 句柄 RAII 自动释放安全包装器
struct SafeHInternet(*mut std::ffi::c_void);

impl SafeHInternet {
    pub fn new(handle: *mut std::ffi::c_void) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    pub fn raw(&self) -> *mut std::ffi::c_void {
        self.0
    }
}

impl Drop for SafeHInternet {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

/// 将 Rust 字符串转换为 Windows API 要求的以 null 结尾的宽字符向量 (UTF-16)
fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 解析 URL 组成部分：(是否为 HTTPS, 主机名, 端口, 路径及查询串)
pub fn parse_url_components(url: &str) -> Result<(bool, String, u16, String), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL 不能为空".to_string());
    }

    let (is_https, default_port, rest) = if let Some(stripped) = trimmed.strip_prefix("https://") {
        (true, 443u16, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("http://") {
        (false, 80u16, stripped)
    } else {
        return Err("仅支持 HTTP 或 HTTPS 协议".to_string());
    };

    if rest.is_empty() {
        return Err("URL 缺少主机名".to_string());
    }

    // 分离主机端口部分与路径/查询部分
    let path_start_idx = rest.find(|c| c == '/' || c == '?' || c == '#');
    let (host_port_part, raw_path_part) = match path_start_idx {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    if host_port_part.is_empty() {
        return Err("主机名不能为空".to_string());
    }

    // 剔除 URL 中的 fragment (# 后面的锚点)
    let path_no_fragment = match raw_path_part.find('#') {
        Some(idx) => &raw_path_part[..idx],
        None => raw_path_part,
    };

    // 格式化 path_and_query
    let path_and_query = if path_no_fragment.is_empty() {
        "/".to_string()
    } else if !path_no_fragment.starts_with('/') {
        format!("/{}", path_no_fragment)
    } else {
        path_no_fragment.to_string()
    };

    // 解析 host 和 port (考虑 IPv6 [::1]:port 形式)
    let (host, port) = if host_port_part.starts_with('[') {
        // IPv6
        if let Some(close_bracket) = host_port_part.find(']') {
            let host_inner = &host_port_part[1..close_bracket];
            let after = &host_port_part[close_bracket + 1..];
            if after.starts_with(':') {
                let p: u16 = after[1..]
                    .parse()
                    .map_err(|e| format!("无效的 IPv6 端口: {}", e))?;
                (host_inner.to_string(), p)
            } else {
                (host_inner.to_string(), default_port)
            }
        } else {
            return Err("畸形的 IPv6 主机格式".to_string());
        }
    } else if let Some(colon_idx) = host_port_part.rfind(':') {
        let host_name = &host_port_part[..colon_idx];
        let port_str = &host_port_part[colon_idx + 1..];
        if host_name.is_empty() {
            return Err("主机名不能为空".to_string());
        }
        let p: u16 = port_str
            .parse()
            .map_err(|e| format!("无效的端口号 {}: {}", port_str, e))?;
        (host_name.to_string(), p)
    } else {
        (host_port_part.to_string(), default_port)
    };

    Ok((is_https, host, port, path_and_query))
}

/// URL 百分号编码解码工具函数
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let high = (bytes[i + 1] as char).to_digit(16);
            let low = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (high, low) {
                decoded.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).to_string()
}

/// 清洗并净化文件名，去除 Windows 非法字符并防止路径遍历
fn sanitize_filename(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len());
    for c in name.chars() {
        // Windows 文件名非法字符: \ / : * ? " < > | 以及 ASCII 控制字符
        if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
            cleaned.push('_');
        } else {
            cleaned.push(c);
        }
    }
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "download.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 优先从 Content-Disposition 提取文件名，若无则从 URL 路径尾部提取
pub fn extract_filename_from_url(url: &str, disposition: Option<&str>) -> String {
    // 1. 优先尝试解析 Content-Disposition
    if let Some(disp) = disposition {
        // 先检查 RFC 5987 / RFC 6266 标准的 filename*=
        if let Some(idx) = disp.to_ascii_lowercase().find("filename*=") {
            let rest = &disp[idx + 10..];
            let value_part = rest.split(';').next().unwrap_or("").trim();
            // 格式可能是 UTF-8''encoded_name 或直接 encoded_name
            let raw_encoded = if let Some(last_quote) = value_part.rfind("''") {
                &value_part[last_quote + 2..]
            } else {
                value_part
            };
            let decoded = percent_decode(raw_encoded.trim_matches('"'));
            let sanitized = sanitize_filename(&decoded);
            if sanitized != "download.bin" {
                return sanitized;
            }
        }

        // 检查标准 filename=
        if let Some(idx) = disp.to_ascii_lowercase().find("filename=") {
            let rest = &disp[idx + 9..];
            let value_part = rest.split(';').next().unwrap_or("").trim();
            let unquoted = value_part.trim_matches('"').trim_matches('\'').trim();
            let sanitized = sanitize_filename(unquoted);
            if sanitized != "download.bin" {
                return sanitized;
            }
        }
    }

    // 2. 从 URL 路径截取文件名
    let url_without_fragment = url.split('#').next().unwrap_or("");
    let url_without_query = url_without_fragment.split('?').next().unwrap_or("");
    if let Some(last_slash) = url_without_query.rfind('/') {
        let segment = &url_without_query[last_slash + 1..];
        if !segment.is_empty() {
            let decoded = percent_decode(segment);
            let sanitized = sanitize_filename(&decoded);
            if sanitized != "download.bin" {
                return sanitized;
            }
        }
    }

    // 3. 兜底默认文件名
    "download.bin".to_string()
}

/// 将指定文件大小均匀切分为连续无缝无重叠的分片
pub fn split_into_chunks(total_bytes: u64, thread_count: usize) -> Vec<DownloadChunk> {
    let threads = if thread_count == 0 { 1 } else { thread_count };

    // 0 字节特殊情况处理
    if total_bytes == 0 {
        return vec![DownloadChunk {
            id: 0,
            start: 0,
            end: 0,
            downloaded: 0,
            is_finished: true,
        }];
    }

    // 如果字节数小于线程数，分片数量调整为字节数大小
    let actual_threads = if total_bytes < threads as u64 {
        total_bytes as usize
    } else {
        threads
    };

    let base_size = total_bytes / actual_threads as u64;
    let remainder = total_bytes % actual_threads as u64;

    let mut chunks = Vec::with_capacity(actual_threads);
    let mut current_start = 0u64;

    for i in 0..actual_threads {
        let chunk_len = if (i as u64) < remainder {
            base_size + 1
        } else {
            base_size
        };

        let start = current_start;
        let end = current_start + chunk_len - 1;
        current_start = end + 1;

        chunks.push(DownloadChunk {
            id: i,
            start,
            end,
            downloaded: 0,
            is_finished: false,
        });
    }

    chunks
}

/// 从 HTTP 响应中读取指定的头部字符串
unsafe fn query_header_str(
    h_request: *mut std::ffi::c_void,
    info_level: u32,
    custom_name: Option<&[u16]>,
) -> Option<String> {
    let custom_ptr = match custom_name {
        Some(slice) => slice.as_ptr(),
        None => std::ptr::null(),
    };

    let mut byte_len = 0u32;
    // 第一次调用获取所需的缓冲区字节大小
    let _ = WinHttpQueryHeaders(
        h_request,
        info_level,
        custom_ptr,
        std::ptr::null_mut(),
        &mut byte_len,
        std::ptr::null_mut(),
    );

    if byte_len == 0 {
        return None;
    }

    let char_len = (byte_len as usize) / std::mem::size_of::<u16>();
    let mut buffer = vec![0u16; char_len + 1];

    let success = WinHttpQueryHeaders(
        h_request,
        info_level,
        custom_ptr,
        buffer.as_mut_ptr() as *mut std::ffi::c_void,
        &mut byte_len,
        std::ptr::null_mut(),
    );

    if success != 0 {
        let valid_len = buffer.iter().position(|&c| c == 0).unwrap_or(char_len);
        Some(String::from_utf16_lossy(&buffer[..valid_len]).trim().to_string())
    } else {
        None
    }
}

/// 从 HTTP 响应中读取数值状态码 (如 200, 206, 404 等)
unsafe fn query_status_code(h_request: *mut std::ffi::c_void) -> Result<u32, String> {
    let mut status_code = 0u32;
    let mut len = std::mem::size_of::<u32>() as u32;
    let success = WinHttpQueryHeaders(
        h_request,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        std::ptr::null(),
        &mut status_code as *mut _ as *mut std::ffi::c_void,
        &mut len,
        std::ptr::null_mut(),
    );

    if success != 0 {
        Ok(status_code)
    } else {
        let err = GetLastError();
        Err(format!("获取 HTTP 响应状态码失败，系统错误代码: {}", err))
    }
}

/// 解析 Content-Range 响应头中的文件总大小
/// 例如: "bytes 0-0/1048576" -> 1048576
fn parse_total_from_content_range(content_range: &str) -> Option<u64> {
    let slash_idx = content_range.rfind('/')?;
    let total_part = content_range[slash_idx + 1..].trim();
    total_part.parse::<u64>().ok()
}

/// 使用 WinHTTP 探测 URL 目标大小、是否支持 Range 以及建议文件名
/// 流程：优先使用 HEAD 轻量探测；若服务器不支持 HEAD 或未声明 Range，则 fallback 到 Range: bytes=0-0 GET 探测
pub fn probe_url_meta(url: &str, custom_user_agent: Option<&str>) -> Result<UrlMeta, String> {
    let (is_https, host, port, path_and_query) = parse_url_components(url)?;

    let ua = custom_user_agent.unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) OmniBox-TurboDownloader/1.0");
    let ua_wide = to_wide_null(ua);
    let host_wide = to_wide_null(&host);
    let path_wide = to_wide_null(&path_and_query);

    // 1. 初始化 WinHTTP 会话
    let session = SafeHInternet::new(unsafe {
        WinHttpOpen(
            ua_wide.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    })
    .ok_or_else(|| {
        let err = unsafe { GetLastError() };
        format!("初始化 WinHTTP 会话失败，系统错误码: {}", err)
    })?;

    // 设置网络超时 (解析 10s, 连接 10s, 发送 15s, 接收 15s)
    unsafe {
        WinHttpSetTimeouts(session.raw(), 10_000, 10_000, 15_000, 15_000);
    }

    // 2. 连接服务器
    let connect = SafeHInternet::new(unsafe {
        WinHttpConnect(session.raw(), host_wide.as_ptr(), port, 0)
    })
    .ok_or_else(|| {
        let err = unsafe { GetLastError() };
        format!("连接目标服务器 {}:{} 失败，系统错误码: {}", host, port, err)
    })?;

    let req_flags = if is_https { WINHTTP_FLAG_SECURE } else { 0 };

    // 3. 第一阶段：尝试 HEAD 探测
    let head_verb = to_wide_null("HEAD");
    let head_request = SafeHInternet::new(unsafe {
        WinHttpOpenRequest(
            connect.raw(),
            head_verb.as_ptr(),
            path_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            req_flags,
        )
    });

    if let Some(req) = head_request {
        let send_res = unsafe {
            WinHttpSendRequest(
                req.raw(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                0,
                0,
            )
        };

        if send_res != 0 && unsafe { WinHttpReceiveResponse(req.raw(), std::ptr::null_mut()) } != 0 {
            if let Ok(status) = unsafe { query_status_code(req.raw()) } {
                if status == 200 || status == 206 {
                    let cl_str = unsafe { query_header_str(req.raw(), WINHTTP_QUERY_CONTENT_LENGTH, None) };
                    let total_bytes = cl_str.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

                    let ar_str = unsafe { query_header_str(req.raw(), WINHTTP_QUERY_ACCEPT_RANGES, None) };
                    let supports_range = ar_str
                        .as_deref()
                        .map(|s| s.to_ascii_lowercase().contains("bytes"))
                        .unwrap_or(false);

                    let disp_str = unsafe { query_header_str(req.raw(), WINHTTP_QUERY_CONTENT_DISPOSITION, None) };
                    let suggested_filename = extract_filename_from_url(url, disp_str.as_deref());

                    // 若 HEAD 明确指出支持 bytes 分片且已拿到大小，则直接返回
                    if supports_range && total_bytes > 0 {
                        return Ok(UrlMeta {
                            total_bytes,
                            supports_range: true,
                            suggested_filename,
                        });
                    }
                }
            }
        }
    }

    // 4. 第二阶段：Fallback 发送 Range: bytes=0-0 GET 探测
    let get_verb = to_wide_null("GET");
    let get_request = SafeHInternet::new(unsafe {
        WinHttpOpenRequest(
            connect.raw(),
            get_verb.as_ptr(),
            path_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            req_flags,
        )
    })
    .ok_or_else(|| {
        let err = unsafe { GetLastError() };
        format!("创建 WinHTTP GET 探测请求失败，系统错误码: {}", err)
    })?;

    let range_header = to_wide_null("Range: bytes=0-0\r\n");
    let send_get_res = unsafe {
        WinHttpSendRequest(
            get_request.raw(),
            range_header.as_ptr(),
            (range_header.len() - 1) as u32,
            std::ptr::null(),
            0,
            0,
            0,
        )
    };

    if send_get_res == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!("发送 WinHTTP GET 探测请求失败，系统错误码: {}", err));
    }

    if unsafe { WinHttpReceiveResponse(get_request.raw(), std::ptr::null_mut()) } == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!("接收 WinHTTP 探测响应失败，系统错误码: {}", err));
    }

    let status = unsafe { query_status_code(get_request.raw()) }?;
    let disp_str = unsafe { query_header_str(get_request.raw(), WINHTTP_QUERY_CONTENT_DISPOSITION, None) };
    let suggested_filename = extract_filename_from_url(url, disp_str.as_deref());

    if status == 206 {
        // 206 Partial Content: 明确支持 HTTP Range
        let content_range_name = to_wide_null("Content-Range");
        let cr_str = unsafe {
            query_header_str(
                get_request.raw(),
                WINHTTP_QUERY_CUSTOM,
                Some(&content_range_name),
            )
        };

        let total_bytes = cr_str
            .as_deref()
            .and_then(parse_total_from_content_range)
            .or_else(|| {
                // 如果 Content-Range 中未包含总数，尝试从 Content-Length 获取
                unsafe {
                    query_header_str(get_request.raw(), WINHTTP_QUERY_CONTENT_LENGTH, None)
                }
                .and_then(|s| s.parse::<u64>().ok())
            })
            .unwrap_or(0);

        Ok(UrlMeta {
            total_bytes,
            supports_range: true,
            suggested_filename,
        })
    } else if status == 200 {
        // 200 OK: 服务器不支持 Range，返回了全量文件
        let cl_str = unsafe { query_header_str(get_request.raw(), WINHTTP_QUERY_CONTENT_LENGTH, None) };
        let total_bytes = cl_str.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

        Ok(UrlMeta {
            total_bytes,
            supports_range: false,
            suggested_filename,
        })
    } else {
        Err(format!("HTTP 网络探测失败，服务器返回异常状态码: {}", status))
    }
}

/// 使用 WinHTTP 进行 Range 分片下载或全量流式下载
///
/// 边拉取网络字节边通过 `on_data` 回调提供给调用方，便于调用方直接 Seek 写入目标文件。
/// 支持传入原子取消信号以实现及时中断。
pub fn download_range_stream<F>(
    url: &str,
    range: Option<(u64, u64)>,
    custom_user_agent: Option<&str>,
    cancel_token: &std::sync::atomic::AtomicBool,
    mut on_data: F,
) -> Result<u64, String>
where
    F: FnMut(&[u8]) -> Result<(), String>,
{
    let (is_https, host, port, path_and_query) = parse_url_components(url)?;

    let ua = custom_user_agent.unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) OmniBox-TurboDownloader/1.0");
    let ua_wide = to_wide_null(ua);
    let host_wide = to_wide_null(&host);
    let path_wide = to_wide_null(&path_and_query);

    // 1. 初始化 WinHTTP 会话
    let session = SafeHInternet::new(unsafe {
        WinHttpOpen(
            ua_wide.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    })
    .ok_or_else(|| {
        let err = unsafe { GetLastError() };
        format!("初始化 WinHTTP 会话失败，系统错误码: {}", err)
    })?;

    // 设置超时时间 (解析 10s, 连接 10s, 发送 15s, 接收 30s)
    unsafe {
        WinHttpSetTimeouts(session.raw(), 10_000, 10_000, 15_000, 30_000);
    }

    // 2. 连接服务器
    let connect = SafeHInternet::new(unsafe {
        WinHttpConnect(session.raw(), host_wide.as_ptr(), port, 0)
    })
    .ok_or_else(|| {
        let err = unsafe { GetLastError() };
        format!("连接目标服务器 {}:{} 失败，系统错误码: {}", host, port, err)
    })?;

    let req_flags = if is_https { WINHTTP_FLAG_SECURE } else { 0 };

    // 3. 打开 GET 请求
    let get_verb = to_wide_null("GET");
    let request = SafeHInternet::new(unsafe {
        WinHttpOpenRequest(
            connect.raw(),
            get_verb.as_ptr(),
            path_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            req_flags,
        )
    })
    .ok_or_else(|| {
        let err = unsafe { GetLastError() };
        format!("创建 WinHTTP GET 请求失败，系统错误码: {}", err)
    })?;

    // 4. 配置 Range 请求头
    let (header_ptr, _header_len) = if let Some((start, end)) = range {
        let range_header = format!("Range: bytes={}-{}\r\n", start, end);
        let wide = to_wide_null(&range_header);
        // 保存 wide 变量延长生命周期
        (Some(wide), range_header.len())
    } else {
        (None, 0)
    };

    let send_res = match &header_ptr {
        Some(wide) => unsafe {
            WinHttpSendRequest(
                request.raw(),
                wide.as_ptr(),
                (wide.len() - 1) as u32,
                std::ptr::null(),
                0,
                0,
                0,
            )
        },
        None => unsafe {
            WinHttpSendRequest(
                request.raw(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                0,
                0,
            )
        },
    };

    if send_res == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!("发送 WinHTTP 下载请求失败，系统错误码: {}", err));
    }

    if unsafe { WinHttpReceiveResponse(request.raw(), std::ptr::null_mut()) } == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!("接收 WinHTTP 响应失败，系统错误码: {}", err));
    }

    let status = unsafe { query_status_code(request.raw()) }?;
    if status != 200 && status != 206 {
        return Err(format!("下载请求失败，服务器返回 HTTP 状态码: {}", status));
    }

    // 5. 循环读取数据流
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total_downloaded: u64 = 0;
    let max_expected = range.map(|(s, e)| e.saturating_sub(s) + 1);

    loop {
        if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("download_cancelled".to_string());
        }

        let mut bytes_read = 0u32;
        let to_read = if let Some(expected) = max_expected {
            let remaining = expected.saturating_sub(total_downloaded);
            if remaining == 0 {
                break;
            }
            (remaining as usize).min(buffer.len()) as u32
        } else {
            buffer.len() as u32
        };

        let read_ok = unsafe {
            WinHttpReadData(
                request.raw(),
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                to_read,
                &mut bytes_read,
            )
        };

        if read_ok == 0 {
            let err = unsafe { GetLastError() };
            return Err(format!("读取网络数据流失败，系统错误码: {}", err));
        }

        if bytes_read == 0 {
            break;
        }

        on_data(&buffer[..bytes_read as usize])?;
        total_downloaded += bytes_read as u64;

        if let Some(expected) = max_expected {
            if total_downloaded >= expected {
                break;
            }
        }
    }

    Ok(total_downloaded)
}

// 下载器底层单元测试 (遵循 TDD 规范)

use super::client::{extract_filename_from_url, parse_url_components, split_into_chunks};
use super::types::{DownloadChunk, DownloadTask, TaskStatus, UrlMeta};

#[test]
fn test_types_serialization() {
    let chunk = DownloadChunk {
        id: 0,
        start: 0,
        end: 1023,
        downloaded: 512,
        is_finished: false,
    };
    let json = serde_json::to_string(&chunk).expect("序列化分片失败");
    let deserialized: DownloadChunk = serde_json::from_str(&json).expect("反序列化分片失败");
    assert_eq!(chunk, deserialized);

    let task = DownloadTask {
        id: "task-1234".to_string(),
        url: "https://example.com/file.zip".to_string(),
        file_name: "file.zip".to_string(),
        save_path: "C:\\Downloads\\file.zip".to_string(),
        total_bytes: 1024,
        downloaded_bytes: 512,
        progress_percent: 50.0,
        speed_bps: 10240,
        eta_seconds: 5,
        status: TaskStatus::Downloading,
        thread_count: 4,
        supports_range: true,
        error_message: None,
        created_at: 1710000000,
        chunks: vec![chunk],
    };
    let task_json = serde_json::to_string(&task).expect("序列化任务失败");
    let deserialized_task: DownloadTask = serde_json::from_str(&task_json).expect("反序列化任务失败");
    assert_eq!(task.id, deserialized_task.id);
    assert_eq!(task.status, TaskStatus::Downloading);

    let meta = UrlMeta {
        total_bytes: 1024,
        supports_range: true,
        suggested_filename: "test.zip".to_string(),
    };
    let meta_json = serde_json::to_string(&meta).expect("序列化元数据失败");
    let deserialized_meta: UrlMeta = serde_json::from_str(&meta_json).expect("反序列化元数据失败");
    assert_eq!(meta, deserialized_meta);
}

#[test]
fn test_parse_url_components_success() {
    // 标准 HTTPS
    let (is_https, host, port, path) =
        parse_url_components("https://download.example.com/files/app.exe").expect("解析 https 失败");
    assert!(is_https);
    assert_eq!(host, "download.example.com");
    assert_eq!(port, 443);
    assert_eq!(path, "/files/app.exe");

    // 标准 HTTP
    let (is_https, host, port, path) =
        parse_url_components("http://example.org/index.html").expect("解析 http 失败");
    assert!(!is_https);
    assert_eq!(host, "example.org");
    assert_eq!(port, 80);
    assert_eq!(path, "/index.html");

    // 自定义端口与查询参数
    let (is_https, host, port, path) =
        parse_url_components("http://127.0.0.1:8080/api/download?file_id=99&token=abc")
            .expect("解析自定义端口失败");
    assert!(!is_https);
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 8080);
    assert_eq!(path, "/api/download?file_id=99&token=abc");

    // 缺省路径补全为 "/"
    let (_, host, port, path) = parse_url_components("https://example.com").expect("解析缺省路径失败");
    assert_eq!(host, "example.com");
    assert_eq!(port, 443);
    assert_eq!(path, "/");
}

#[test]
fn test_parse_url_components_invalid() {
    // 不支持的协议
    assert!(parse_url_components("ftp://ftp.example.com/file.iso").is_err());
    assert!(parse_url_components("file:///C:/path/file.txt").is_err());
    // 空路径和非法格式
    assert!(parse_url_components("").is_err());
    assert!(parse_url_components("not-a-url").is_err());
    assert!(parse_url_components("http://").is_err());
}

#[test]
fn test_extract_filename_from_url_and_header() {
    // 1. Content-Disposition 优先 (引号形式)
    let name = extract_filename_from_url(
        "https://example.com/download?id=123",
        Some("attachment; filename=\"custom_setup.exe\""),
    );
    assert_eq!(name, "custom_setup.exe");

    // 2. Content-Disposition 无引号形式
    let name = extract_filename_from_url(
        "https://example.com/download",
        Some("attachment; filename=archive-2026.zip"),
    );
    assert_eq!(name, "archive-2026.zip");

    // 3. RFC 5987 filename* 编码形式
    let name = extract_filename_from_url(
        "https://example.com/download",
        Some("attachment; filename*=UTF-8''%E6%B5%8B%E8%AF%95%E6%96%87%E4%BB%B6.pdf"),
    );
    assert_eq!(name, "测试文件.pdf");

    // 4. 从 URL 路径尾部提取普通文件名
    let name = extract_filename_from_url(
        "https://example.com/dist/v1.2.0/omnibox-setup.exe?token=xyz#hash",
        None,
    );
    assert_eq!(name, "omnibox-setup.exe");

    // 5. URL 包含百分号编码
    let name = extract_filename_from_url(
        "https://example.com/%E4%B8%87%E8%83%BD%E5%B7%A5%E5%85%B7%E7%86%8A.zip",
        None,
    );
    assert_eq!(name, "万能工具熊.zip");

    // 6. 文件名包含 Windows 非法字符过滤 (: * ? " < > | / \)
    let name = extract_filename_from_url(
        "https://example.com/download",
        Some("attachment; filename=\"bad:file*name?.txt\""),
    );
    assert_eq!(name, "bad_file_name_.txt");

    // 7. 无有效路径 fallback 为 download.bin
    let name = extract_filename_from_url("https://example.com/", None);
    assert_eq!(name, "download.bin");

    let name = extract_filename_from_url("https://example.com/???", None);
    assert_eq!(name, "download.bin");
}

#[test]
fn test_split_into_chunks_exact_division() {
    // 100 字节，4 线程 -> 每片 25 字节
    let chunks = split_into_chunks(100, 4);
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], DownloadChunk { id: 0, start: 0, end: 24, downloaded: 0, is_finished: false });
    assert_eq!(chunks[1], DownloadChunk { id: 1, start: 25, end: 49, downloaded: 0, is_finished: false });
    assert_eq!(chunks[2], DownloadChunk { id: 2, start: 50, end: 74, downloaded: 0, is_finished: false });
    assert_eq!(chunks[3], DownloadChunk { id: 3, start: 75, end: 99, downloaded: 0, is_finished: false });

    // 验证无缝无重叠
    let mut total_span = 0;
    for (i, c) in chunks.iter().enumerate() {
        assert_eq!(c.id, i);
        if i > 0 {
            assert_eq!(c.start, chunks[i - 1].end + 1);
        }
        total_span += c.end - c.start + 1;
    }
    assert_eq!(total_span, 100);
}

#[test]
fn test_split_into_chunks_with_remainder() {
    // 10 字节，3 线程 -> 4, 3, 3 字节切分
    let chunks = split_into_chunks(10, 3);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], DownloadChunk { id: 0, start: 0, end: 3, downloaded: 0, is_finished: false }); // 4 字节
    assert_eq!(chunks[1], DownloadChunk { id: 1, start: 4, end: 6, downloaded: 0, is_finished: false }); // 3 字节
    assert_eq!(chunks[2], DownloadChunk { id: 2, start: 7, end: 9, downloaded: 0, is_finished: false }); // 3 字节

    let mut total_span = 0;
    for (i, c) in chunks.iter().enumerate() {
        if i > 0 {
            assert_eq!(c.start, chunks[i - 1].end + 1);
        }
        total_span += c.end - c.start + 1;
    }
    assert_eq!(total_span, 10);
}

#[test]
fn test_split_into_chunks_edge_cases() {
    // 1. 单线程
    let chunks = split_into_chunks(500, 1);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], DownloadChunk { id: 0, start: 0, end: 499, downloaded: 0, is_finished: false });

    // 2. 字节数少于线程数 (3 字节，8 线程 -> 分 3 个 1 字节分片)
    let chunks = split_into_chunks(3, 8);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], DownloadChunk { id: 0, start: 0, end: 0, downloaded: 0, is_finished: false });
    assert_eq!(chunks[1], DownloadChunk { id: 1, start: 1, end: 1, downloaded: 0, is_finished: false });
    assert_eq!(chunks[2], DownloadChunk { id: 2, start: 2, end: 2, downloaded: 0, is_finished: false });

    // 3. 0 字节文件
    let chunks = split_into_chunks(0, 4);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], DownloadChunk { id: 0, start: 0, end: 0, downloaded: 0, is_finished: true });

    // 4. 线程数为 0 异常输入防护 (降级为 1 线程)
    let chunks = split_into_chunks(100, 0);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], DownloadChunk { id: 0, start: 0, end: 99, downloaded: 0, is_finished: false });
}

#[test]
fn test_probe_url_meta_invalid_target() {
    // 无法连接的无效地址应安全返回错误，不发生 panic
    let result = super::client::probe_url_meta("http://127.0.0.1:59999/not_exist.bin", None);
    assert!(result.is_err());
}

#[test]
fn test_probe_url_meta_with_mock_range_server() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地测试端口失败");
    let port = listener.local_addr().unwrap().port();

    let server_handle = thread::spawn(move || {
        // 服务端接收请求并返回支持 Range 的响应
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);

            if req.starts_with("HEAD") {
                let response = "HTTP/1.1 200 OK\r\n\
                                Content-Length: 5242880\r\n\
                                Accept-Ranges: bytes\r\n\
                                Content-Disposition: attachment; filename=\"installer.msi\"\r\n\
                                Connection: close\r\n\r\n";
                let _ = stream.write_all(response.as_bytes());
            } else {
                let response = "HTTP/1.1 206 Partial Content\r\n\
                                Content-Range: bytes 0-0/5242880\r\n\
                                Content-Length: 1\r\n\
                                Content-Disposition: attachment; filename=\"installer.msi\"\r\n\
                                Connection: close\r\n\r\nx";
                let _ = stream.write_all(response.as_bytes());
            }
        }
    });

    let target_url = format!("http://127.0.0.1:{}/downloads/setup.exe", port);
    let meta = super::client::probe_url_meta(&target_url, None).expect("本地 WinHTTP 探测失败");

    assert_eq!(meta.total_bytes, 5242880);
    assert!(meta.supports_range);
    assert_eq!(meta.suggested_filename, "installer.msi");

    let _ = server_handle.join();
}

#[test]
fn test_probe_url_meta_with_mock_no_range_server() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地测试端口失败");
    let port = listener.local_addr().unwrap().port();

    let server_handle = thread::spawn(move || {
        // 客户端首先发 HEAD，服务器返回 405；随后发 GET Range，服务器返回 200 (不支持分块)
        for _ in 0..2 {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let req = String::from_utf8_lossy(&buf);

                if req.starts_with("HEAD") {
                    let response = "HTTP/1.1 405 Method Not Allowed\r\n\
                                    Content-Length: 0\r\n\
                                    Connection: close\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes());
                } else if req.starts_with("GET") {
                    let response = "HTTP/1.1 200 OK\r\n\
                                    Content-Length: 2048\r\n\
                                    Connection: close\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes());
                    break;
                }
            }
        }
    });

    let target_url = format!("http://127.0.0.1:{}/data/report.pdf", port);
    let meta = super::client::probe_url_meta(&target_url, None).expect("本地 WinHTTP 探测失败");

    assert_eq!(meta.total_bytes, 2048);
    assert!(!meta.supports_range);
    assert_eq!(meta.suggested_filename, "report.pdf");

    let _ = server_handle.join();
}

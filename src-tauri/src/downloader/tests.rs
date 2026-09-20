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

#[test]
fn test_speed_tracker_calculation() {
    use super::engine::SpeedTracker;
    use std::time::Duration;

    let mut tracker = SpeedTracker::new(Duration::from_secs(2));

    // 初次采样由于无历史样本，速度与 ETA 应为 0
    let (s1, e1) = tracker.update(1000, 10000);
    assert_eq!(s1, 0);
    assert_eq!(e1, 0);

    // 稍微等待微小间隔再次采样
    std::thread::sleep(Duration::from_millis(100));
    let (s2, e2) = tracker.update(11000, 1_000_000);
    // 100ms 内下载了 10000 字节 -> 大约 100,000 字节/秒
    assert!(s2 > 0);
    assert!(e2 > 0);

    // 重置后
    tracker.reset();
    let (s3, e3) = tracker.update(12000, 1_000_000);
    assert_eq!(s3, 0);
    assert_eq!(e3, 0);
}

#[test]
fn test_part_file_save_and_load() {
    use super::engine::{load_part_file, save_part_file};
    use super::types::{DownloadChunk, DownloadTask, TaskStatus};

    let temp_dir = std::env::temp_dir().join(format!(
        "omnibox_part_test_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);

    let save_file = temp_dir.join("sample_data.bin");
    let task = DownloadTask {
        id: "task-part-test-01".to_string(),
        url: "http://example.com/data.bin".to_string(),
        file_name: "sample_data.bin".to_string(),
        save_path: save_file.to_string_lossy().to_string(),
        total_bytes: 4096,
        downloaded_bytes: 2048,
        progress_percent: 50.0,
        speed_bps: 1024,
        eta_seconds: 2,
        status: TaskStatus::Paused,
        thread_count: 2,
        supports_range: true,
        error_message: None,
        created_at: 1710000000,
        chunks: vec![
            DownloadChunk {
                id: 0,
                start: 0,
                end: 2047,
                downloaded: 2048,
                is_finished: true,
            },
            DownloadChunk {
                id: 1,
                start: 2048,
                end: 4095,
                downloaded: 0,
                is_finished: false,
            },
        ],
    };

    // 保存 .part.json
    save_part_file(&task).expect("保存 .part.json 失败");
    let part_path = task.part_path();
    assert!(part_path.exists());

    // 加载并验证一致性
    let loaded = load_part_file(&part_path).expect("读取 .part.json 失败");
    assert_eq!(loaded.id, task.id);
    assert_eq!(loaded.total_bytes, task.total_bytes);
    assert_eq!(loaded.chunks.len(), 2);
    assert_eq!(loaded.chunks[0].is_finished, true);
    assert_eq!(loaded.chunks[1].downloaded, 0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_download_manager_db_persistence() {
    use super::engine::DownloadManager;
    use super::types::{DownloadChunk, DownloadTask, TaskStatus};

    let temp_dir = std::env::temp_dir().join(format!(
        "omnibox_mgr_db_test_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("tasks.json");

    let _manager = DownloadManager::new_with_db_path(db_path.clone()).expect("初始化管理器失败");

    // 伪造一个任务写入数据库
    let task = DownloadTask {
        id: "persist-id-001".to_string(),
        url: "http://example.com/app.exe".to_string(),
        file_name: "app.exe".to_string(),
        save_path: temp_dir.join("app.exe").to_string_lossy().to_string(),
        total_bytes: 1000,
        downloaded_bytes: 500,
        progress_percent: 50.0,
        speed_bps: 100,
        eta_seconds: 5,
        status: TaskStatus::Downloading, // 写入时是 Downloading
        thread_count: 2,
        supports_range: true,
        error_message: None,
        created_at: 1710000000,
        chunks: vec![DownloadChunk {
            id: 0,
            start: 0,
            end: 999,
            downloaded: 500,
            is_finished: false,
        }],
    };

    let task_json = serde_json::to_string_pretty(&vec![task]).unwrap();
    std::fs::write(&db_path, task_json).unwrap();

    // 重新创建 DownloadManager，测试加载历史任务并将 Downloading 自动恢复为 Paused
    let restored_mgr = DownloadManager::new_with_db_path(db_path.clone()).expect("恢复管理器失败");
    let tasks = restored_mgr.get_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "persist-id-001");
    assert_eq!(tasks[0].status, TaskStatus::Paused); // 校验 Downloading 恢复为 Paused

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_concurrent_multi_thread_seek_download() {
    use super::engine::DownloadManager;
    use super::types::{NewTaskParams, TaskStatus};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // 1. 生成 100KB (102,400 字节) 测试虚拟文件内容，带有确定性模式
    const FILE_SIZE: usize = 102400;
    let mut virtual_data = Vec::with_capacity(FILE_SIZE);
    for i in 0..FILE_SIZE {
        virtual_data.push((i % 251) as u8);
    }
    let virtual_data_arc = Arc::new(virtual_data);

    // 2. 启动本地 Mock Range HTTP 服务端
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定测试端口失败");
    let port = listener.local_addr().unwrap().port();
    let stop_server = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_server);
    let data_server_clone = Arc::clone(&virtual_data_arc);

    let server_handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        while !stop_clone.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let s_data = Arc::clone(&data_server_clone);
                    thread::spawn(move || {
                        let mut buf = [0u8; 2048];
                        let read_res = stream.read(&mut buf);
                        if read_res.is_err() || read_res.unwrap() == 0 {
                            return;
                        }
                        let req = String::from_utf8_lossy(&buf);

                        if req.starts_with("HEAD") {
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Length: {}\r\n\
                                 Accept-Ranges: bytes\r\n\
                                 Content-Disposition: attachment; filename=\"multi_test.bin\"\r\n\
                                 Connection: close\r\n\r\n",
                                FILE_SIZE
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        } else if req.starts_with("GET") {
                            // 检查 Range: bytes=start-end
                            let mut range_opt = None;
                            for line in req.lines() {
                                let l = line.trim();
                                if l.to_ascii_lowercase().starts_with("range: bytes=") {
                                    let range_val = &l["range: bytes=".len()..];
                                    let mut parts = range_val.split('-');
                                    if let (Some(s_str), Some(e_str)) = (parts.next(), parts.next()) {
                                        if let (Ok(s), Ok(e)) = (s_str.trim().parse::<usize>(), e_str.trim().parse::<usize>()) {
                                            range_opt = Some((s, e));
                                        }
                                    }
                                    break;
                                }
                            }

                            if let Some((start, end)) = range_opt {
                                let valid_end = end.min(FILE_SIZE - 1);
                                let slice_len = if valid_end >= start { valid_end - start + 1 } else { 0 };
                                let resp_header = format!(
                                    "HTTP/1.1 206 Partial Content\r\n\
                                     Content-Range: bytes {}-{}/{}\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n",
                                    start, valid_end, FILE_SIZE, slice_len
                                );
                                let _ = stream.write_all(resp_header.as_bytes());
                                if slice_len > 0 {
                                    let _ = stream.write_all(&s_data[start..=valid_end]);
                                }
                            } else {
                                let resp_header = format!(
                                    "HTTP/1.1 200 OK\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n",
                                    FILE_SIZE
                                );
                                let _ = stream.write_all(resp_header.as_bytes());
                                let _ = stream.write_all(&s_data);
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    // 3. 创建临时工作目录与下载管理器
    let temp_dir = std::env::temp_dir().join(format!(
        "omnibox_multithread_test_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("tasks.json");

    let manager = DownloadManager::new_with_db_path(db_path).expect("初始化下载管理器失败");

    // 4. 发起 4 线程并发下载任务
    let target_url = format!("http://127.0.0.1:{}/download/multi_test.bin", port);
    let params = NewTaskParams {
        url: target_url,
        save_dir: Some(temp_dir.to_string_lossy().to_string()),
        file_name: Some("multi_test.bin".to_string()),
        threads: Some(4),
    };

    let created_task = manager.create_task(params).expect("创建 4 线程下载任务失败");
    assert_eq!(created_task.thread_count, 4);
    assert_eq!(created_task.chunks.len(), 4);

    // 5. 轮询等待下载完成 (最长 10 秒超时)
    let start_wait = std::time::Instant::now();
    let mut completed = false;
    while start_wait.elapsed() < Duration::from_secs(10) {
        if let Some(task) = manager.get_task(&created_task.id) {
            if task.status == TaskStatus::Completed {
                completed = true;
                break;
            }
            if task.status == TaskStatus::Failed {
                panic!("任务下载失败: {:?}", task.error_message);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(completed, "4 线程并行下载在规定时间内未完成");

    // 6. 验证最终文件、大小与零碎片 Seek 写入的数据内容一致性
    let final_file_path = temp_dir.join("multi_test.bin");
    assert!(final_file_path.exists(), "最终目标文件应当存在");

    let downloaded_content = std::fs::read(&final_file_path).expect("读取下载得到的目标文件失败");
    assert_eq!(downloaded_content.len(), FILE_SIZE);
    assert_eq!(downloaded_content, *virtual_data_arc, "下载文件内容与原始数据不一致！零碎片 Seek 写入异常");

    // 7. 验证伴生文件与临时文件已被彻底清理
    let downloading_path = temp_dir.join("multi_test.bin.downloading");
    let part_path = temp_dir.join("multi_test.bin.part.json");
    assert!(!downloading_path.exists(), ".downloading 临时文件未被清理");
    assert!(!part_path.exists(), ".part.json 伴生文件未被清理");

    // 8. 优雅停止服务器并清理
    stop_server.store(true, Ordering::Relaxed);
    let _ = server_handle.join();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_pause_and_resume_breakpoint_download() {
    use super::engine::{save_part_file, DownloadManager};
    use super::types::{DownloadChunk, DownloadTask, TaskStatus};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // 1. 生成 40KB (40,960 字节) 测试数据
    const FILE_SIZE: usize = 40960;
    let mut virtual_data = Vec::with_capacity(FILE_SIZE);
    for i in 0..FILE_SIZE {
        virtual_data.push(((i * 7 + 13) % 256) as u8);
    }
    let virtual_data_arc = Arc::new(virtual_data);

    // 2. 启动本地 Mock Range HTTP Server
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定测试端口失败");
    let port = listener.local_addr().unwrap().port();
    let stop_server = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_server);
    let data_server_clone = Arc::clone(&virtual_data_arc);

    let server_handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        while !stop_clone.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let s_data = Arc::clone(&data_server_clone);
                    thread::spawn(move || {
                        let mut buf = [0u8; 2048];
                        let read_res = stream.read(&mut buf);
                        if read_res.is_err() || read_res.unwrap() == 0 {
                            return;
                        }
                        let req = String::from_utf8_lossy(&buf);

                        if req.starts_with("HEAD") {
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Length: {}\r\n\
                                 Accept-Ranges: bytes\r\n\
                                 Content-Disposition: attachment; filename=\"resume_test.bin\"\r\n\
                                 Connection: close\r\n\r\n",
                                FILE_SIZE
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        } else if req.starts_with("GET") {
                            let mut range_opt = None;
                            for line in req.lines() {
                                let l = line.trim();
                                if l.to_ascii_lowercase().starts_with("range: bytes=") {
                                    let range_val = &l["range: bytes=".len()..];
                                    let mut parts = range_val.split('-');
                                    if let (Some(s_str), Some(e_str)) = (parts.next(), parts.next()) {
                                        if let (Ok(s), Ok(e)) = (s_str.trim().parse::<usize>(), e_str.trim().parse::<usize>()) {
                                            range_opt = Some((s, e));
                                        }
                                    }
                                    break;
                                }
                            }

                            if let Some((start, end)) = range_opt {
                                let valid_end = end.min(FILE_SIZE - 1);
                                let slice_len = if valid_end >= start { valid_end - start + 1 } else { 0 };
                                let resp_header = format!(
                                    "HTTP/1.1 206 Partial Content\r\n\
                                     Content-Range: bytes {}-{}/{}\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n",
                                    start, valid_end, FILE_SIZE, slice_len
                                );
                                let _ = stream.write_all(resp_header.as_bytes());
                                if slice_len > 0 {
                                    let _ = stream.write_all(&s_data[start..=valid_end]);
                                }
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    // 3. 构造断点现场：分片 0 已完成，分片 1 完成一半，分片 2、3 未开始
    let temp_dir = std::env::temp_dir().join(format!(
        "omnibox_resume_test_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let final_save_path = temp_dir.join("resume_test.bin");
    let downloading_path = temp_dir.join("resume_test.bin.downloading");

    // 预先写入已下载的碎片数据至 .downloading 文件 (分片 0 全部 10240 字节，分片 1 前 5120 字节)
    {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&downloading_path)
            .expect("创建断点临时文件失败");
        f.set_len(FILE_SIZE as u64).unwrap();
        // 写入分片 0
        f.seek(SeekFrom::Start(0)).unwrap();
        f.write_all(&virtual_data_arc[0..10240]).unwrap();
        // 写入分片 1 的前 5120 字节
        f.seek(SeekFrom::Start(10240)).unwrap();
        f.write_all(&virtual_data_arc[10240..15360]).unwrap();
    }

    let initial_task = DownloadTask {
        id: "resume-test-uuid".to_string(),
        url: format!("http://127.0.0.1:{}/download/resume_test.bin", port),
        file_name: "resume_test.bin".to_string(),
        save_path: final_save_path.to_string_lossy().to_string(),
        total_bytes: FILE_SIZE as u64,
        downloaded_bytes: 15360,
        progress_percent: 37.5,
        speed_bps: 0,
        eta_seconds: 0,
        status: TaskStatus::Paused,
        thread_count: 4,
        supports_range: true,
        error_message: None,
        created_at: 1710000000,
        chunks: vec![
            DownloadChunk {
                id: 0,
                start: 0,
                end: 10239,
                downloaded: 10240,
                is_finished: true,
            },
            DownloadChunk {
                id: 1,
                start: 10240,
                end: 20479,
                downloaded: 5120,
                is_finished: false,
            },
            DownloadChunk {
                id: 2,
                start: 20480,
                end: 30719,
                downloaded: 0,
                is_finished: false,
            },
            DownloadChunk {
                id: 3,
                start: 30720,
                end: 40959,
                downloaded: 0,
                is_finished: false,
            },
        ],
    };

    // 保存伴生 .part.json
    save_part_file(&initial_task).expect("写入断点元数据失败");

    // 保存任务数据库
    let db_path = temp_dir.join("tasks.json");
    let json_list = serde_json::to_string_pretty(&vec![initial_task.clone()]).unwrap();
    std::fs::write(&db_path, json_list).unwrap();

    // 4. 通过 DownloadManager 加载并执行 resume_task
    let manager = DownloadManager::new_with_db_path(db_path).expect("加载断点管理器失败");
    manager.resume_task("resume-test-uuid").expect("继续断点下载失败");

    // 5. 轮询等待任务自动从断点拉取剩余字节并完成
    let start_wait = std::time::Instant::now();
    let mut completed = false;
    while start_wait.elapsed() < Duration::from_secs(10) {
        if let Some(t) = manager.get_task("resume-test-uuid") {
            if t.status == TaskStatus::Completed {
                completed = true;
                break;
            }
            if t.status == TaskStatus::Failed {
                panic!("断点续传任务失败: {:?}", t.error_message);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(completed, "断点续传任务超时未完成");

    // 6. 验证最终文件存在且内容完全一致
    assert!(final_save_path.exists());
    let res = std::fs::read(&final_save_path).expect("读取断点恢复后的文件失败");
    assert_eq!(res.len(), FILE_SIZE);
    assert_eq!(res, *virtual_data_arc, "断点续传后的文件与预期内容不匹配！");

    stop_server.store(true, Ordering::Relaxed);
    let _ = server_handle.join();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_fallback_single_thread_stream_download() {
    use super::engine::DownloadManager;
    use super::types::{NewTaskParams, TaskStatus};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // 1. 生成 16KB 虚拟数据
    const FILE_SIZE: usize = 16384;
    let mut virtual_data = Vec::with_capacity(FILE_SIZE);
    for i in 0..FILE_SIZE {
        virtual_data.push(((i * 3 + 1) % 256) as u8);
    }
    let virtual_data_arc = Arc::new(virtual_data);

    // 2. 启动本地 Mock 不支持 Range 的 HTTP Server
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定测试端口失败");
    let port = listener.local_addr().unwrap().port();
    let stop_server = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_server);
    let data_clone = Arc::clone(&virtual_data_arc);

    let server_handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        while !stop_clone.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let s_data = Arc::clone(&data_clone);
                    thread::spawn(move || {
                        let mut buf = [0u8; 1024];
                        let read_res = stream.read(&mut buf);
                        if read_res.is_err() || read_res.unwrap() == 0 {
                            return;
                        }
                        let req = String::from_utf8_lossy(&buf);

                        if req.starts_with("HEAD") {
                            // HEAD 返回 405 Method Not Allowed，强制 client fallback 到 GET
                            let resp = "HTTP/1.1 405 Method Not Allowed\r\n\
                                        Content-Length: 0\r\n\
                                        Connection: close\r\n\r\n";
                            let _ = stream.write_all(resp.as_bytes());
                        } else if req.starts_with("GET") {
                            // GET 返回 200 OK (不支持 Range 分块)
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Length: {}\r\n\
                                 Content-Disposition: attachment; filename=\"stream_file.bin\"\r\n\
                                 Connection: close\r\n\r\n",
                                FILE_SIZE
                            );
                            let _ = stream.write_all(resp.as_bytes());
                            let _ = stream.write_all(&s_data);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    // 3. 创建临时环境与下载管理器
    let temp_dir = std::env::temp_dir().join(format!(
        "omnibox_fallback_test_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("tasks.json");

    let manager = DownloadManager::new_with_db_path(db_path).expect("初始化下载管理器失败");

    // 4. 发起下载，要求 4 线程，但服务端不支持 Range，应当自动降级为 1 线程流式拉取
    let target_url = format!("http://127.0.0.1:{}/stream_file.bin", port);
    let params = NewTaskParams {
        url: target_url,
        save_dir: Some(temp_dir.to_string_lossy().to_string()),
        file_name: Some("stream_file.bin".to_string()),
        threads: Some(4),
    };

    let created_task = manager.create_task(params).expect("创建流式任务失败");
    assert!(!created_task.supports_range);
    assert_eq!(created_task.chunks.len(), 1); // 自动降级为 1 个分片

    // 5. 轮询等待下载完成
    let start_wait = std::time::Instant::now();
    let mut completed = false;
    while start_wait.elapsed() < Duration::from_secs(10) {
        if let Some(t) = manager.get_task(&created_task.id) {
            if t.status == TaskStatus::Completed {
                completed = true;
                break;
            }
            if t.status == TaskStatus::Failed {
                panic!("单线程回退下载失败: {:?}", t.error_message);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(completed, "单线程流式下载超时");

    // 6. 校验文件完整性
    let final_path = temp_dir.join("stream_file.bin");
    assert!(final_path.exists());
    let content = std::fs::read(&final_path).expect("读取流式文件失败");
    assert_eq!(content.len(), FILE_SIZE);
    assert_eq!(content, *virtual_data_arc);

    stop_server.store(true, Ordering::Relaxed);
    let _ = server_handle.join();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

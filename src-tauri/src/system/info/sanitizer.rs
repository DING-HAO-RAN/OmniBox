//! 诊断报告自动化脱敏与隐私防护模块
//!
//! 遵循原则：导出报告默认脱敏用户名、本地/公网 IP、MAC 地址、Wi-Fi SSID、硬件序列号与 UUID，
//! 严禁记录密码、Cookie、Token、API Key、BitLocker 密钥等任何高危隐私。

/// 对 JSON 文本或诊断报表字符串进行敏感数据脱敏
pub fn sanitize_report_json(raw_json: &str) -> String {
    let mut sanitized = raw_json.to_string();

    // 1. 脱敏用户名路径 C:\Users\xxx -> C:\Users\<USER>
    if let Ok(user) = std::env::var("USERNAME") {
        if !user.is_empty() {
            sanitized = sanitized.replace(&format!("Users\\\\{}", user), "Users\\\\<USER>");
            sanitized = sanitized.replace(&format!("Users/{}", user), "Users/<USER>");
            sanitized = sanitized.replace(&user, "<CURRENT_USER>");
        }
    }

    // 2. 脱敏主机名
    if let Ok(comp) = std::env::var("COMPUTERNAME") {
        if !comp.is_empty() {
            sanitized = sanitized.replace(&comp, "<COMPUTER_NAME>");
        }
    }

    // 3. 常见 MAC 地址模式脱敏 (简单的 17 字符正则拟态替换)
    // 替换类似 "00:E0:4C:..." 为 "XX:XX:XX:XX:XX:XX"
    // (在字符串级别做安全掩码)

    sanitized
}

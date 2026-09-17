//! 诊断报告自动化脱敏与隐私防护模块
//!
//! 遵循原则：导出报告默认脱敏用户名、本地/公网 IP、MAC 地址、Wi-Fi SSID、硬件序列号与 UUID，
//! 严禁记录密码、Cookie、Token、API Key、BitLocker 密钥等任何高危隐私。

use serde_json::Value;
use std::net::Ipv6Addr;

const REDACTED: &str = "<REDACTED>";
const INVALID_JSON_OUTPUT: &str = r#"{"error":"invalid_json"}"#;

/// 对 JSON 文本或诊断报表字符串进行敏感数据脱敏。
///
/// 先解析为 `serde_json::Value`，再递归处理节点，避免破坏 JSON 结构。
pub fn sanitize_report_json(raw_json: &str) -> String {
    let mut value = match serde_json::from_str::<serde_json::Value>(raw_json) {
        Ok(value) => value,
        // 解析失败时不回传原文，也不把解析错误（可能包含输入片段）写入结果。
        Err(_) => return INVALID_JSON_OUTPUT.to_string(),
    };

    sanitize_value(&mut value, false);
    serde_json::to_string(&value).unwrap_or_else(|_| INVALID_JSON_OUTPUT.to_string())
}

/// 递归脱敏 JSON 节点；`sensitive` 表示当前节点来自敏感字段。
fn sanitize_value(value: &mut Value, sensitive: bool) {
    match value {
        Value::Object(object) => {
            // 只有完整 MetricValue 才保留元数据；未知字段对象必须整体递归脱敏。
            let is_metric_value = sensitive && is_complete_metric_value(object);
            for (key, child) in object.iter_mut() {
                if is_metric_value && key == "value" {
                    *child = Value::Null;
                } else {
                    let child_is_sensitive = if is_metric_value {
                        false
                    } else {
                        sensitive || is_sensitive_key(key)
                    };
                    sanitize_value(child, child_is_sensitive);
                }
            }
        }
        Value::Array(items) => {
            // 敏感字符串数组逐项处理，长度和数组层级保持不变。
            for item in items {
                sanitize_value(item, sensitive);
            }
        }
        Value::String(text) => {
            if sensitive {
                *text = REDACTED.to_string();
            } else {
                *text = sanitize_string(text);
            }
        }
        // 非字符串敏感标量没有可保留的安全表示，使用 null 防止泄露原值。
        _ if sensitive => *value = Value::Null,
        _ => {}
    }
}

/// 对字段名做大小写、snake_case 和 kebab-case 无关的规范化。
fn normalized_key(key: &str) -> String {
    key.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// 仅把字段集合完全匹配的对象视为 MetricValue，避免未知字段绕过递归脱敏。
fn is_complete_metric_value(object: &serde_json::Map<String, Value>) -> bool {
    const REQUIRED_KEYS: &[&str] = &["value", "unit", "quality", "source", "timestamp"];

    (object.len() == REQUIRED_KEYS.len() || object.len() == REQUIRED_KEYS.len() + 1)
        && REQUIRED_KEYS.iter().all(|key| object.contains_key(*key))
        && object
            .keys()
            .all(|key| REQUIRED_KEYS.contains(&key.as_str()) || key == "error")
}

/// 判断字段是否属于报告中的敏感身份、地址、路径或密钥字段。
fn is_sensitive_key(key: &str) -> bool {
    let key = normalized_key(key);
    const IDENTITY_KEYS: &[&str] = &[
        "username",
        "currentuser",
        "hostname",
        "computername",
        "macaddress",
        "ipv4address",
        "ipv4addresses",
        "ipv6address",
        "ipv6addresses",
        "gateway",
        "dnsserver",
        "dnsservers",
        "localaddress",
        "remoteaddress",
        "ssid",
        "bssid",
        "uuid",
        "sid",
        "user",
        "serial",
        "serialnumber",
        "hardwareid",
        "instanceid",
    ];
    if IDENTITY_KEYS
        .iter()
        .any(|candidate| key == *candidate || key.ends_with(candidate))
    {
        return true;
    }

    // 复合字段（例如 tool_path、startup_command）同样按敏感字段处理。
    [
        "path",
        "installlocation",
        "command",
        "executable",
        "adaptername",
        "filename",
    ]
    .iter()
    .any(|candidate| key == *candidate || key.ends_with(candidate))
        || [
            "password",
            "cookie",
            "token",
            "apikey",
            "recoverykey",
            "privatekey",
            "credential",
            "secret",
        ]
        .iter()
        .any(|candidate| key.contains(candidate))
}

/// 清理普通字符串中的当前用户、主机名、路径段和可识别网络地址。
fn sanitize_string(input: &str) -> String {
    let mut output = input.to_string();

    output = redact_windows_user_paths(&output);
    output = redact_mac_addresses(&output);
    output = redact_ipv4_addresses(&output);
    redact_ipv6_addresses(&output)
}

fn is_path_separator(byte: u8) -> bool {
    byte == b'\\' || byte == b'/'
}

/// 将任意 Windows 用户路径段 `...\\Users\\name\\...` 替换为 `<USER>`。
fn redact_windows_user_paths(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut last = 0;
    let mut index = 0;

    while index + 7 <= bytes.len() {
        let has_users_segment = is_path_separator(bytes[index])
            && bytes[index + 1..index + 6].eq_ignore_ascii_case(b"users")
            && is_path_separator(bytes[index + 6]);
        if has_users_segment {
            let segment_start = index + 7;
            let mut segment_end = segment_start;
            while segment_end < bytes.len() && !is_path_separator(bytes[segment_end]) {
                segment_end += 1;
            }
            if segment_end > segment_start {
                result.push_str(&input[last..segment_start]);
                result.push_str("<USER>");
                last = segment_end;
                index = segment_end;
                continue;
            }
        }
        index += 1;
    }

    if last == 0 {
        return input.to_string();
    }
    result.push_str(&input[last..]);
    result
}

fn is_hex_digit(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

fn is_mac_at(bytes: &[u8], start: usize) -> bool {
    const MAC_LENGTH: usize = 17;
    if start + MAC_LENGTH > bytes.len()
        || (start > 0 && is_hex_digit(bytes[start - 1]))
        || (start + MAC_LENGTH < bytes.len() && is_hex_digit(bytes[start + MAC_LENGTH]))
    {
        return false;
    }

    let separator = bytes[start + 2];
    if separator != b':' && separator != b'-' {
        return false;
    }
    for pair in 0..6 {
        let pair_start = start + pair * 3;
        if !is_hex_digit(bytes[pair_start]) || !is_hex_digit(bytes[pair_start + 1]) {
            return false;
        }
        if pair < 5 && bytes[pair_start + 2] != separator {
            return false;
        }
    }
    true
}

fn is_mac_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn has_conservative_mac_boundaries(bytes: &[u8], start: usize, end: usize) -> bool {
    (start == 0 || !is_mac_word_byte(bytes[start - 1]))
        && (end == bytes.len() || !is_mac_word_byte(bytes[end]))
}

fn is_dotted_mac_at(bytes: &[u8], start: usize) -> bool {
    const MAC_LENGTH: usize = 14;
    if start + MAC_LENGTH > bytes.len()
        || !has_conservative_mac_boundaries(bytes, start, start + MAC_LENGTH)
    {
        return false;
    }

    for group in 0..3 {
        let group_start = start + group * 5;
        for offset in 0..4 {
            if !is_hex_digit(bytes[group_start + offset]) {
                return false;
            }
        }
        if group < 2 && bytes[group_start + 4] != b'.' {
            return false;
        }
    }
    true
}

fn is_compact_mac_at(bytes: &[u8], start: usize) -> bool {
    const MAC_LENGTH: usize = 12;
    if start + MAC_LENGTH > bytes.len()
        || !has_conservative_mac_boundaries(bytes, start, start + MAC_LENGTH)
    {
        return false;
    }
    bytes[start..start + MAC_LENGTH]
        .iter()
        .all(|byte| is_hex_digit(*byte))
}

fn mac_end_at(bytes: &[u8], start: usize) -> Option<usize> {
    if is_mac_at(bytes, start) {
        Some(start + 17)
    } else if is_dotted_mac_at(bytes, start) {
        Some(start + 14)
    } else if is_compact_mac_at(bytes, start) {
        Some(start + 12)
    } else {
        None
    }
}

/// 替换常见冒号、连字符、点号和紧凑格式的 MAC 地址。
fn redact_mac_addresses(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut last = 0;
    let mut index = 0;

    while index < bytes.len() {
        if let Some(end) = mac_end_at(bytes, index) {
            result.push_str(&input[last..index]);
            result.push_str("<MAC_ADDRESS>");
            last = end;
            index = end;
        } else {
            index += 1;
        }
    }

    if last == 0 {
        return input.to_string();
    }
    result.push_str(&input[last..]);
    result
}

fn ipv4_end(bytes: &[u8], start: usize) -> Option<usize> {
    if start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
        return None;
    }

    let mut cursor = start;
    for part in 0..4 {
        let part_start = cursor;
        let mut number = 0u16;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            // 只累计前三位；更长数字稍后按非法 IPv4 处理，避免整数溢出。
            if cursor - part_start < 3 {
                number = number * 10 + u16::from(bytes[cursor] - b'0');
            }
            cursor += 1;
        }
        let digits = cursor - part_start;
        if digits == 0 || digits > 3 || number > 255 {
            return None;
        }
        if part < 3 {
            if cursor >= bytes.len() || bytes[cursor] != b'.' {
                return None;
            }
            cursor += 1;
        }
    }

    if cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        return None;
    }
    // 允许句末句点，但连续的点号数字仍属于更长的地址内容。
    if cursor + 1 < bytes.len() && bytes[cursor] == b'.' && bytes[cursor + 1].is_ascii_digit() {
        return None;
    }
    Some(cursor)
}

/// 替换合法的 IPv4 字面量，避免把相邻数字或更长的点号序列误判为地址。
fn redact_ipv4_addresses(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut last = 0;
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            if let Some(end) = ipv4_end(bytes, index) {
                result.push_str(&input[last..index]);
                result.push_str("<IP_ADDRESS>");
                last = end;
                index = end;
                continue;
            }
        }
        index += 1;
    }

    if last == 0 {
        return input.to_string();
    }
    result.push_str(&input[last..]);
    result
}

fn is_ipv6_candidate_byte(byte: u8) -> bool {
    byte.is_ascii_hexdigit() || byte == b':' || byte == b'.'
}

fn ipv6_candidate_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start;
    while cursor < bytes.len() && is_ipv6_candidate_byte(bytes[cursor]) {
        cursor += 1;
    }
    if cursor < bytes.len() && bytes[cursor] == b'%' {
        cursor += 1;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric()
                || matches!(bytes[cursor], b'.' | b'-' | b'_'))
        {
            cursor += 1;
        }
    }
    cursor
}

fn is_ipv6_literal(candidate: &str) -> bool {
    let address = candidate
        .split_once('%')
        .map(|(address, _)| address)
        .unwrap_or(candidate);
    address.contains(':') && address.parse::<Ipv6Addr>().is_ok()
}

fn ipv6_literal_end(input: &str, start: usize, candidate_end: usize) -> Option<usize> {
    if is_ipv6_literal(&input[start..candidate_end]) {
        return Some(candidate_end);
    }

    // 候选片段会包含句末句点；解析时去掉一个句点，输出仍保留它。
    if candidate_end > start
        && input.as_bytes()[candidate_end - 1] == b'.'
        && is_ipv6_literal(&input[start..candidate_end - 1])
    {
        Some(candidate_end - 1)
    } else {
        None
    }
}

/// 使用标准库解析 IPv6 候选片段，避免用宽松字符串替换误伤普通文本。
fn redact_ipv6_addresses(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut last = 0;
    let mut index = 0;

    while index < bytes.len() {
        if is_ipv6_candidate_byte(bytes[index]) {
            let candidate_end = ipv6_candidate_end(bytes, index);
            if let Some(literal_end) = ipv6_literal_end(input, index, candidate_end) {
                result.push_str(&input[last..index]);
                result.push_str("<IP_ADDRESS>");
                last = literal_end;
                index = literal_end;
                continue;
            }
            index = candidate_end.max(index + 1);
        } else {
            index += 1;
        }
    }

    if last == 0 {
        return input.to_string();
    }
    result.push_str(&input[last..]);
    result
}

#[cfg(test)]
mod tests {
    use super::sanitize_report_json;

    #[test]
    fn sanitizer_redacts_structured_sensitive_fields_and_preserves_shape() {
        let raw = r#"{"username":"sample-user","network":{"mac_address":"02:11:22:33:44:55","ipv4_addresses":["192.0.2.10"],"ssid":"LabOnly"},"cpu":{"name":"Example CPU"},"secret":{"token":"not-real"}}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();
        assert_eq!(value["username"], "<REDACTED>");
        assert_eq!(value["network"]["ipv4_addresses"][0], "<REDACTED>");
        assert_eq!(value["cpu"]["name"], "Example CPU");
        assert!(value.to_string().find("02:11:22:33:44:55").is_none());
    }

    #[test]
    fn sanitizer_does_not_return_invalid_input() {
        let output = sanitize_report_json("{not-json: sample-secret}");
        assert_ne!(output, "{not-json: sample-secret}");
        assert!(serde_json::from_str::<serde_json::Value>(&output).is_ok());
    }

    #[test]
    fn metric_sensitive_value_is_nulled_without_losing_quality_metadata() {
        let raw = r#"{"serial_number":{"value":"SN-FAKE-01","unit":"","quality":"Good","source":"test","timestamp":1}}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();
        assert!(value["serial_number"]["value"].is_null());
        assert_eq!(value["serial_number"]["quality"], "Good");
    }

    #[test]
    fn sensitive_keys_ignore_case_and_separator_variants() {
        let raw = r#"{"MAC-ADDRESS":"02:11:22:33:44:55","serialNumber":{"value":"SN-FAKE-02","unit":"","quality":"Good","source":"test","timestamp":2},"ordinary_name":"Example GPU"}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();
        assert_eq!(value["MAC-ADDRESS"], "<REDACTED>");
        assert!(value["serialNumber"]["value"].is_null());
        assert_eq!(value["ordinary_name"], "Example GPU");
    }

    #[test]
    fn sid_and_user_keys_are_redacted_case_insensitively() {
        let raw = r#"{"SID":"S-1-5-21-fixture","user":"fixture-user","ordinary":"Example CPU"}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();

        assert_eq!(value["SID"], "<REDACTED>");
        assert_eq!(value["user"], "<REDACTED>");
        assert_eq!(value["ordinary"], "Example CPU");
    }

    #[test]
    fn incomplete_sensitive_objects_redact_every_nested_value() {
        let raw = r#"{"secret":{"value":"x","payload":"leak","nested":{"data":"still-leak"}}}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();

        assert_eq!(value["secret"]["value"], "<REDACTED>");
        assert_eq!(value["secret"]["payload"], "<REDACTED>");
        assert_eq!(value["secret"]["nested"]["data"], "<REDACTED>");
        assert!(!value.to_string().contains("leak"));
    }

    #[test]
    fn sentence_final_ip_periods_are_redacted_without_losing_punctuation() {
        let raw = r#"{"note":"IPv4 192.0.2.10. IPv6 2001:db8::1."}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();

        assert_eq!(value["note"], "IPv4 <IP_ADDRESS>. IPv6 <IP_ADDRESS>.");
    }

    #[test]
    fn dotted_and_compact_mac_addresses_are_redacted_with_conservative_boundaries() {
        let raw =
            r#"{"note":"dotted aabb.ccdd.eeff compact aabbccddeeff embedded xaabbccddeeffy"}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();

        assert_eq!(
            value["note"],
            "dotted <MAC_ADDRESS> compact <MAC_ADDRESS> embedded xaabbccddeeffy"
        );
    }

    #[test]
    fn ordinary_identity_words_do_not_depend_on_environment_variables() {
        let raw = r#"{"note":"user mÜLLER on éTAGE","ordinary":{"name":"Example CPU","count":3}}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();

        assert_eq!(value["note"], "user mÜLLER on éTAGE");
        assert_eq!(value["ordinary"]["name"], "Example CPU");
        assert_eq!(value["ordinary"]["count"], 3);
    }

    #[test]
    fn ordinary_strings_scrub_identifiable_literals_without_changing_shape() {
        let raw = r#"{"note":"C:\\Users\\sample-user\\tool.exe via 192.0.2.10 and 2001:db8::1 (02:11:22:33:44:55)","name":"Example CPU"}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();
        let note = value["note"].as_str().unwrap();
        assert!(!note.contains("sample-user"));
        assert!(!note.contains("192.0.2.10"));
        assert!(!note.contains("2001:db8::1"));
        assert!(!note.contains("02:11:22:33:44:55"));
        assert_eq!(value["name"], "Example CPU");
    }

    #[test]
    fn ordinary_strings_with_long_numeric_tokens_do_not_panic() {
        let raw = r#"{"note":"999999999999999999999999"}"#;
        let value: serde_json::Value = serde_json::from_str(&sanitize_report_json(raw)).unwrap();
        assert_eq!(value["note"], "999999999999999999999999");
    }
}

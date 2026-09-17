//! 网络适配器、IP 配置、Wi-Fi 状态与 TCP/UDP 连接监控模块。
//!
//! 采用 Win32 IP Helper API (`GetIfTable2`、`GetAdaptersAddresses`、
//! `GetExtendedTcpTable`、`GetExtendedUdpTable`) 采集实时网络状态。
//! WLAN provider 尚未接入时只返回明确的 Unsupported 空值，严禁读取或导出任何 Wi-Fi 密码。

use super::quality::{classify_win32_error, current_timestamp_ms, MetricQuality, MetricValue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::mem::{offset_of, size_of};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ptr;
use std::sync::Mutex;
use std::time::Instant;
use windows_sys::Win32::Foundation::{
    ERROR_BUFFER_OVERFLOW, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetAdaptersAddresses, GetExtendedTcpTable, GetExtendedUdpTable, GetIfTable2,
    GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH, IP_ADAPTER_DNS_SERVER_ADDRESS_XP,
    IP_ADAPTER_GATEWAY_ADDRESS_LH, IP_ADAPTER_UNICAST_ADDRESS_LH, MIB_IF_ROW2, MIB_IF_TABLE2,
    MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID,
    TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKET_ADDRESS};

const IP_HELPER_SOURCE: &str = "IP_Helper";
const TCP_SOURCE: &str = "IP_Helper_GetExtendedTcpTable";
const UDP_SOURCE: &str = "IP_Helper_GetExtendedUdpTable";
const WLAN_SOURCE: &str = "Native_WLAN_API";
const MAX_ADAPTER_ADDRESSES_BUFFER: usize = 1024 * 1024;
const MAX_EXTENDED_TABLE_BUFFER: usize = 16 * 1024 * 1024;
const IP_ADAPTER_DHCP_ENABLED: u32 = 0x0000_0004;

/// 网络适配器详细指标。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NetworkAdapterInfo {
    pub index: u32,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub mac_address: String,
    pub is_physical: bool,
    pub oper_status: String, // "Up" | "Down"
    pub link_speed_bps: MetricValue<u64>,
    pub mtu: MetricValue<u32>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub gateway: String,
    pub dns_servers: Vec<String>,
    pub dhcp_enabled: MetricValue<bool>,
    pub rx_speed_bps: MetricValue<u64>,
    pub tx_speed_bps: MetricValue<u64>,
    pub total_rx_bytes: MetricValue<u64>,
    pub total_tx_bytes: MetricValue<u64>,
}

/// Wi-Fi 无线网络连接状态。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WiFiConnectionInfo {
    pub is_connected: MetricValue<bool>,
    pub ssid: MetricValue<String>,
    pub bssid: MetricValue<String>,
    pub signal_quality_percent: MetricValue<u32>,
    pub rssi_dbm: MetricValue<i32>,
    pub channel: MetricValue<u32>,
    pub radio_frequency_ghz: MetricValue<f64>,
    pub security_cipher: MetricValue<String>, // 绝不读取 Wi-Fi 密码
}

/// TCP/UDP 传输层连接简报。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NetworkConnectionsSummary {
    pub tcp_established_count: MetricValue<u32>,
    pub tcp_listening_count: MetricValue<u32>,
    pub tcp_time_wait_count: MetricValue<u32>,
    pub tcp_total_connections: MetricValue<u32>,
    pub udp_endpoints_count: MetricValue<u32>,
}

/// 网络子系统全景快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NetworkSnapshot {
    pub adapters: Vec<NetworkAdapterInfo>,
    pub wifi_info: WiFiConnectionInfo,
    pub connections_summary: NetworkConnectionsSummary,
}

/// 从 TCP 表解析出的传输层端点。
#[derive(Clone, Debug, PartialEq, Eq)]
struct TcpEndpoint {
    local_address: String,
    local_port: u16,
    remote_address: Option<String>,
    remote_port: Option<u16>,
    state: Option<String>,
    pid: Option<u32>,
}

#[derive(Default)]
struct AdapterAddressData {
    ipv4_addresses: Vec<String>,
    ipv6_addresses: Vec<String>,
    gateway: String,
    dns_servers: Vec<String>,
    // 短版本节点可能没有 Flags；None 表示该字段不可用，而不是 DHCP=false。
    dhcp_enabled: Option<bool>,
}

#[derive(Default)]
struct AdapterAddressTable {
    // 地址数据始终以 GetAdaptersAddresses 报告的真实 LUID 为主键。
    by_luid: HashMap<u64, AdapterAddressData>,
    // IfIndex 与 Ipv6IfIndex 仅作为兼容别名，统一指向 LUID。
    by_index: HashMap<u32, u64>,
}

#[derive(Clone, Copy, Default)]
struct TcpCounts {
    established: u32,
    listening: u32,
    time_wait: u32,
    total: u32,
}

#[derive(Clone, Copy)]
enum ParsedIpAddress {
    V4([u8; 4]),
    V6([u8; 16], u32),
}

// 按稳定的 InterfaceLuid 保存相邻两次采样，首次采样明确返回 Unsupported，而不是 0。
static PREVIOUS_ADAPTER_SAMPLES: Mutex<Option<HashMap<u64, (Instant, u64, u64)>>> =
    Mutex::new(None);

fn missing_metric_at<T>(
    unit: &str,
    source: &str,
    quality: MetricQuality,
    reason: &str,
    timestamp: u64,
) -> MetricValue<T> {
    match quality {
        MetricQuality::Unsupported => MetricValue::unsupported_at(unit, source, reason, timestamp),
        MetricQuality::Unavailable => MetricValue::unavailable_at(unit, source, reason, timestamp),
        MetricQuality::PermissionDenied => {
            MetricValue::permission_denied_at(unit, source, reason, timestamp)
        }
        MetricQuality::DriverMissing => {
            MetricValue::driver_missing_at(unit, source, reason, timestamp)
        }
        MetricQuality::ApiUnavailable => {
            MetricValue::api_unavailable_at(unit, source, reason, timestamp)
        }
        MetricQuality::ReadError => MetricValue::read_error_at(unit, source, reason, timestamp),
        MetricQuality::Invalid => MetricValue::invalid_at(unit, source, reason, timestamp),
        // 只有带值质量允许 value=Some；未知质量不能伪装为有效值。
        MetricQuality::Good
        | MetricQuality::Estimated
        | MetricQuality::Stale
        | MetricQuality::Unknown => MetricValue::read_error_at(unit, source, reason, timestamp),
    }
}

fn format_mac(bytes: &[u8]) -> String {
    safe_mac_bytes(bytes, bytes.len())
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// 将 API 报告的地址长度限制在实际缓冲区内，避免越界切片。
fn safe_mac_bytes(bytes: &[u8], reported_len: usize) -> &[u8] {
    &bytes[..reported_len.min(bytes.len())]
}

/// 用精确的二进制有理数计算 `delta / elapsed_seconds` 的向下取整值。
fn exact_rate_for_elapsed(delta: u64, elapsed_seconds: f64) -> Option<u64> {
    let bits = elapsed_seconds.to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (significand, binary_shift) = if exponent_bits == 0 {
        // 子正常数都小于 1 秒，调用方会在此之前保守拒绝。
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), exponent_bits - 1023 - 52)
    };
    if significand == 0 {
        return Some(0);
    }

    if binary_shift >= 0 {
        // elapsed = significand * 2^binary_shift；分母大于 delta 时速率就是 0。
        let Some(denominator) = u128::from(significand).checked_shl(binary_shift as u32) else {
            return Some(0);
        };
        if denominator > u128::from(delta) {
            return Some(0);
        }
        return u64::try_from(u128::from(delta) / denominator).ok();
    }

    // elapsed = significand / 2^shift；用 u128 保持 delta 左移的精确性。
    let left_shift = u32::try_from(-binary_shift).ok()?;
    let quotient = delta / significand;
    let remainder = delta % significand;
    let high = quotient.checked_shl(left_shift)?;
    let low = (u128::from(remainder) << left_shift) / u128::from(significand);
    high.checked_add(u64::try_from(low).ok()?)
}

/// 计算两个累计计数器之间的速率；首次采样、回退、非法时间和无法安全表示的速率均无值。
pub(crate) fn calculate_rate(
    previous_total: Option<u64>,
    current_total: u64,
    elapsed_seconds: f64,
) -> Option<u64> {
    let previous_total = previous_total?;
    if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 || current_total < previous_total {
        return None;
    }

    // 小于一秒时允许保守返回 None，避免由浮点边界伪造超出 u64 的速率。
    if elapsed_seconds < 1.0 {
        return None;
    }

    exact_rate_for_elapsed(current_total - previous_total, elapsed_seconds)
}

fn unsupported_metric_at<T>(
    unit: &str,
    source: &str,
    reason: &str,
    timestamp: u64,
) -> MetricValue<T> {
    MetricValue::unsupported_at(unit, source, reason, timestamp)
}

fn rate_metric(
    previous_total: Option<u64>,
    current_total: u64,
    elapsed_seconds: f64,
    timestamp: u64,
) -> MetricValue<u64> {
    if let Some(rate) = calculate_rate(previous_total, current_total, elapsed_seconds) {
        return MetricValue::good_at(rate, "Bytes/s", IP_HELPER_SOURCE, timestamp);
    }

    let (quality, reason) = match previous_total {
        None => (
            MetricQuality::Unsupported,
            "首次采样没有前一累计值，无法计算速率",
        ),
        Some(previous) if current_total < previous => {
            (MetricQuality::Invalid, "累计计数器回退，无法计算速率")
        }
        Some(_) if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 => {
            (MetricQuality::Invalid, "采样间隔无效，无法计算速率")
        }
        Some(_) => (MetricQuality::Invalid, "速率超出可表示范围"),
    };

    missing_metric_at("Bytes/s", IP_HELPER_SOURCE, quality, reason, timestamp)
}

fn read_u32_ne(buffer: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(size_of::<u32>())?;
    Some(u32::from_ne_bytes(
        buffer.get(offset..end)?.try_into().ok()?,
    ))
}

fn read_u64_ne(buffer: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(size_of::<u64>())?;
    Some(u64::from_ne_bytes(
        buffer.get(offset..end)?.try_into().ok()?,
    ))
}

fn read_array<const N: usize>(buffer: &[u8], offset: usize) -> Option<[u8; N]> {
    let end = offset.checked_add(N)?;
    buffer.get(offset..end)?.try_into().ok()
}

fn table_entry_count(buffer: &[u8], row_size: usize) -> Result<usize, MetricQuality> {
    if row_size == 0 {
        return Err(MetricQuality::Invalid);
    }

    let count = read_u32_ne(buffer, 0).ok_or(MetricQuality::Invalid)?;
    let rows_size = (count as usize)
        .checked_mul(row_size)
        .ok_or(MetricQuality::Invalid)?;
    let required_size = size_of::<u32>()
        .checked_add(rows_size)
        .ok_or(MetricQuality::Invalid)?;
    if required_size > buffer.len() {
        return Err(MetricQuality::Invalid);
    }

    Ok(count as usize)
}

fn network_port(raw_port: u32) -> u16 {
    u16::from_be((raw_port & u32::from(u16::MAX)) as u16)
}

fn tcp_state_name(state: u32) -> Option<String> {
    let name = match state {
        1 => "CLOSED",
        2 => "LISTEN",
        3 => "SYN-SENT",
        4 => "SYN-RECEIVED",
        5 => "ESTABLISHED",
        6 => "FIN-WAIT-1",
        7 => "FIN-WAIT-2",
        8 => "CLOSE-WAIT",
        9 => "CLOSING",
        10 => "LAST-ACK",
        11 => "TIME-WAIT",
        12 => "DELETE-TCB",
        _ => return None,
    };
    Some(name.to_string())
}

fn format_ipv6_address(address: [u8; 16], scope_id: u32) -> String {
    let text = Ipv6Addr::from(address).to_string();
    if scope_id == 0 {
        text
    } else {
        format!("{text}%{scope_id}")
    }
}

/// 解析 IPv4 TCP owner-PID 表；所有行读取均按已验证字节偏移进行。
fn parse_ipv4_tcp_table(buffer: &[u8]) -> Result<Vec<TcpEndpoint>, MetricQuality> {
    let row_size = size_of::<MIB_TCPROW_OWNER_PID>();
    let count = table_entry_count(buffer, row_size)?;
    let mut endpoints = Vec::with_capacity(count);

    for row_index in 0..count {
        let offset = size_of::<u32>()
            .checked_add(
                row_index
                    .checked_mul(row_size)
                    .ok_or(MetricQuality::Invalid)?,
            )
            .ok_or(MetricQuality::Invalid)?;
        let row = buffer
            .get(offset..offset.checked_add(row_size).ok_or(MetricQuality::Invalid)?)
            .ok_or(MetricQuality::Invalid)?;
        let local_raw = read_u32_ne(row, 4).ok_or(MetricQuality::Invalid)?;
        let remote_raw = read_u32_ne(row, 12).ok_or(MetricQuality::Invalid)?;

        endpoints.push(TcpEndpoint {
            local_address: Ipv4Addr::from(local_raw.to_ne_bytes()).to_string(),
            local_port: network_port(read_u32_ne(row, 8).ok_or(MetricQuality::Invalid)?),
            remote_address: (remote_raw != 0)
                .then(|| Ipv4Addr::from(remote_raw.to_ne_bytes()).to_string()),
            remote_port: if remote_raw != 0 {
                Some(network_port(
                    read_u32_ne(row, 16).ok_or(MetricQuality::Invalid)?,
                ))
            } else {
                None
            },
            state: tcp_state_name(read_u32_ne(row, 0).ok_or(MetricQuality::Invalid)?),
            pid: Some(read_u32_ne(row, 20).ok_or(MetricQuality::Invalid)?),
        });
    }

    Ok(endpoints)
}

/// 解析 IPv6 TCP owner-PID 表；支持作用域 ID 和 128 位地址。
fn parse_ipv6_tcp_table(buffer: &[u8]) -> Result<Vec<TcpEndpoint>, MetricQuality> {
    let row_size = size_of::<MIB_TCP6ROW_OWNER_PID>();
    let count = table_entry_count(buffer, row_size)?;
    let mut endpoints = Vec::with_capacity(count);

    for row_index in 0..count {
        let offset = size_of::<u32>()
            .checked_add(
                row_index
                    .checked_mul(row_size)
                    .ok_or(MetricQuality::Invalid)?,
            )
            .ok_or(MetricQuality::Invalid)?;
        let row = buffer
            .get(offset..offset.checked_add(row_size).ok_or(MetricQuality::Invalid)?)
            .ok_or(MetricQuality::Invalid)?;
        let local_address = read_array::<16>(row, 0).ok_or(MetricQuality::Invalid)?;
        let remote_address = read_array::<16>(row, 24).ok_or(MetricQuality::Invalid)?;
        let local_scope_id = read_u32_ne(row, 16).ok_or(MetricQuality::Invalid)?;
        let remote_scope_id = read_u32_ne(row, 40).ok_or(MetricQuality::Invalid)?;

        endpoints.push(TcpEndpoint {
            local_address: format_ipv6_address(local_address, local_scope_id),
            local_port: network_port(read_u32_ne(row, 20).ok_or(MetricQuality::Invalid)?),
            remote_address: (remote_address != [0; 16])
                .then(|| format_ipv6_address(remote_address, remote_scope_id)),
            remote_port: if remote_address != [0; 16] {
                Some(network_port(
                    read_u32_ne(row, 44).ok_or(MetricQuality::Invalid)?,
                ))
            } else {
                None
            },
            state: tcp_state_name(read_u32_ne(row, 48).ok_or(MetricQuality::Invalid)?),
            pid: Some(read_u32_ne(row, 52).ok_or(MetricQuality::Invalid)?),
        });
    }

    Ok(endpoints)
}

fn query_tcp_table(family: u32) -> Result<Vec<TcpEndpoint>, MetricQuality> {
    let mut required_size = 0u32;
    let initial_result = unsafe {
        GetExtendedTcpTable(
            ptr::null_mut(),
            &mut required_size,
            0,
            family,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };

    if initial_result != ERROR_SUCCESS
        && initial_result != ERROR_INSUFFICIENT_BUFFER
        && initial_result != ERROR_BUFFER_OVERFLOW
    {
        return Err(classify_win32_error(initial_result));
    }
    if required_size == 0 {
        return (initial_result == ERROR_SUCCESS)
            .then(Vec::new)
            .ok_or_else(|| classify_win32_error(initial_result));
    }
    if required_size as usize > MAX_EXTENDED_TABLE_BUFFER {
        return Err(MetricQuality::Invalid);
    }

    let mut buffer = vec![0u8; required_size as usize];
    loop {
        let mut returned_size = buffer.len() as u32;
        let result = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut returned_size,
                0,
                family,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if result == ERROR_INSUFFICIENT_BUFFER || result == ERROR_BUFFER_OVERFLOW {
            let new_size = returned_size as usize;
            if new_size <= buffer.len() || new_size > MAX_EXTENDED_TABLE_BUFFER {
                return Err(MetricQuality::ReadError);
            }
            buffer.resize(new_size, 0);
            continue;
        }
        if result != ERROR_SUCCESS {
            return Err(classify_win32_error(result));
        }

        let returned_size = if returned_size == 0 {
            buffer.len()
        } else {
            returned_size as usize
        };
        if returned_size > buffer.len() || returned_size < size_of::<u32>() {
            return Err(MetricQuality::Invalid);
        }
        buffer.truncate(returned_size);
        return match family {
            value if value == AF_INET as u32 => parse_ipv4_tcp_table(&buffer),
            value if value == AF_INET6 as u32 => parse_ipv6_tcp_table(&buffer),
            _ => Err(MetricQuality::Unsupported),
        };
    }
}

fn query_udp_count(family: u32) -> Result<u32, MetricQuality> {
    let mut required_size = 0u32;
    let initial_result = unsafe {
        GetExtendedUdpTable(
            ptr::null_mut(),
            &mut required_size,
            0,
            family,
            UDP_TABLE_OWNER_PID,
            0,
        )
    };

    if initial_result != ERROR_SUCCESS
        && initial_result != ERROR_INSUFFICIENT_BUFFER
        && initial_result != ERROR_BUFFER_OVERFLOW
    {
        return Err(classify_win32_error(initial_result));
    }
    if required_size == 0 {
        return (initial_result == ERROR_SUCCESS)
            .then_some(0)
            .ok_or_else(|| classify_win32_error(initial_result));
    }
    if required_size as usize > MAX_EXTENDED_TABLE_BUFFER {
        return Err(MetricQuality::Invalid);
    }

    let mut buffer = vec![0u8; required_size as usize];
    loop {
        let mut returned_size = buffer.len() as u32;
        let result = unsafe {
            GetExtendedUdpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut returned_size,
                0,
                family,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if result == ERROR_INSUFFICIENT_BUFFER || result == ERROR_BUFFER_OVERFLOW {
            let new_size = returned_size as usize;
            if new_size <= buffer.len() || new_size > MAX_EXTENDED_TABLE_BUFFER {
                return Err(MetricQuality::ReadError);
            }
            buffer.resize(new_size, 0);
            continue;
        }
        if result != ERROR_SUCCESS {
            return Err(classify_win32_error(result));
        }

        let returned_size = if returned_size == 0 {
            buffer.len()
        } else {
            returned_size as usize
        };
        if returned_size > buffer.len() || returned_size < size_of::<u32>() {
            return Err(MetricQuality::Invalid);
        }
        buffer.truncate(returned_size);
        let row_size = if family == AF_INET as u32 {
            size_of::<MIB_UDPROW_OWNER_PID>()
        } else if family == AF_INET6 as u32 {
            size_of::<MIB_UDP6ROW_OWNER_PID>()
        } else {
            return Err(MetricQuality::Unsupported);
        };
        let count = table_entry_count(&buffer, row_size)?;
        return u32::try_from(count).map_err(|_| MetricQuality::Invalid);
    }
}

fn count_tcp_endpoints(endpoints: &[TcpEndpoint]) -> Result<TcpCounts, MetricQuality> {
    let mut counts = TcpCounts::default();
    for endpoint in endpoints {
        counts.total = counts.total.checked_add(1).ok_or(MetricQuality::Invalid)?;
        match endpoint.state.as_deref() {
            Some("ESTABLISHED") => {
                counts.established = counts
                    .established
                    .checked_add(1)
                    .ok_or(MetricQuality::Invalid)?;
            }
            Some("LISTEN") => {
                counts.listening = counts
                    .listening
                    .checked_add(1)
                    .ok_or(MetricQuality::Invalid)?;
            }
            Some("TIME-WAIT") => {
                counts.time_wait = counts
                    .time_wait
                    .checked_add(1)
                    .ok_or(MetricQuality::Invalid)?;
            }
            _ => {}
        }
    }
    Ok(counts)
}

fn combine_tcp_counts(first: TcpCounts, second: TcpCounts) -> Result<TcpCounts, MetricQuality> {
    Ok(TcpCounts {
        established: first
            .established
            .checked_add(second.established)
            .ok_or(MetricQuality::Invalid)?,
        listening: first
            .listening
            .checked_add(second.listening)
            .ok_or(MetricQuality::Invalid)?,
        time_wait: first
            .time_wait
            .checked_add(second.time_wait)
            .ok_or(MetricQuality::Invalid)?,
        total: first
            .total
            .checked_add(second.total)
            .ok_or(MetricQuality::Invalid)?,
    })
}

fn buffer_contains_bytes(
    pointer: *const u8,
    length: usize,
    base: *const u8,
    buffer_len: usize,
) -> bool {
    let base_address = base as usize;
    let pointer_address = pointer as usize;
    let Some(buffer_end) = base_address.checked_add(buffer_len) else {
        return false;
    };
    let Some(pointer_end) = pointer_address.checked_add(length) else {
        return false;
    };
    pointer_address >= base_address && pointer_end <= buffer_end
}

unsafe fn socket_address_to_ip(
    address: SOCKET_ADDRESS,
    base: *const u8,
    buffer_len: usize,
) -> Option<ParsedIpAddress> {
    let sockaddr_len = usize::try_from(address.iSockaddrLength).ok()?;
    let sockaddr = address.lpSockaddr as *const u8;
    if sockaddr_len < 2 || !buffer_contains_bytes(sockaddr, sockaddr_len, base, buffer_len) {
        return None;
    }

    let bytes = std::slice::from_raw_parts(sockaddr, sockaddr_len);
    let family = u16::from_ne_bytes([bytes[0], bytes[1]]);
    match family {
        value if value == AF_INET => {
            let address = bytes.get(4..8)?.try_into().ok()?;
            Some(ParsedIpAddress::V4(address))
        }
        value if value == AF_INET6 => {
            let address = bytes.get(8..24)?.try_into().ok()?;
            let scope_id = bytes
                .get(24..28)
                .and_then(|scope| scope.try_into().ok())
                .map(u32::from_ne_bytes)
                .unwrap_or(0);
            Some(ParsedIpAddress::V6(address, scope_id))
        }
        _ => None,
    }
}

fn push_ip_address(
    address: ParsedIpAddress,
    ipv4_addresses: &mut Vec<String>,
    ipv6_addresses: &mut Vec<String>,
) {
    match address {
        ParsedIpAddress::V4(bytes) => ipv4_addresses.push(Ipv4Addr::from(bytes).to_string()),
        ParsedIpAddress::V6(bytes, scope_id) => {
            ipv6_addresses.push(format_ipv6_address(bytes, scope_id));
        }
    }
}

unsafe fn read_node_field<T: Copy>(node: &[u8], offset: usize) -> Option<T> {
    let end = offset.checked_add(size_of::<T>())?;
    let bytes = node.get(offset..end)?;
    Some(ptr::read_unaligned(bytes.as_ptr() as *const T))
}

unsafe fn declared_node<'a>(
    pointer: *const u8,
    base: *const u8,
    buffer: &'a [u8],
) -> Result<&'a [u8], MetricQuality> {
    // Length 是所有 IP Helper 链节点的共同首字段，先只读取这 4 字节。
    if !buffer_contains_bytes(pointer, size_of::<u32>(), base, buffer.len()) {
        return Err(MetricQuality::Invalid);
    }
    let length = ptr::read_unaligned(pointer as *const u32) as usize;
    if length < size_of::<u32>() || !buffer_contains_bytes(pointer, length, base, buffer.len()) {
        return Err(MetricQuality::Invalid);
    }

    let offset = (pointer as usize)
        .checked_sub(base as usize)
        .ok_or(MetricQuality::Invalid)?;
    let end = offset.checked_add(length).ok_or(MetricQuality::Invalid)?;
    buffer.get(offset..end).ok_or(MetricQuality::Invalid)
}

unsafe fn parse_socket_address_chain<T>(
    first: *mut T,
    base: *const u8,
    buffer: &[u8],
    next_offset: usize,
    address_offset: usize,
    mut on_address: impl FnMut(ParsedIpAddress),
) -> Result<(), MetricQuality> {
    let mut current = first;
    let mut visited = HashSet::new();
    let max_nodes = buffer.len() / size_of::<u32>() + 1;

    while !current.is_null() {
        if !visited.insert(current as usize) || visited.len() > max_nodes {
            return Err(MetricQuality::Invalid);
        }
        let node = declared_node(current as *const u8, base, buffer)?;
        if let Some(address) = read_node_field::<SOCKET_ADDRESS>(node, address_offset) {
            if let Some(address) = socket_address_to_ip(address, base, buffer.len()) {
                on_address(address);
            }
        }

        // 旧版本节点可能没有 Next；此时只安全解析当前节点并停止链遍历。
        let Some(next) = read_node_field::<*mut T>(node, next_offset) else {
            break;
        };
        current = next;
    }

    Ok(())
}

fn merge_adapter_address_data(target: &mut AdapterAddressData, mut source: AdapterAddressData) {
    target.ipv4_addresses.append(&mut source.ipv4_addresses);
    target.ipv6_addresses.append(&mut source.ipv6_addresses);
    if target.gateway.is_empty() {
        target.gateway = source.gateway;
    }
    target.dns_servers.append(&mut source.dns_servers);
    if source.dhcp_enabled.is_some() {
        target.dhcp_enabled = source.dhcp_enabled;
    }
}

unsafe fn parse_adapter_addresses_buffer(
    buffer: &[u8],
) -> Result<AdapterAddressTable, MetricQuality> {
    if buffer.is_empty() {
        return Ok(AdapterAddressTable::default());
    }

    let base = buffer.as_ptr();
    let mut current = base;
    let mut visited_adapters = HashSet::new();
    let max_adapters = buffer.len() / size_of::<u32>() + 1;
    let mut result = AdapterAddressTable::default();

    while !current.is_null() {
        if !visited_adapters.insert(current as usize) || visited_adapters.len() > max_adapters {
            return Err(MetricQuality::Invalid);
        }
        let adapter = declared_node(current, base, buffer)?;

        // LUID 是跨 IPv4/IPv6 和索引变化的稳定身份，缺失时不能安全回填到 MIB_IF_ROW2。
        let luid = read_u64_ne(adapter, offset_of!(IP_ADAPTER_ADDRESSES_LH, Luid))
            .ok_or(MetricQuality::Invalid)?;
        if luid == 0 {
            return Err(MetricQuality::Invalid);
        }

        let if_index = read_u32_ne(adapter, size_of::<u32>());
        let ipv6_if_index = read_u32_ne(adapter, offset_of!(IP_ADAPTER_ADDRESSES_LH, Ipv6IfIndex));
        let dhcp_enabled = read_u32_ne(adapter, offset_of!(IP_ADAPTER_ADDRESSES_LH, Anonymous2))
            .map(|flags| (flags & IP_ADAPTER_DHCP_ENABLED) != 0);
        let mut data = AdapterAddressData {
            dhcp_enabled,
            ..AdapterAddressData::default()
        };

        if let Some(first) = read_node_field::<*mut IP_ADAPTER_UNICAST_ADDRESS_LH>(
            adapter,
            offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstUnicastAddress),
        ) {
            parse_socket_address_chain(
                first,
                base,
                buffer,
                offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Next),
                offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Address),
                |address| {
                    push_ip_address(address, &mut data.ipv4_addresses, &mut data.ipv6_addresses);
                },
            )?;
        }

        if let Some(first) = read_node_field::<*mut IP_ADAPTER_GATEWAY_ADDRESS_LH>(
            adapter,
            offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstGatewayAddress),
        ) {
            parse_socket_address_chain(
                first,
                base,
                buffer,
                offset_of!(IP_ADAPTER_GATEWAY_ADDRESS_LH, Next),
                offset_of!(IP_ADAPTER_GATEWAY_ADDRESS_LH, Address),
                |address| {
                    if data.gateway.is_empty() {
                        data.gateway = match address {
                            ParsedIpAddress::V4(bytes) => Ipv4Addr::from(bytes).to_string(),
                            ParsedIpAddress::V6(bytes, scope_id) => {
                                format_ipv6_address(bytes, scope_id)
                            }
                        };
                    }
                },
            )?;
        }

        if let Some(first) = read_node_field::<*mut IP_ADAPTER_DNS_SERVER_ADDRESS_XP>(
            adapter,
            offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstDnsServerAddress),
        ) {
            parse_socket_address_chain(
                first,
                base,
                buffer,
                offset_of!(IP_ADAPTER_DNS_SERVER_ADDRESS_XP, Next),
                offset_of!(IP_ADAPTER_DNS_SERVER_ADDRESS_XP, Address),
                |address| {
                    let value = match address {
                        ParsedIpAddress::V4(bytes) => Ipv4Addr::from(bytes).to_string(),
                        ParsedIpAddress::V6(bytes, scope_id) => {
                            format_ipv6_address(bytes, scope_id)
                        }
                    };
                    data.dns_servers.push(value);
                },
            )?;
        }

        let next = read_node_field::<*mut IP_ADAPTER_ADDRESSES_LH>(
            adapter,
            offset_of!(IP_ADAPTER_ADDRESSES_LH, Next),
        );
        let entry = result.by_luid.entry(luid).or_default();
        merge_adapter_address_data(entry, data);
        for index in [if_index, ipv6_if_index].into_iter().flatten() {
            if index != 0 {
                result.by_index.insert(index, luid);
            }
        }

        // 旧版本节点没有 Next 时，当前数据仍可用，但不能继续假读后续字段。
        let Some(next) = next else { break };
        current = next as *const u8;
    }

    Ok(result)
}

fn query_adapters_addresses() -> Result<AdapterAddressTable, MetricQuality> {
    let mut required_size = 0u32;
    let initial_result = unsafe {
        GetAdaptersAddresses(
            AF_UNSPEC as u32,
            GAA_FLAG_INCLUDE_GATEWAYS,
            ptr::null(),
            ptr::null_mut(),
            &mut required_size,
        )
    };

    if initial_result != ERROR_SUCCESS
        && initial_result != ERROR_BUFFER_OVERFLOW
        && initial_result != ERROR_INSUFFICIENT_BUFFER
    {
        return Err(classify_win32_error(initial_result));
    }
    if required_size == 0 {
        return (initial_result == ERROR_SUCCESS)
            .then(AdapterAddressTable::default)
            .ok_or_else(|| classify_win32_error(initial_result));
    }
    if required_size as usize > MAX_ADAPTER_ADDRESSES_BUFFER {
        return Err(MetricQuality::Invalid);
    }

    let mut buffer = vec![0u8; required_size as usize];
    loop {
        let mut returned_size = buffer.len() as u32;
        let result = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                ptr::null(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut returned_size,
            )
        };
        if result == ERROR_BUFFER_OVERFLOW || result == ERROR_INSUFFICIENT_BUFFER {
            let new_size = returned_size as usize;
            if new_size <= buffer.len() || new_size > MAX_ADAPTER_ADDRESSES_BUFFER {
                return Err(MetricQuality::ReadError);
            }
            buffer.resize(new_size, 0);
            continue;
        }
        if result != ERROR_SUCCESS {
            return Err(classify_win32_error(result));
        }

        let returned_size = if returned_size == 0 {
            buffer.len()
        } else {
            returned_size as usize
        };
        if returned_size > buffer.len() {
            return Err(MetricQuality::Invalid);
        }
        buffer.truncate(returned_size);
        return unsafe { parse_adapter_addresses_buffer(&buffer) };
    }
}

fn hardware_interface_flag(row: &MIB_IF_ROW2) -> bool {
    // windows-sys 0.59 暴露该 API flag 的原始 bitfield；bit 0 即 HardwareInterface。
    row.InterfaceAndOperStatusFlags._bitfield & 0x01 != 0
}

fn format_oper_status(status: i32) -> String {
    match status {
        1 => "Up (已连接)".to_string(),
        2 => "Down (已断开)".to_string(),
        3 => "Testing".to_string(),
        4 => "Unknown".to_string(),
        5 => "Dormant".to_string(),
        6 => "Not present".to_string(),
        7 => "Lower layer down".to_string(),
        value => format!("状态 {value}"),
    }
}

unsafe fn interface_luid_value(row: &MIB_IF_ROW2) -> u64 {
    // NET_LUID_LH 的 Value 是稳定的 64 位接口身份，不受 IfIndex 重分配影响。
    row.InterfaceLuid.Value
}

fn update_adapter_rates(
    interface_luid: u64,
    total_rx: u64,
    total_tx: u64,
    sample_time: Instant,
    timestamp: u64,
) -> (MetricValue<u64>, MetricValue<u64>) {
    let mut guard = PREVIOUS_ADAPTER_SAMPLES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let samples = guard.get_or_insert_with(HashMap::new);
    let previous = samples.insert(interface_luid, (sample_time, total_rx, total_tx));
    let (previous_rx, previous_tx, elapsed) = match previous {
        Some((previous_time, previous_rx, previous_tx)) => (
            Some(previous_rx),
            Some(previous_tx),
            previous_time
                .checked_duration_since(sample_time)
                .map(|_| -1.0)
                .or_else(|| {
                    sample_time
                        .checked_duration_since(previous_time)
                        .map(|d| d.as_secs_f64())
                })
                .unwrap_or(-1.0),
        ),
        None => (None, None, -1.0),
    };

    (
        rate_metric(previous_rx, total_rx, elapsed, timestamp),
        rate_metric(previous_tx, total_tx, elapsed, timestamp),
    )
}

/// 采集系统中所有网络适配器与实时收发速率。
pub fn collect_network_adapters() -> Vec<NetworkAdapterInfo> {
    collect_network_adapters_at(current_timestamp_ms())
}

fn collect_network_adapters_at(timestamp: u64) -> Vec<NetworkAdapterInfo> {
    let mut adapters = Vec::new();

    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = ptr::null_mut();
        let result = GetIfTable2(&mut table);
        if result != ERROR_SUCCESS || table.is_null() {
            if !table.is_null() {
                FreeMibTable(table as *const _);
            }
            return adapters;
        }

        let address_data = query_adapters_addresses();
        let sample_time = Instant::now();
        let num_entries = (*table).NumEntries as usize;
        let rows = (*table).Table.as_ptr();

        for index in 0..num_entries {
            let row = &*rows.add(index);
            let alias_end = row
                .Alias
                .iter()
                .position(|&value| value == 0)
                .unwrap_or(row.Alias.len());
            let description_end = row
                .Description
                .iter()
                .position(|&value| value == 0)
                .unwrap_or(row.Description.len());
            let alias = String::from_utf16_lossy(&row.Alias[..alias_end])
                .trim()
                .to_string();
            let description = String::from_utf16_lossy(&row.Description[..description_end])
                .trim()
                .to_string();
            let interface_index = row.InterfaceIndex;
            let interface_luid = interface_luid_value(row);
            let (ipv4_addresses, ipv6_addresses, gateway, dns_servers, dhcp_enabled) =
                match address_data.as_ref() {
                    Ok(data) => match data.by_luid.get(&interface_luid) {
                        Some(data) => (
                            data.ipv4_addresses.clone(),
                            data.ipv6_addresses.clone(),
                            data.gateway.clone(),
                            data.dns_servers.clone(),
                            data.dhcp_enabled
                                .map(|enabled| {
                                    MetricValue::good_at(enabled, "", IP_HELPER_SOURCE, timestamp)
                                })
                                .unwrap_or_else(|| {
                                    missing_metric_at(
                                        "",
                                        IP_HELPER_SOURCE,
                                        MetricQuality::Unsupported,
                                        "地址节点未包含 DHCP Flags，DHCP 状态不可用",
                                        timestamp,
                                    )
                                }),
                        ),
                        None => (
                            Vec::new(),
                            Vec::new(),
                            String::new(),
                            Vec::new(),
                            missing_metric_at(
                                "",
                                IP_HELPER_SOURCE,
                                MetricQuality::Unsupported,
                                "地址 API 未返回该接口，DHCP 状态不可用",
                                timestamp,
                            ),
                        ),
                    },
                    Err(quality) => (
                        Vec::new(),
                        Vec::new(),
                        String::new(),
                        Vec::new(),
                        missing_metric_at(
                            "",
                            IP_HELPER_SOURCE,
                            *quality,
                            "GetAdaptersAddresses 失败，DHCP 状态不可用",
                            timestamp,
                        ),
                    ),
                };
            let (rx_speed_bps, tx_speed_bps) = update_adapter_rates(
                interface_luid,
                row.InOctets,
                row.OutOctets,
                sample_time,
                timestamp,
            );

            adapters.push(NetworkAdapterInfo {
                index: interface_index,
                name: if alias.is_empty() {
                    description.clone()
                } else {
                    alias.clone()
                },
                alias,
                description,
                mac_address: format_mac(safe_mac_bytes(
                    &row.PhysicalAddress,
                    row.PhysicalAddressLength as usize,
                )),
                is_physical: hardware_interface_flag(row),
                oper_status: format_oper_status(row.OperStatus),
                link_speed_bps: MetricValue::good_at(
                    row.ReceiveLinkSpeed,
                    "bit/s",
                    IP_HELPER_SOURCE,
                    timestamp,
                ),
                mtu: MetricValue::good_at(row.Mtu, "Bytes", IP_HELPER_SOURCE, timestamp),
                ipv4_addresses,
                ipv6_addresses,
                gateway,
                dns_servers,
                dhcp_enabled,
                rx_speed_bps,
                tx_speed_bps,
                total_rx_bytes: MetricValue::good_at(
                    row.InOctets,
                    "Bytes",
                    IP_HELPER_SOURCE,
                    timestamp,
                ),
                total_tx_bytes: MetricValue::good_at(
                    row.OutOctets,
                    "Bytes",
                    IP_HELPER_SOURCE,
                    timestamp,
                ),
            });
        }

        FreeMibTable(table as *const _);
    }

    adapters
}

/// 采集 Wi-Fi 无线网络连接状态；Native WLAN provider 接入前保持明确空值。
pub fn collect_wifi_status() -> WiFiConnectionInfo {
    collect_wifi_status_at(current_timestamp_ms())
}

fn collect_wifi_status_at(timestamp: u64) -> WiFiConnectionInfo {
    WiFiConnectionInfo {
        is_connected: unsupported_metric_at("", WLAN_SOURCE, "WLAN provider 尚未接入", timestamp),
        ssid: unsupported_metric_at("", WLAN_SOURCE, "WLAN provider 尚未接入", timestamp),
        bssid: unsupported_metric_at("", WLAN_SOURCE, "WLAN provider 尚未接入", timestamp),
        signal_quality_percent: unsupported_metric_at(
            "%",
            WLAN_SOURCE,
            "WLAN provider 尚未接入",
            timestamp,
        ),
        rssi_dbm: unsupported_metric_at("dBm", WLAN_SOURCE, "WLAN provider 尚未接入", timestamp),
        channel: unsupported_metric_at("", WLAN_SOURCE, "WLAN provider 尚未接入", timestamp),
        radio_frequency_ghz: unsupported_metric_at(
            "GHz",
            WLAN_SOURCE,
            "WLAN provider 尚未接入",
            timestamp,
        ),
        security_cipher: unsupported_metric_at(
            "",
            WLAN_SOURCE,
            "WLAN provider 尚未接入",
            timestamp,
        ),
    }
}

/// 采集 TCP / UDP 活跃连接数概况。
pub fn collect_connections_summary() -> NetworkConnectionsSummary {
    collect_connections_summary_at(current_timestamp_ms())
}

fn collect_connections_summary_at(timestamp: u64) -> NetworkConnectionsSummary {
    let tcp_v4 = query_tcp_table(AF_INET as u32);
    let tcp_v6 = query_tcp_table(AF_INET6 as u32);
    let tcp_counts = match (&tcp_v4, &tcp_v6) {
        (Ok(v4), Ok(v6)) => count_tcp_endpoints(v4).and_then(|first| {
            count_tcp_endpoints(v6).and_then(|second| combine_tcp_counts(first, second))
        }),
        (Err(quality), _) | (_, Err(quality)) => Err(*quality),
    };
    let tcp_metrics = match tcp_counts {
        Ok(counts) => (
            MetricValue::good_at(counts.established, "connections", TCP_SOURCE, timestamp),
            MetricValue::good_at(counts.listening, "connections", TCP_SOURCE, timestamp),
            MetricValue::good_at(counts.time_wait, "connections", TCP_SOURCE, timestamp),
            MetricValue::good_at(counts.total, "connections", TCP_SOURCE, timestamp),
        ),
        Err(quality) => (
            missing_metric_at(
                "connections",
                TCP_SOURCE,
                quality,
                "TCP API 查询失败",
                timestamp,
            ),
            missing_metric_at(
                "connections",
                TCP_SOURCE,
                quality,
                "TCP API 查询失败",
                timestamp,
            ),
            missing_metric_at(
                "connections",
                TCP_SOURCE,
                quality,
                "TCP API 查询失败",
                timestamp,
            ),
            missing_metric_at(
                "connections",
                TCP_SOURCE,
                quality,
                "TCP API 查询失败",
                timestamp,
            ),
        ),
    };

    let udp_v4 = query_udp_count(AF_INET as u32);
    let udp_v6 = query_udp_count(AF_INET6 as u32);
    let udp_count = match (&udp_v4, &udp_v6) {
        (Ok(v4), Ok(v6)) => v4.checked_add(*v6).ok_or(MetricQuality::Invalid),
        (Err(quality), _) | (_, Err(quality)) => Err(*quality),
    };
    let udp_metric = match udp_count {
        Ok(count) => MetricValue::good_at(count, "endpoints", UDP_SOURCE, timestamp),
        Err(quality) => missing_metric_at(
            "endpoints",
            UDP_SOURCE,
            quality,
            "UDP API 查询失败",
            timestamp,
        ),
    };

    NetworkConnectionsSummary {
        tcp_established_count: tcp_metrics.0,
        tcp_listening_count: tcp_metrics.1,
        tcp_time_wait_count: tcp_metrics.2,
        tcp_total_connections: tcp_metrics.3,
        udp_endpoints_count: udp_metric,
    }
}

/// 采集网络子系统全景快照。
pub fn collect_network_snapshot() -> NetworkSnapshot {
    let timestamp = current_timestamp_ms();
    NetworkSnapshot {
        adapters: collect_network_adapters_at(timestamp),
        wifi_info: collect_wifi_status_at(timestamp),
        connections_summary: collect_connections_summary_at(timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};
    use std::time::Duration;

    fn put_u32(buffer: &mut [u8], offset: usize, value: u32) {
        buffer[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn put_u64(buffer: &mut [u8], offset: usize, value: u64) {
        buffer[offset..offset + size_of::<u64>()].copy_from_slice(&value.to_ne_bytes());
    }

    unsafe fn put_pointer<T>(buffer: &mut [u8], offset: usize, target_offset: usize) {
        let pointer = if target_offset == usize::MAX {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr().add(target_offset) as *mut T
        };
        std::ptr::write_unaligned(buffer.as_mut_ptr().add(offset) as *mut *mut T, pointer);
    }

    fn adapter_buffer(luid: u64, if_index: u32, ipv6_if_index: u32) -> Vec<u8> {
        let mut buffer = vec![0u8; size_of::<IP_ADAPTER_ADDRESSES_LH>()];
        put_u32(&mut buffer, 0, size_of::<IP_ADAPTER_ADDRESSES_LH>() as u32);
        put_u32(&mut buffer, size_of::<u32>(), if_index);
        put_u32(
            &mut buffer,
            offset_of!(IP_ADAPTER_ADDRESSES_LH, Ipv6IfIndex),
            ipv6_if_index,
        );
        put_u64(&mut buffer, offset_of!(IP_ADAPTER_ADDRESSES_LH, Luid), luid);
        buffer
    }

    #[test]
    fn counter_reset_produces_no_network_rate() {
        assert_eq!(calculate_rate(Some(500), 100, 1.0), None);
    }

    #[test]
    fn subsecond_rate_does_not_round_overflow_to_u64_max() {
        assert_eq!(calculate_rate(Some(0), 1u64 << 63, 0.5), None);
    }

    #[test]
    fn rate_floor_does_not_round_u64_max_upward() {
        assert_eq!(calculate_rate(Some(0), u64::MAX, 2.0), Some(u64::MAX / 2));
    }

    #[test]
    fn adapter_addresses_are_keyed_by_luid_and_alias_both_indices() {
        let luid = 0x1_0000_0007u64;
        let if_index = 7;
        let ipv6_if_index = 7007;
        let buffer = adapter_buffer(luid, if_index, ipv6_if_index);

        let table = unsafe { parse_adapter_addresses_buffer(&buffer) }.unwrap();
        assert!(table.by_luid.contains_key(&luid));
        assert_eq!(table.by_index.get(&if_index), Some(&luid));
        assert_eq!(table.by_index.get(&ipv6_if_index), Some(&luid));
    }

    #[test]
    fn short_adapter_version_is_read_without_full_struct_load() {
        let luid = 0x2_0000_0007u64;
        let mut buffer = adapter_buffer(luid, 7, 7007);
        let short_length = offset_of!(IP_ADAPTER_ADDRESSES_LH, Luid) + size_of::<u64>();
        put_u32(&mut buffer, 0, short_length as u32);

        let table = unsafe { parse_adapter_addresses_buffer(&buffer) }.unwrap();
        assert!(table.by_luid.contains_key(&luid));
    }

    #[test]
    fn short_unicast_gateway_and_dns_nodes_are_parsed_field_by_field() {
        let luid = 0x3_0000_0007u64;
        let mut buffer = adapter_buffer(luid, 7, 7007);
        let node_specs = [
            (
                offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Address),
                offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstUnicastAddress),
                [192, 0, 2, 1],
            ),
            (
                offset_of!(IP_ADAPTER_GATEWAY_ADDRESS_LH, Address),
                offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstGatewayAddress),
                [192, 0, 2, 2],
            ),
            (
                offset_of!(IP_ADAPTER_DNS_SERVER_ADDRESS_XP, Address),
                offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstDnsServerAddress),
                [192, 0, 2, 3],
            ),
        ];
        let mut node_offsets = [0usize; 3];
        let mut sockaddr_offsets = [0usize; 3];

        for (index, (address_offset, _, _)) in node_specs.iter().enumerate() {
            let node_offset = buffer.len();
            let node_length = address_offset + size_of::<SOCKET_ADDRESS>();
            let sockaddr_offset = node_offset + node_length;
            buffer.resize(sockaddr_offset + 8, 0);
            put_u32(&mut buffer, node_offset, node_length as u32);
            sockaddr_offsets[index] = sockaddr_offset;
            node_offsets[index] = node_offset;
        }

        for (index, (address_offset, _, _)) in node_specs.iter().enumerate() {
            let node_offset = node_offsets[index];
            let sockaddr_offset = sockaddr_offsets[index];
            unsafe {
                let socket_address = SOCKET_ADDRESS {
                    lpSockaddr: buffer.as_mut_ptr().add(sockaddr_offset) as *mut _,
                    iSockaddrLength: 8,
                };
                std::ptr::write_unaligned(
                    buffer.as_mut_ptr().add(node_offset + address_offset) as *mut SOCKET_ADDRESS,
                    socket_address,
                );
            }
        }

        unsafe {
            put_pointer::<IP_ADAPTER_UNICAST_ADDRESS_LH>(
                &mut buffer,
                offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstUnicastAddress),
                node_offsets[0],
            );
            put_pointer::<IP_ADAPTER_GATEWAY_ADDRESS_LH>(
                &mut buffer,
                offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstGatewayAddress),
                node_offsets[1],
            );
            put_pointer::<IP_ADAPTER_DNS_SERVER_ADDRESS_XP>(
                &mut buffer,
                offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstDnsServerAddress),
                node_offsets[2],
            );
        }

        for (index, octets) in node_specs.iter().map(|(_, _, octets)| octets).enumerate() {
            let sockaddr_offset = sockaddr_offsets[index];
            buffer[sockaddr_offset..sockaddr_offset + 2]
                .copy_from_slice(&(AF_INET as u16).to_ne_bytes());
            buffer[sockaddr_offset + 4..sockaddr_offset + 8].copy_from_slice(octets);
        }

        let table = unsafe { parse_adapter_addresses_buffer(&buffer) }.unwrap();
        let data = table.by_luid.get(&luid).unwrap();
        assert_eq!(data.ipv4_addresses, vec!["192.0.2.1"]);
        assert_eq!(data.gateway, "192.0.2.2");
        assert_eq!(data.dns_servers, vec!["192.0.2.3"]);
    }

    #[test]
    fn interface_identity_change_does_not_reuse_previous_rate_sample() {
        const FIRST_LUID: u64 = 0x1_0000_0001;
        const SECOND_LUID: u64 = 0x2_0000_0001;
        let start = Instant::now();
        PREVIOUS_ADAPTER_SAMPLES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_or_insert_with(HashMap::new)
            .clear();

        let first = update_adapter_rates(FIRST_LUID, 100, 200, start, 1);
        let changed =
            update_adapter_rates(SECOND_LUID, 110, 210, start + Duration::from_secs(1), 2);
        let same_identity =
            update_adapter_rates(FIRST_LUID, 150, 250, start + Duration::from_secs(2), 3);

        assert_eq!(first.0.quality, MetricQuality::Unsupported);
        assert_eq!(changed.0.quality, MetricQuality::Unsupported);
        assert_eq!(same_identity.0.quality, MetricQuality::Good);
    }

    #[test]
    fn malformed_tcp_buffer_returns_invalid_without_pointer_reads() {
        assert_eq!(
            parse_ipv4_tcp_table(&[0; 3]).unwrap_err(),
            MetricQuality::Invalid
        );
    }

    #[test]
    fn physical_address_length_is_clamped_to_the_api_buffer() {
        assert_eq!(
            safe_mac_bytes(&[1, 2, 3, 4, 5, 6, 7, 8], 100),
            &[1, 2, 3, 4, 5, 6, 7, 8]
        );
    }
}

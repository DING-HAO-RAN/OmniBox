//! 浏览器会话迁移器核心数据结构与规范
//!
//! 100% 对齐 Browser Session Migrator 的 Bundle Schema 与配置模型。

use serde::{Deserialize, Serialize};

/// 单条 Cookie 记录结构
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CookieRecord {
    pub domain: String,
    pub path: String,
    pub name: String,
    pub value: String,
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<f64>,
    pub secure: bool,
    #[serde(rename = "httpOnly")]
    pub http_only: bool,
    #[serde(rename = "sameSite")]
    pub same_site: Option<String>,
}

/// 导出环境元数据
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExportMetadata {
    #[serde(rename = "exportedAt")]
    pub exported_at: String,
    #[serde(rename = "operatingSystem")]
    pub operating_system: String,
    pub browser: String,
    #[serde(rename = "browserVersion")]
    pub browser_version: String,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
    #[serde(rename = "acceptLanguage")]
    pub accept_language: String,
    pub timezone: String,
    #[serde(rename = "screenResolution")]
    pub screen_resolution: String,
}

/// 会话打包 Bundle 结构体 (Schema: bsm/session-bundle/v1)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SessionBundle {
    pub schema: String,
    #[serde(rename = "targetDomain")]
    pub target_domain: String,
    #[serde(rename = "siteUrl")]
    pub site_url: String,
    pub metadata: ExportMetadata,
    pub cookies: Vec<CookieRecord>,
}

/// 正在运行的 CDP 浏览器状态
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunningBrowserInfo {
    pub browser: String,
    pub endpoint: String,
    pub port: u16,
    pub profile_dir: String,
    pub pid: u32,
    pub browser_version: String,
    pub is_running: bool,
}

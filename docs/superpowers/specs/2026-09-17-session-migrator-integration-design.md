# 跨设备浏览器会话迁移与免密登录器 (Session Migrator) 整合设计规范

## 1. 目标与背景

将 `bilibili登录器` (Browser Session Migrator) 的完整功能 1:1 无损整合进 OmniBox 桌面万能工具箱中。
该功能主要面向用户合法拥有的个人账户（如 Bilibili、各大视频/社交/工作平台）在不同设备或不同浏览器之间的安全会话迁移。
UI 与美术风格必须保持 Windows 11 Fluent Design（云母磨砂质感、精致卡片、统一色调、柔和动画与高可读性）。

---

## 2. 核心功能与操作流程规范 (1:1 严格对齐)

### 2.1 浏览器连接控制区 (Connection Settings)
1. **浏览器选择**：支持 `Google Chrome` 和 `Microsoft Edge`。
   - 自动在 Windows 注册表和系统路径探测对应可执行文件路径 (`chrome.exe` / `msedge.exe`)。
2. **CDP 端口设置**：默认 `9222`，支持 1024~65535 自定义端口。
3. **独立配置目录 (Profile Dir)**：
   - 默认使用 `%LOCALAPPDATA%\BrowserSessionMigrator\profiles\{browser}-cdp`（或 `%LOCALAPPDATA%\OmniBox\profiles\{browser}-cdp`）。
   - 附带「选择目录」按钮，支持调用 Windows 原生文件夹选择器。
4. **控制按钮组**：
   - 「一键启动本地 CDP」：后台启动带 `--remote-debugging-address=127.0.0.1`、`--remote-debugging-port={port}`、`--user-data-dir={profile}` 的独立浏览器，轮询健康检查 `/json/version` 直至就绪。
   - 「停止 CDP 浏览器」：安全终止由工具启动的专属浏览器进程，不影响用户日常正在使用的其他浏览器。
   - 「生成桌面启动脚本」：在桌面一键生成对应的 `.cmd` 启动脚本，以便用户后续可直接打开同一个隔离登录环境。
5. **网络与域名参数**：
   - 本地 CDP 地址：默认 `http://127.0.0.1:9222`，严格强制回环校验。
   - 目标域名 (Domain)：默认预设 `bilibili.com`，可自定义输入其他合法域名。
   - 站点 URL (Site URL)：默认预设 `https://www.bilibili.com/`。
   - 包含子域名复选框：`包含目标域名的子域名（仅在确有需要时勾选）`，默认选中。

---

### 2.2 导出加密文件 (Export Tab)
- 说明文案：“在浏览器 A 中确认已经登录，然后选择一个仅由你控制的输出位置。导出文件不会包含明文 Cookie。”
- 默认 sessions 目录展示：展示 `%APPDATA%\OmniBox\sessions\` 路径。
- **「选择位置并导出」按钮**：
  - 弹出密码输入对话框（包含口令确认），内存使用后即刻擦除。
  - 连接 CDP，提取目标域名下所有有效未过期 Cookie，规范化属性。
  - 采集环境元数据（时间、系统、浏览器版本、分辨率、语言、时区）。
  - 执行 `Argon2id` (或 `PBKDF2`) + `AES-256-GCM` 加密生成 `bsm/1` 标准格式 JSON。
  - 保存文件并显示 SHA-256 完整性摘要，自动填充到导入页。
- **「导出扩展兼容文件」按钮**：
  - 采用 `PBKDF2-HMAC-SHA256` + `AES-256-GCM` 格式 `bsm/2`。
  - 支持用户直接配合内置的浏览器扩展导入到日常浏览器的默认 Profile 中。

---

### 2.3 从加密文件导入 (Import Tab)
- 说明文案：“先把加密文件安全传到设备 B，再连接目标浏览器。导入不会删除目标浏览器已有 Cookie。”
- **文件选择行**：文件路径输入框 + 「选择 JSON 文件」按钮 + 「清空」按钮。
- **「打开项目 sessions 存放目录」按钮**：系统资源管理器直接唤起目录。
- **「导入已选择的 JSON 文件」按钮**：
  - 提示输入口令，校验加密文件完整性与 AAD。
  - 严格校验目标域名一致性、Cookie 格式合法性与过期时间。
  - 通过 CDP `Network.setCookies` 注入，并自动导航至目标站点 `site_url`。
  - 界面反馈注入成功的 Cookie 数量。

---

### 2.4 日常浏览器配套扩展 (Extension Companion)
- 内置 Chromium (Chrome/Edge) 与 Firefox 扩展文件。
- 提供一键导出或解压配套扩展目录功能，方便用户直接通过开发者模式加载并在日常浏览器中使用。

---

## 3. UI 与美学规范 (保持 Fluent Design 统一)

- 整体沿用深色磨砂亚克力玻璃材质 (`backdrop-blur-xl`, `#0d1117` / `#161b22`)。
- 采用标志性的电光粉/青紫双色调（契合 B 站粉 `#fb7299` 与会话安全紫 `#8b5cf6`），呈现既专业又精致的现代桌面质感。
- 表单项、标签页、按钮微动效、Toast 通知与标题栏风格与整个 OmniBox 工具箱 100% 融合。
- 侧边栏图标与路由无缝融入 `ToolRegistry`。

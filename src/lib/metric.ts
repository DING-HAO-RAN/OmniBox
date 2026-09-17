import type { CollectionStatus, MetricValue } from '../types/module';

/** 仅允许真实、有限的成功指标进入数值计算。 */
function hasUsableValue<T>(metric: MetricValue<T>): boolean {
  if (metric.value === null || metric.value === undefined) return false;
  return typeof metric.value !== 'number' || Number.isFinite(metric.value);
}

/** 提取可用于算术和图表的数值，保留真实的 0。 */
export function metricNumber(metric?: MetricValue<number> | null): number | null {
  if (!metric || (metric.quality !== 'Good' && metric.quality !== 'Estimated')) {
    return null;
  }
  return typeof metric.value === 'number' && Number.isFinite(metric.value) ? metric.value : null;
}

/** 将指标质量转换为统一的前端状态。 */
export function metricState(
  metric?: MetricValue<unknown> | null,
): 'value' | 'unsupported' | 'unavailable' | 'permission' | 'error' {
  if (!metric) return 'unavailable';
  if ((metric.quality === 'Good' || metric.quality === 'Estimated') && hasUsableValue(metric)) {
    return 'value';
  }
  switch (metric.quality) {
    case 'Unsupported':
      return 'unsupported';
    case 'Unavailable':
      return 'unavailable';
    case 'PermissionDenied':
      return 'permission';
    default:
      return 'error';
  }
}

/** 显示文本指标或其质量状态，不把未知状态当作有效值。 */
export function metricText(metric?: MetricValue<string> | null): string {
  if (!metric) return '暂不可用';
  if ((metric.quality === 'Good' || metric.quality === 'Estimated') && metric.value !== null) {
    return metric.value;
  }
  switch (metric.quality) {
    case 'Unsupported':
      return '暂不支持';
    case 'PermissionDenied':
      return '权限不足';
    case 'Unavailable':
      return '暂不可用';
    default:
      return '读取失败/数据无效';
  }
}

/** 追加图表采样；无效指标写入 null 以保留断点。 */
export function appendMetricPoint(
  history: Array<number | null>,
  metric: MetricValue<number>,
  maxPoints: number,
): void {
  if (maxPoints <= 0) return;
  history.push(metricNumber(metric));
  while (history.length > maxPoints) history.shift();
}

/** 采样失败时保留时间点，但用 null 断开图表曲线。 */
export function appendMetricGap(history: Array<number | null>, maxPoints: number): void {
  if (maxPoints <= 0) return;
  history.push(null);
  while (history.length > maxPoints) history.shift();
}

/** 将指标的质量、来源和采样时间压缩为可读的元数据提示。 */
export function metricMeta(metric?: MetricValue<unknown> | null): string {
  if (!metric) return '质量：Unavailable · 来源：— · 时间：—';
  const timestamp = Number.isFinite(metric.timestamp) ? new Date(metric.timestamp).toISOString() : '—';
  const source = metric.source.trim() ? sanitizePreviewString(metric.source) : '—';
  return `质量：${metric.quality} · 来源：${source} · 时间：${timestamp}`;
}

/** 只有成功或估算集合才能把空数组显示为真实的 0。 */
export function collectionCount(
  items: readonly unknown[] | undefined,
  status: Pick<CollectionStatus, 'quality'> | Pick<MetricValue<unknown>, 'quality'> | undefined,
  reportLoaded: boolean,
): string {
  if (!reportLoaded) return '—';
  if (!status) return '暂不可用';
  if (status.quality === 'Good' || status.quality === 'Estimated') {
    return String(items?.length ?? 0);
  }
  return qualityLabel(status.quality);
}

function qualityLabel(quality: MetricValue<unknown>['quality']): string {
  switch (quality) {
    case 'Unsupported':
      return '暂不支持';
    case 'PermissionDenied':
      return '权限不足';
    case 'Unavailable':
      return '暂不可用';
    default:
      return '读取失败/数据无效';
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizedKey(key: string): string {
  return key.replace(/[^a-z0-9]/gi, '').toLowerCase();
}

/** 与 Rust 脱敏器保持相同的字段覆盖，避免预览分支出现隐私回退。 */
function isSensitiveKey(key: string): boolean {
  const normalized = normalizedKey(key);
  const identityKeys = [
    'username', 'currentuser', 'hostname', 'computername', 'macaddress',
    'ipv4address', 'ipv4addresses', 'ipv6address', 'ipv6addresses', 'gateway',
    'dnsserver', 'dnsservers', 'localaddress', 'remoteaddress', 'ssid', 'bssid',
    'uuid', 'serial', 'serialnumber', 'hardwareid', 'instanceid', 'sid', 'user',
  ];
  if (identityKeys.some((candidate) => normalized === candidate || normalized.endsWith(candidate))) {
    return true;
  }
  return [
    'path', 'installlocation', 'command', 'executable', 'adaptername', 'filename',
  ].some((candidate) => normalized === candidate || normalized.endsWith(candidate)) || [
    'password', 'cookie', 'token', 'apikey', 'recoverykey', 'privatekey', 'credential', 'secret',
  ].some((candidate) => normalized.includes(candidate));
}

function isCompleteMetricValue(value: Record<string, unknown>): boolean {
  const required = ['value', 'unit', 'quality', 'source', 'timestamp'];
  return (Object.keys(value).length === required.length || Object.keys(value).length === required.length + 1)
    && required.every((key) => key in value)
    && Object.keys(value).every((key) => required.includes(key) || key === 'error');
}

function sanitizePreviewString(input: string): string {
  let output = input.replace(/([\\/])Users([\\/])[^\\/]+/gi, '$1Users$2<USER>');
  output = output.replace(/(?<![a-z0-9])(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}(?![a-z0-9])/gi, '<MAC_ADDRESS>');
  output = output.replace(/(?<![a-z0-9])(?:[0-9a-f]{4}\.){2}[0-9a-f]{4}(?![a-z0-9])/gi, '<MAC_ADDRESS>');
  output = output.replace(/(?<![a-z0-9])[0-9a-f]{12}(?![a-z0-9])/gi, '<MAC_ADDRESS>');
  output = output.replace(/\b(?:\d{1,3}\.){3}\d{1,3}\b/g, (candidate) => {
    const parts = candidate.split('.').map(Number);
    return parts.every((part) => part >= 0 && part <= 255) ? '<IP_ADDRESS>' : candidate;
  });
  return output;
}

/** 对浏览器预览数据做结构化脱敏；不改变 MetricValue 的质量元数据。 */
export function sanitizePreviewReport(value: unknown, sensitive = false): unknown {
  if (Array.isArray(value)) return value.map((item) => sanitizePreviewReport(item, sensitive));
  if (isRecord(value)) {
    const metricValue = sensitive && isCompleteMetricValue(value);
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => {
        if (metricValue && key === 'value') return [key, null];
        const childSensitive = metricValue ? false : sensitive || isSensitiveKey(key);
        return [key, sanitizePreviewReport(child, childSensitive)];
      }),
    );
  }
  if (typeof value === 'string') return sensitive ? '<REDACTED>' : sanitizePreviewString(value);
  return sensitive ? null : value;
}

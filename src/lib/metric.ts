import type { MetricValue } from '../types/module';

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

import { describe, expect, it } from 'vitest';
import type { MetricValue } from '../types/module';
import {
  appendMetricGap,
  appendMetricPoint,
  collectionCount,
  metricMeta,
  metricNumber,
  metricState,
  metricText,
  sanitizePreviewReport,
} from './metric';

function metric<T>(quality: MetricValue<T>['quality'], value: T | null): MetricValue<T> {
  return {
    value,
    unit: '',
    quality,
    source: 'test',
    timestamp: 1,
    error: null,
  };
}

describe('metric helpers', () => {
  it('returns null for unsupported numeric metrics', () => {
    expect(metricNumber(metric('Unsupported', 42))).toBeNull();
  });

  it('preserves a valid zero numeric metric', () => {
    expect(metricNumber(metric('Good', 0))).toBe(0);
  });

  it('renders permission denied as Chinese status text', () => {
    expect(metricText(metric<string>('PermissionDenied', null))).toBe('权限不足');
    expect(metricState(metric('PermissionDenied', null))).toBe('permission');
  });

  it('keeps null samples as chart gaps', () => {
    const history: Array<number | null> = [10, 20];
    appendMetricPoint(history, metric('Unsupported', 30), 3);
    expect(history).toEqual([10, 20, null]);
  });

  it('truncates chart history from the beginning', () => {
    const history: Array<number | null> = [1, 2, 3];
    appendMetricPoint(history, metric('Good', 4), 3);
    expect(history).toEqual([2, 3, 4]);
  });

  it('appends a null gap for a failed sampling round', () => {
    const history: Array<number | null> = [1, 2];
    appendMetricGap(history, 3);
    expect(history).toEqual([1, 2, null]);
  });

  it('formats metric quality, source, and timestamp metadata', () => {
    expect(metricMeta(metric('Estimated', 42))).toBe(
      '质量：Estimated · 来源：test · 时间：1970-01-01T00:00:00.001Z',
    );
  });

  it('shows collection failure states instead of a false zero count', () => {
    expect(collectionCount([], metric('Unsupported', null), true)).toBe('暂不支持');
    expect(collectionCount([], metric('ReadError', null), true)).toBe('读取失败/数据无效');
    expect(collectionCount([], metric('Good', null), true)).toBe('0');
    expect(collectionCount([], undefined, false)).toBe('—');
  });

  it('structurally redacts sensitive preview keys and metric values', () => {
    const sanitized = sanitizePreviewReport({
      username: 'fixture-user',
      user: 'fixture-user-2',
      serial_number: { ...metric('Good', 'SN-1'), unit: '' },
      ordinary: 'C:\\Users\\fixture-user\\tool.exe via 192.0.2.10',
    }) as Record<string, unknown>;

    expect(sanitized.username).toBe('<REDACTED>');
    expect(sanitized.user).toBe('<REDACTED>');
    expect((sanitized.serial_number as Record<string, unknown>).value).toBeNull();
    expect(sanitized.ordinary).toBe('C:\\Users\\<USER>\\tool.exe via <IP_ADDRESS>');
  });
});

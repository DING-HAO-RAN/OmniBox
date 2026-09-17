import { describe, expect, it } from 'vitest';
import type { MetricValue } from '../types/module';
import { appendMetricPoint, metricNumber, metricState, metricText } from './metric';

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
});

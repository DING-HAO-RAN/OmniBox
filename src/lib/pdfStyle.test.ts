import { describe, expect, it } from 'vitest';
import { getPageDimensionsMm, millimetersToInches } from './pdfStyle';

describe('pdfStyle', () => {
  it('converts A4 dimensions and margins from millimeters to inches', () => {
    expect(getPageDimensionsMm('a4', 'portrait')).toEqual({ widthMm: 210, heightMm: 297 });
    expect(getPageDimensionsMm('a4', 'landscape')).toEqual({ widthMm: 297, heightMm: 210 });
    expect(millimetersToInches(25.4)).toBeCloseTo(1, 6);
    expect(millimetersToInches(20)).toBeCloseTo(0.787401, 5);
  });
});

/**
 * Markdown 转 PDF 模块的前后端共享设置类型。
 */

export type PageSizePreset = 'a3' | 'a4' | 'a5' | 'letter' | 'legal' | 'custom';
export type PageOrientation = 'portrait' | 'landscape';
export type CodeTheme = 'light' | 'dark';

/**
 * PDF 页面与内容排版设置。
 * 数值字段在前端使用 mm/pt，传入 Rust 后只在边界处换算为英寸。
 */
export interface PdfSettings {
  pageSize: PageSizePreset;
  orientation: PageOrientation;
  customWidthMm: number;
  customHeightMm: number;
  marginTopMm: number;
  marginRightMm: number;
  marginBottomMm: number;
  marginLeftMm: number;
  baseFontFamily: string;
  baseFontSizePt: number;
  lineHeight: number;
  paragraphSpacingPt: number;
  headingScale: number;
  textColor: string;
  linkColor: string;
  codeFontFamily: string;
  codeTheme: CodeTheme;
  imageMaxWidthPercent: number;
  showBackgrounds: boolean;
  showHeaderFooter: boolean;
  headerText: string;
  footerText: string;
  showPageNumber: boolean;
  scaleFactor: number;
}

/** 默认的 A4 阅读型文档设置。 */
export const DEFAULT_PDF_SETTINGS: PdfSettings = {
  pageSize: 'a4',
  orientation: 'portrait',
  customWidthMm: 210,
  customHeightMm: 297,
  marginTopMm: 20,
  marginRightMm: 20,
  marginBottomMm: 20,
  marginLeftMm: 20,
  baseFontFamily: '"Microsoft YaHei", "Segoe UI", sans-serif',
  baseFontSizePt: 10.5,
  lineHeight: 1.6,
  paragraphSpacingPt: 6,
  headingScale: 1,
  textColor: '#1f2937',
  linkColor: '#2563eb',
  codeFontFamily: 'Consolas, "Cascadia Mono", monospace',
  codeTheme: 'light',
  imageMaxWidthPercent: 100,
  showBackgrounds: true,
  showHeaderFooter: false,
  headerText: '',
  footerText: '',
  showPageNumber: false,
  scaleFactor: 1,
};

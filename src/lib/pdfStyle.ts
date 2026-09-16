/**
 * PDF 页面尺寸、单位换算和预览 CSS 变量。
 * 该文件只处理纯数据，避免把 DOM 或 Tauri 依赖带入可测试的核心逻辑。
 */

import type {
  PageOrientation,
  PageSizePreset,
  PdfSettings,
} from '../types/markdown-pdf';

export interface PageDimensionsMm {
  widthMm: number;
  heightMm: number;
}

export interface NativePrintValues {
  pageWidthIn: number;
  pageHeightIn: number;
  marginTopIn: number;
  marginRightIn: number;
  marginBottomIn: number;
  marginLeftIn: number;
  scaleFactor: number;
  landscape: boolean;
  printBackgrounds: boolean;
  printHeaderFooter: boolean;
  headerTitle: string;
  footerUri: string;
}

const PAGE_SIZES_MM: Record<Exclude<PageSizePreset, 'custom'>, PageDimensionsMm> = {
  a3: { widthMm: 297, heightMm: 420 },
  a4: { widthMm: 210, heightMm: 297 },
  a5: { widthMm: 148, heightMm: 210 },
  letter: { widthMm: 215.9, heightMm: 279.4 },
  legal: { widthMm: 215.9, heightMm: 355.6 },
};

/** 将毫米换算为 WebView2 使用的英寸。 */
export function millimetersToInches(valueMm: number): number {
  return valueMm / 25.4;
}

/** 获取应用方向后的物理页面尺寸。 */
export function getPageDimensionsMm(
  pageSize: PageSizePreset,
  orientation: PageOrientation,
  customWidthMm = 210,
  customHeightMm = 297,
): PageDimensionsMm {
  const base = pageSize === 'custom'
    ? { widthMm: customWidthMm, heightMm: customHeightMm }
    : PAGE_SIZES_MM[pageSize];

  return orientation === 'landscape'
    ? { widthMm: base.heightMm, heightMm: base.widthMm }
    : { widthMm: base.widthMm, heightMm: base.heightMm };
}

/** 把设置映射为预览容器使用的 CSS 自定义属性。 */
export function buildPdfCssVariables(settings: PdfSettings): Record<string, string> {
  const page = getPageDimensionsMm(
    settings.pageSize,
    settings.orientation,
    settings.customWidthMm,
    settings.customHeightMm,
  );

  return {
    '--pdf-page-width': `${page.widthMm}mm`,
    '--pdf-page-height': `${page.heightMm}mm`,
    '--pdf-margin-top': `${settings.marginTopMm}mm`,
    '--pdf-margin-right': `${settings.marginRightMm}mm`,
    '--pdf-margin-bottom': `${settings.marginBottomMm}mm`,
    '--pdf-margin-left': `${settings.marginLeftMm}mm`,
    '--pdf-base-font-family': settings.baseFontFamily,
    '--pdf-base-font-size': `${settings.baseFontSizePt}pt`,
    '--pdf-line-height': `${settings.lineHeight}`,
    '--pdf-paragraph-spacing': `${settings.paragraphSpacingPt}pt`,
    '--pdf-heading-scale': `${settings.headingScale}`,
    '--pdf-text-color': settings.textColor,
    '--pdf-link-color': settings.linkColor,
    '--pdf-code-font-family': settings.codeFontFamily,
    '--pdf-image-max-width': `${settings.imageMaxWidthPercent}%`,
    '--pdf-heading-1-size': `${settings.baseFontSizePt * 2.05 * settings.headingScale}pt`,
    '--pdf-heading-2-size': `${settings.baseFontSizePt * 1.65 * settings.headingScale}pt`,
    '--pdf-heading-3-size': `${settings.baseFontSizePt * 1.35 * settings.headingScale}pt`,
    '--pdf-heading-4-size': `${settings.baseFontSizePt * 1.15 * settings.headingScale}pt`,
    '--pdf-heading-5-size': `${settings.baseFontSizePt * settings.headingScale}pt`,
    '--pdf-heading-6-size': `${settings.baseFontSizePt * 0.9 * settings.headingScale}pt`,
  };
}

/** 把前端设置映射为 Rust/WebView2 使用的物理打印值。 */
export function buildNativePrintSettings(settings: PdfSettings): NativePrintValues {
  const page = getPageDimensionsMm(
    settings.pageSize,
    settings.orientation,
    settings.customWidthMm,
    settings.customHeightMm,
  );

  return {
    pageWidthIn: millimetersToInches(page.widthMm),
    pageHeightIn: millimetersToInches(page.heightMm),
    marginTopIn: millimetersToInches(settings.marginTopMm),
    marginRightIn: millimetersToInches(settings.marginRightMm),
    marginBottomIn: millimetersToInches(settings.marginBottomMm),
    marginLeftIn: millimetersToInches(settings.marginLeftMm),
    scaleFactor: settings.scaleFactor,
    landscape: settings.orientation === 'landscape',
    printBackgrounds: settings.showBackgrounds,
    printHeaderFooter: settings.showPageNumber,
    headerTitle: settings.headerText,
    // 空值可以避免 WebView2 在页脚中显示当前应用 URI。
    footerUri: '',
  };
}

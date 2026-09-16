import { isTauri, invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';
import type { PdfSettings } from '../types/markdown-pdf';

/** 通过桌面端 WebView2 原生打印引擎导出 PDF。 */
export function usePdfExport(waitUntilReady: () => Promise<void>) {
  const isExporting = ref(false);

  async function exportPdf(settings: PdfSettings): Promise<boolean> {
    if (!isTauri()) {
      throw new Error('PDF 导出需要运行 OmniBox 桌面版。');
    }

    const outputPath = await invoke<string | null>('choose_pdf_output');
    if (!outputPath) {
      return false;
    }

    isExporting.value = true;
    document.documentElement.classList.add('pdf-exporting');

    try {
      await waitUntilReady();
      await invoke('export_markdown_pdf', {
        outputPath,
        settings,
      });
      return true;
    } finally {
      document.documentElement.classList.remove('pdf-exporting');
      isExporting.value = false;
    }
  }

  return {
    isExporting,
    exportPdf,
  };
}

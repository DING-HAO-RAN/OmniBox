import { isTauri, invoke } from '@tauri-apps/api/core';
import { nextTick, onUnmounted, ref, watch, type Ref } from 'vue';
import { renderMarkdown } from '../lib/markdownRenderer';

interface MarkdownImageData {
  mimeType: string;
  dataBase64: string;
}

/**
 * 管理 Markdown 的防抖渲染、桌面端本地图片补齐以及导出前的就绪等待。
 */
export function useMarkdownPreview(
  source: Ref<string>,
  documentPath: Ref<string | null>,
  previewRoot: Ref<HTMLElement | null>,
) {
  const renderedHtml = ref('');
  const isRendering = ref(false);
  const renderError = ref('');

  let renderTimer: ReturnType<typeof setTimeout> | null = null;
  let renderVersion = 0;
  let pendingRender: Promise<void> = Promise.resolve();

  async function hydrateLocalImages(localImageSources: string[], version: number): Promise<void> {
    if (!isTauri() || !documentPath.value || !previewRoot.value || localImageSources.length === 0) {
      return;
    }

    const images = Array.from(previewRoot.value.querySelectorAll<HTMLImageElement>('img[data-markdown-source]'));
    let failedImageCount = 0;

    await Promise.all(
      images.map(async (image) => {
        const relativePath = image.dataset.markdownSource;
        if (!relativePath || !localImageSources.includes(relativePath)) {
          return;
        }

        try {
          const imageData = await invoke<MarkdownImageData>('read_markdown_image', {
            documentPath: documentPath.value,
            relativePath,
          });

          if (version !== renderVersion) {
            return;
          }

          image.src = `data:${imageData.mimeType};base64,${imageData.dataBase64}`;
          image.dataset.previewReady = 'true';
        } catch {
          failedImageCount += 1;
          image.removeAttribute('src');
          image.dataset.previewError = 'true';
        }
      }),
    );

    if (version === renderVersion && failedImageCount > 0) {
      renderError.value = `${failedImageCount} 张本地图片无法读取，已保留替代文本。`;
    }
  }

  async function renderNow(): Promise<void> {
    const version = ++renderVersion;
    isRendering.value = true;
    renderError.value = '';

    try {
      const result = renderMarkdown(source.value);
      renderedHtml.value = result.html;
      await nextTick();
      await hydrateLocalImages(result.localImageSources, version);
    } catch (error) {
      if (version === renderVersion) {
        renderedHtml.value = '';
        renderError.value = error instanceof Error ? error.message : 'Markdown 预览失败。';
      }
    } finally {
      if (version === renderVersion) {
        isRendering.value = false;
      }
    }
  }

  function scheduleRender(): void {
    if (renderTimer) {
      clearTimeout(renderTimer);
    }

    renderTimer = setTimeout(() => {
      renderTimer = null;
      pendingRender = renderNow();
    }, 120);
  }

  async function waitUntilReady(): Promise<void> {
    if (renderTimer) {
      clearTimeout(renderTimer);
      renderTimer = null;
      pendingRender = renderNow();
    }

    await pendingRender;
    await nextTick();

    if (typeof document !== 'undefined' && 'fonts' in document) {
      await document.fonts.ready;
    }

    const images = previewRoot.value ? Array.from(previewRoot.value.querySelectorAll<HTMLImageElement>('img')) : [];
    await Promise.all(
      images.map(async (image) => {
        if (image.complete || typeof image.decode !== 'function') {
          return;
        }

        try {
          await image.decode();
        } catch {
          // 图片加载失败不阻塞文本 PDF 导出。
        }
      }),
    );
  }

  watch([source, documentPath], scheduleRender, { immediate: true });

  onUnmounted(() => {
    if (renderTimer) {
      clearTimeout(renderTimer);
    }
  });

  return {
    renderedHtml,
    isRendering,
    renderError,
    waitUntilReady,
    renderNow,
  };
}

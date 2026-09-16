import DOMPurify from 'dompurify';
import hljs from 'highlight.js/lib/common';
import { Marked, type RendererObject } from 'marked';

export interface MarkdownRenderResult {
  html: string;
  localImageSources: string[];
}

// 只允许常用安全协议；无协议的相对路径会交给桌面端按文档目录解析。
const SAFE_URI_REGEXP = /^(?:(?:https?|mailto):|data:image\/(?:png|jpeg|gif|webp);base64,|[^:]*$)/i;

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => {
    const entities: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;',
    };
    return entities[character];
  });
}

function isLocalImageSource(source: string): boolean {
  return source.trim() !== '' && !/^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(source);
}

function isSafeImageSource(source: string): boolean {
  return /^(?:https?:|data:image\/(?:png|jpeg|gif|webp);base64,|[^:]*$)/i.test(source);
}

function createRenderer(): RendererObject {
  return {
    code({ text, lang }) {
      const language = (lang ?? '').trim().split(/\s+/u)[0] ?? '';
      const normalizedLanguage = language.replace(/[^a-z\d_+-]/gi, '').slice(0, 40);
      let highlightedCode = escapeHtml(text);

      if (normalizedLanguage && hljs.getLanguage(normalizedLanguage)) {
        try {
          highlightedCode = hljs.highlight(text, { language: normalizedLanguage }).value;
        } catch {
          // 未知或不完整的语法只回退为纯文本，不阻塞实时预览。
          highlightedCode = escapeHtml(text);
        }
      }

      const languageClass = normalizedLanguage ? ` language-${escapeHtml(normalizedLanguage)}` : '';
      return `<pre class="markdown-code-block"><code class="hljs${languageClass}">${highlightedCode}</code></pre>`;
    },

    image({ href, title, text }) {
      const source = href ?? '';
      const safeSource = isSafeImageSource(source) ? source : '';
      const titleAttribute = title ? ` title="${escapeHtml(title)}"` : '';
      return `<img src="${escapeHtml(safeSource)}" data-markdown-source="${escapeHtml(source)}" alt="${escapeHtml(text)}"${titleAttribute} loading="lazy">`;
    },
  };
}

const markdownParser = new Marked({ gfm: true, breaks: true });
markdownParser.use({ renderer: createRenderer() });

/**
 * 将 Markdown 转成可预览的安全 HTML，并保留本地图片引用供桌面端补齐数据。
 * 原始 HTML 统一经过 DOMPurify 清理，调用方可以安全地使用 v-html。
 */
export function renderMarkdown(markdown: string): MarkdownRenderResult {
  const rawHtml = markdownParser.parse(markdown) as string;

  const html = DOMPurify.sanitize(rawHtml, {
    ADD_ATTR: ['data-markdown-source', 'loading'],
    ALLOW_DATA_ATTR: true,
    ALLOWED_URI_REGEXP: SAFE_URI_REGEXP,
  });

  if (typeof document === 'undefined') {
    return { html, localImageSources: [] };
  }

  const container = document.createElement('div');
  container.innerHTML = html;
  const localImageSources = Array.from(container.querySelectorAll<HTMLImageElement>('img[data-markdown-source]'))
    .map((image) => image.dataset.markdownSource ?? '')
    .filter(isLocalImageSource)
    .filter((source, index, sources) => sources.indexOf(source) === index);

  return { html, localImageSources };
}

<template>
  <section
    ref="rootElement"
    class="markdown-preview-host"
    :style="cssVariables"
    aria-label="Markdown 实时预览"
  >
    <div class="markdown-panel-heading">
      <div>
        <span class="markdown-panel-kicker">PREVIEW</span>
        <h2>实时预览</h2>
      </div>
      <div class="markdown-preview-meta">
        <span>{{ pageLabel }}</span>
        <span v-if="isRendering" class="markdown-rendering-indicator">渲染中…</span>
      </div>
    </div>

    <div class="markdown-preview-scroll win11-scrollbar">
      <article
        class="markdown-paper"
        :class="{ 'markdown-paper-dark-code': settings.codeTheme === 'dark' }"
      >
        <div v-if="settings.showHeaderFooter && settings.headerText" class="pdf-print-header">
          {{ settings.headerText }}
        </div>

        <div v-if="html" class="markdown-content" v-html="html"></div>
        <div v-else class="markdown-empty-state">
          <span class="markdown-empty-icon">✦</span>
          <p>在左侧输入 Markdown，预览会实时更新</p>
        </div>

        <div v-if="settings.showHeaderFooter && settings.footerText" class="pdf-print-footer">
          {{ settings.footerText }}
        </div>
      </article>
    </div>

    <div v-if="error" class="markdown-preview-warning">{{ error }}</div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { buildPdfCssVariables, getPageDimensionsMm } from '../../lib/pdfStyle';
import type { PdfSettings } from '../../types/markdown-pdf';

const props = defineProps<{
  html: string;
  settings: PdfSettings;
  isRendering: boolean;
  error?: string;
}>();

const emit = defineEmits<{
  'root-ready': [element: HTMLElement | null];
}>();

const rootElement = ref<HTMLElement | null>(null);
const cssVariables = computed(() => buildPdfCssVariables(props.settings));
const pageLabel = computed(() => {
  const page = getPageDimensionsMm(
    props.settings.pageSize,
    props.settings.orientation,
    props.settings.customWidthMm,
    props.settings.customHeightMm,
  );
  const name = props.settings.pageSize === 'custom' ? '自定义' : props.settings.pageSize.toUpperCase();
  return `${name} ${page.widthMm} × ${page.heightMm} mm`;
});

onMounted(() => emit('root-ready', rootElement.value));
onUnmounted(() => emit('root-ready', null));
</script>

<style scoped>
.markdown-preview-host {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex: 1 1 50%;
  flex-direction: column;
  background: rgba(20, 27, 38, 0.78);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 14px;
  overflow: hidden;
}

.markdown-panel-heading {
  min-height: 58px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
}

.markdown-panel-kicker {
  display: block;
  margin-bottom: 3px;
  color: #34d399;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.16em;
}

h2 {
  margin: 0;
  color: #f3f4f6;
  font-size: 13px;
  font-weight: 650;
}

.markdown-preview-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: rgba(203, 213, 225, 0.54);
  font-size: 10px;
}

.markdown-rendering-indicator {
  color: #60a5fa;
}

.markdown-preview-scroll {
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding: 22px;
  background:
    radial-gradient(circle at 15% 10%, rgba(59, 130, 246, 0.08), transparent 34%),
    #101722;
}

.markdown-paper {
  box-sizing: border-box;
  width: min(var(--pdf-page-width), calc(100% - 4px));
  min-height: var(--pdf-page-height);
  margin: 0 auto;
  padding: var(--pdf-margin-top) var(--pdf-margin-right) var(--pdf-margin-bottom) var(--pdf-margin-left);
  overflow-wrap: anywhere;
  color: var(--pdf-text-color);
  background: #ffffff;
  box-shadow: 0 14px 42px rgba(0, 0, 0, 0.3);
  font-family: var(--pdf-base-font-family);
  font-size: var(--pdf-base-font-size);
  line-height: var(--pdf-line-height);
}

.markdown-paper-dark-code {
  --markdown-code-background: #111827;
  --markdown-code-foreground: #e5e7eb;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  color: #111827;
  break-after: avoid;
  font-weight: 750;
  line-height: 1.25;
}

.markdown-content :deep(h1) { font-size: var(--pdf-heading-1-size); }
.markdown-content :deep(h2) { font-size: var(--pdf-heading-2-size); }
.markdown-content :deep(h3) { font-size: var(--pdf-heading-3-size); }
.markdown-content :deep(h4) { font-size: var(--pdf-heading-4-size); }
.markdown-content :deep(h5) { font-size: var(--pdf-heading-5-size); }
.markdown-content :deep(h6) { font-size: var(--pdf-heading-6-size); }

.markdown-content :deep(h1:first-child),
.markdown-content :deep(h2:first-child),
.markdown-content :deep(h3:first-child) {
  margin-top: 0;
}

.markdown-content :deep(p),
.markdown-content :deep(ul),
.markdown-content :deep(ol),
.markdown-content :deep(blockquote),
.markdown-content :deep(pre),
.markdown-content :deep(table),
.markdown-content :deep(hr) {
  margin-top: 0;
  margin-bottom: var(--pdf-paragraph-spacing);
}

.markdown-content :deep(a) {
  color: var(--pdf-link-color);
  text-decoration: none;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(blockquote) {
  padding: 4px 14px;
  color: #4b5563;
  border-left: 3px solid #93c5fd;
  background: #eff6ff;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  padding-left: 1.5em;
}

.markdown-content :deep(li) {
  margin: 0.15em 0;
}

.markdown-content :deep(li input[type='checkbox']) {
  margin-right: 0.45em;
  accent-color: #2563eb;
}

.markdown-content :deep(.markdown-code-block) {
  overflow: auto;
  padding: 12px 14px;
  color: #334155;
  background: var(--markdown-code-background, #f1f5f9);
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  font-family: var(--pdf-code-font-family);
  font-size: 0.92em;
  line-height: 1.55;
  break-inside: avoid;
}

.markdown-paper-dark-code .markdown-content :deep(.markdown-code-block) {
  color: var(--markdown-code-foreground);
  border-color: #374151;
}

.markdown-content :deep(code:not(.hljs)) {
  padding: 0.12em 0.35em;
  color: #9f1239;
  background: #fff1f2;
  border-radius: 4px;
  font-family: var(--pdf-code-font-family);
  font-size: 0.9em;
}

.markdown-paper-dark-code .markdown-content :deep(code:not(.hljs)) {
  color: #fecdd3;
  background: #4c0519;
}

.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95em;
  break-inside: avoid;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  padding: 6px 8px;
  text-align: left;
  vertical-align: top;
  border: 1px solid #cbd5e1;
}

.markdown-content :deep(th) {
  color: #1e3a8a;
  background: #dbeafe;
  font-weight: 700;
}

.markdown-content :deep(img) {
  display: block;
  max-width: var(--pdf-image-max-width);
  height: auto;
  margin: 12px auto;
  break-inside: avoid;
}

.markdown-content :deep(hr) {
  border: 0;
  border-top: 1px solid #cbd5e1;
}

.pdf-print-header,
.pdf-print-footer {
  color: #64748b;
  font-size: 0.8em;
  letter-spacing: 0.02em;
}

.pdf-print-header {
  margin-bottom: 18px;
  padding-bottom: 8px;
  border-bottom: 1px solid #e2e8f0;
}

.pdf-print-footer {
  margin-top: 18px;
  padding-top: 8px;
  border-top: 1px solid #e2e8f0;
}

.markdown-empty-state {
  min-height: 360px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  color: #94a3b8;
  text-align: center;
}

.markdown-empty-icon {
  display: inline-flex;
  width: 36px;
  height: 36px;
  align-items: center;
  justify-content: center;
  margin-bottom: 12px;
  color: #60a5fa;
  background: #eff6ff;
  border-radius: 12px;
  font-size: 18px;
}

.markdown-empty-state p {
  margin: 0;
  font-size: 12px;
}

.markdown-preview-warning {
  padding: 7px 16px;
  color: #fbbf24;
  background: rgba(120, 53, 15, 0.22);
  border-top: 1px solid rgba(251, 191, 36, 0.16);
  font-size: 10px;
}
</style>

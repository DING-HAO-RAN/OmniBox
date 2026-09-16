<template>
  <section class="markdown-pdf-view">
    <header class="markdown-pdf-toolbar">
      <div class="markdown-pdf-title-block">
        <div class="markdown-pdf-title-row">
          <span class="markdown-pdf-eyebrow">DOCUMENT WORKBENCH</span>
          <h1>Markdown 转 PDF</h1>
          <span class="markdown-document-chip">{{ documentName }}<i v-if="isDirty"></i></span>
        </div>
        <p>边写边看版式，桌面版使用原生 WebView2 输出 PDF</p>
      </div>

      <div class="markdown-pdf-actions">
        <button type="button" class="markdown-toolbar-button" title="新建文档" @click="handleNewDocument">
          <FilePlus2 :size="14" />
          <span>新建</span>
        </button>
        <button type="button" class="markdown-toolbar-button" title="打开 Markdown 文件" @click="handleOpenDocument">
          <FolderOpen :size="14" />
          <span>打开</span>
        </button>
        <button type="button" class="markdown-toolbar-button" title="保存 Markdown 文件" @click="handleSaveDocument">
          <Save :size="14" />
          <span>保存</span>
        </button>
        <button type="button" class="markdown-toolbar-button" title="Markdown 另存为" @click="handleSaveAsDocument">
          <SaveAll :size="14" />
          <span>另存</span>
        </button>
        <span class="markdown-toolbar-divider"></span>
        <button type="button" class="markdown-toolbar-button" :class="{ active: settingsVisible }" title="PDF 页面设置" @click="settingsVisible = !settingsVisible">
          <SlidersHorizontal :size="14" />
          <span>设置</span>
        </button>
        <button type="button" class="markdown-export-button" :disabled="isExporting" @click="handleExportPdf">
          <FileDown :size="15" />
          <span>{{ isExporting ? '导出中…' : '导出 PDF' }}</span>
        </button>
      </div>
    </header>

    <div class="markdown-pdf-content">
      <div class="markdown-pdf-edit-preview">
        <MarkdownEditor :model-value="content" @update:model-value="content = $event" />
        <MarkdownPreview
          :html="renderedHtml"
          :settings="settings"
          :is-rendering="isRendering"
          :error="renderError"
          @root-ready="handlePreviewRoot"
        />
      </div>

      <Transition name="settings-slide">
        <PdfSettingsPanel
          v-if="settingsVisible"
          v-model="settings"
          @reset="resetSettings"
        />
      </Transition>
    </div>

    <footer class="markdown-pdf-statusbar">
      <span class="markdown-status-main">
        <span class="markdown-status-dot" :class="{ busy: isRendering || isExporting }"></span>
        {{ statusText }}
      </span>
      <span class="markdown-status-help">GFM · 代码高亮 · 本地图片 · 页码与背景设置</span>
    </footer>

    <input
      ref="browserFileInput"
      class="markdown-hidden-input"
      type="file"
      accept=".md,.markdown,text/markdown"
      @change="handleBrowserFile"
    >
  </section>
</template>

<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { FileDown, FilePlus2, FolderOpen, Save, SaveAll, SlidersHorizontal } from 'lucide-vue-next';
import MarkdownEditor from '../components/markdown-pdf/MarkdownEditor.vue';
import MarkdownPreview from '../components/markdown-pdf/MarkdownPreview.vue';
import PdfSettingsPanel from '../components/markdown-pdf/PdfSettingsPanel.vue';
import { useMarkdownDocument } from '../composables/useMarkdownDocument';
import { useMarkdownPreview } from '../composables/useMarkdownPreview';
import { usePdfExport } from '../composables/usePdfExport';
import { toast } from '../composables/useToast';
import { DEFAULT_PDF_SETTINGS, type PdfSettings } from '../types/markdown-pdf';

const fence = String.fromCharCode(96).repeat(3);
const DEFAULT_MARKDOWN = [
  '# OmniBox 文档示例',
  '',
  '这是一个支持实时预览的 **Markdown 转 PDF** 工作区。',
  '',
  '## 快速开始',
  '',
  '- 在左侧编辑 Markdown 源文档',
  '- 在右侧查看排版效果',
  '- 打开设置调整纸张、页边距、字号和字体',
  '- 点击「导出 PDF」选择输出位置',
  '',
  '## 常用能力',
  '',
  '| 功能 | 状态 |',
  '| --- | --- |',
  '| GFM 表格与任务清单 | 支持 |',
  '| 代码高亮 | 支持 |',
  '| 本地图片 | 桌面版支持 |',
  '| 自定义页面设置 | 支持 |',
  '',
  '- [x] 调整 A4 / A3 / Letter 页面',
  '- [x] 设置四边页边距',
  '- [ ] 添加更多内容',
  '',
  fence + 'ts',
  'const message = \"Hello, PDF\";',
  'console.log(message);',
  fence,
].join('\\n');

const {
  content,
  documentPath,
  documentName,
  isDirty,
  createNewDocument,
  openDocument,
  openBrowserFile,
  saveDocument,
  saveAsDocument,
} = useMarkdownDocument(DEFAULT_MARKDOWN);
const settings = ref<PdfSettings>({ ...DEFAULT_PDF_SETTINGS });
const settingsVisible = ref(false);
const browserFileInput = ref<HTMLInputElement | null>(null);
const previewRoot = ref<HTMLElement | null>(null);
const {
  renderedHtml,
  isRendering,
  renderError,
  waitUntilReady,
} = useMarkdownPreview(content, documentPath, previewRoot);
const { isExporting, exportPdf } = usePdfExport(waitUntilReady);

const statusText = computed(() => {
  if (isExporting.value) {
    return '正在生成 PDF…';
  }
  if (isRendering.value) {
    return '正在更新预览…';
  }
  if (isDirty.value) {
    return '有未保存修改';
  }
  return documentPath.value ? '文档已保存' : '示例文档';
});

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function canDiscardChanges(): boolean {
  return !isDirty.value || window.confirm('当前文档有未保存修改，确定继续吗？');
}

async function handleNewDocument(): Promise<void> {
  if (!canDiscardChanges()) {
    return;
  }
  createNewDocument('');
  toast.info('已新建文档', '可以从左侧开始编写 Markdown。');
}

async function handleOpenDocument(): Promise<void> {
  if (!canDiscardChanges()) {
    return;
  }

  if (!isTauri()) {
    browserFileInput.value?.click();
    return;
  }

  try {
    if (await openDocument()) {
      toast.success('Markdown 已打开', documentName.value);
    }
  } catch (error) {
    toast.error('打开失败', errorMessage(error));
  }
}

async function handleBrowserFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file) {
    return;
  }

  try {
    await openBrowserFile(file);
    toast.success('Markdown 已打开', documentName.value);
  } catch (error) {
    toast.error('打开失败', errorMessage(error));
  }
}

async function handleSaveDocument(): Promise<void> {
  try {
    if (await saveDocument()) {
      toast.success('Markdown 已保存', documentName.value);
    }
  } catch (error) {
    toast.error('保存失败', errorMessage(error));
  }
}

async function handleSaveAsDocument(): Promise<void> {
  try {
    if (await saveAsDocument()) {
      toast.success('Markdown 已另存', documentName.value);
    }
  } catch (error) {
    toast.error('另存失败', errorMessage(error));
  }
}

async function handleExportPdf(): Promise<void> {
  try {
    if (await exportPdf(settings.value)) {
      toast.success('PDF 导出成功', '文件已保存到你选择的位置。');
    }
  } catch (error) {
    toast.error('PDF 导出失败', errorMessage(error));
  }
}

function resetSettings(): void {
  settings.value = { ...DEFAULT_PDF_SETTINGS };
  toast.info('已恢复默认 PDF 设置');
}

function handlePreviewRoot(element: HTMLElement | null): void {
  previewRoot.value = element;
}

function handleKeyboardShortcut(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault();
    void handleSaveDocument();
  }
}

onMounted(() => window.addEventListener('keydown', handleKeyboardShortcut));
onBeforeUnmount(() => window.removeEventListener('keydown', handleKeyboardShortcut));
</script>

<style scoped>
.markdown-pdf-view {
  min-width: 0;
  min-height: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px;
  box-sizing: border-box;
  color: #e5e7eb;
  background:
    radial-gradient(circle at 85% 0%, rgba(37, 99, 235, 0.11), transparent 32%),
    #0e131b;
}

.markdown-pdf-toolbar {
  min-height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
}

.markdown-pdf-title-block { min-width: 0; }
.markdown-pdf-title-row { min-width: 0; display: flex; align-items: center; gap: 10px; }
.markdown-pdf-eyebrow { color: #60a5fa; font-size: 9px; font-weight: 750; letter-spacing: .12em; white-space: nowrap; }
.markdown-pdf-title-row h1 { margin: 0; color: #f8fafc; font-size: 19px; font-weight: 700; letter-spacing: -.02em; white-space: nowrap; }
.markdown-pdf-title-block p { margin: 5px 0 0; color: #64748b; font-size: 10px; }
.markdown-document-chip { max-width: 170px; overflow: hidden; padding: 4px 7px; color: #a5b4fc; background: rgba(99,102,241,.12); border: 1px solid rgba(129,140,248,.2); border-radius: 6px; font-family: Consolas, monospace; font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.markdown-document-chip i { display: inline-block; width: 5px; height: 5px; margin-left: 5px; vertical-align: middle; background: #fbbf24; border-radius: 50%; }

.markdown-pdf-actions { display: flex; align-items: center; gap: 5px; flex-shrink: 0; }
.markdown-toolbar-button, .markdown-export-button { display: inline-flex; align-items: center; gap: 6px; padding: 7px 8px; color: #aeb9c8; background: rgba(255,255,255,.035); border: 1px solid rgba(255,255,255,.09); border-radius: 7px; font-size: 10px; cursor: pointer; transition: .16s ease; }
.markdown-toolbar-button:hover, .markdown-toolbar-button.active { color: #eff6ff; background: rgba(96,165,250,.11); border-color: rgba(96,165,250,.34); }
.markdown-toolbar-button:active, .markdown-export-button:active { transform: translateY(1px); }
.markdown-toolbar-divider { width: 1px; height: 20px; margin: 0 4px; background: rgba(255,255,255,.12); }
.markdown-export-button { padding: 8px 11px; color: #eff6ff; background: linear-gradient(135deg, #2563eb, #0284c7); border-color: rgba(147,197,253,.3); font-weight: 650; box-shadow: 0 5px 18px rgba(37,99,235,.25); }
.markdown-export-button:hover { background: linear-gradient(135deg, #3b82f6, #0ea5e9); }
.markdown-export-button:disabled { opacity: .55; cursor: wait; }

.markdown-pdf-content { min-height: 0; flex: 1; display: flex; gap: 12px; }
.markdown-pdf-edit-preview { min-width: 0; min-height: 0; flex: 1; display: flex; gap: 12px; }
.markdown-hidden-input { display: none; }

.markdown-pdf-statusbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 17px; color: #64748b; font-size: 10px; }
.markdown-status-main { display: inline-flex; align-items: center; gap: 6px; }
.markdown-status-dot { width: 6px; height: 6px; display: inline-block; background: #34d399; border-radius: 50%; box-shadow: 0 0 8px rgba(52,211,153,.55); }
.markdown-status-dot.busy { background: #60a5fa; box-shadow: 0 0 8px rgba(96,165,250,.6); animation: markdown-pulse 1.1s ease-in-out infinite; }
.markdown-status-help { color: #475569; white-space: nowrap; }

.settings-slide-enter-active, .settings-slide-leave-active { transition: opacity .16s ease, transform .16s ease; }
.settings-slide-enter-from, .settings-slide-leave-to { opacity: 0; transform: translateX(10px); }

@keyframes markdown-pulse { 50% { opacity: .35; } }

@media (max-width: 1120px) {
  .markdown-pdf-title-block p, .markdown-toolbar-button span { display: none; }
  .markdown-pdf-title-row { gap: 7px; }
  .markdown-pdf-title-row h1 { font-size: 16px; }
}

@media (max-width: 900px) {
  .markdown-pdf-view { padding: 12px; }
  .markdown-pdf-toolbar { align-items: flex-start; flex-direction: column; gap: 9px; }
  .markdown-pdf-actions { width: 100%; overflow-x: auto; padding-bottom: 2px; }
  .markdown-pdf-content { flex-direction: column; }
  .markdown-pdf-edit-preview { min-height: 600px; }
  .pdf-settings-panel { width: 100%; min-width: 0; max-height: 360px; }
  .markdown-status-help { display: none; }
}
</style>

import { isTauri, invoke } from '@tauri-apps/api/core';
import { computed, ref } from 'vue';

export interface MarkdownDocumentData {
  path: string;
  content: string;
}

const DEFAULT_FILE_NAME = '未命名.md';

function fileNameFromPath(path: string): string {
  return path.split(/[\\/]/u).pop() || DEFAULT_FILE_NAME;
}

function downloadTextFile(fileName: string, content: string): void {
  const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName.toLowerCase().endsWith('.md') ? fileName : fileName + '.md';
  anchor.click();
  URL.revokeObjectURL(url);
}

/** 统一管理 Markdown 文档的打开、保存、另存为和未保存状态。 */
export function useMarkdownDocument(initialContent: string) {
  const content = ref(initialContent);
  const savedContent = ref(initialContent);
  const documentPath = ref<string | null>(null);
  const displayName = ref(DEFAULT_FILE_NAME);
  const isDirty = computed(() => content.value !== savedContent.value);
  const documentName = computed(() => displayName.value);

  function replaceDocument(document: MarkdownDocumentData): void {
    content.value = document.content;
    savedContent.value = document.content;
    documentPath.value = document.path || null;
    displayName.value = document.path ? fileNameFromPath(document.path) : DEFAULT_FILE_NAME;
  }

  function createNewDocument(newContent = ''): void {
    content.value = newContent;
    savedContent.value = newContent;
    documentPath.value = null;
    displayName.value = DEFAULT_FILE_NAME;
  }

  async function openDocument(): Promise<boolean> {
    if (!isTauri()) {
      return false;
    }

    const path = await invoke<string | null>('choose_markdown_file');
    if (!path) {
      return false;
    }

    const document = await invoke<MarkdownDocumentData>('read_markdown_file', { path });
    replaceDocument(document);
    return true;
  }

  async function openBrowserFile(file: File): Promise<void> {
    replaceDocument({
      path: '',
      content: await file.text(),
    });
    displayName.value = file.name || DEFAULT_FILE_NAME;
  }

  async function saveDocument(): Promise<boolean> {
    if (!documentPath.value) {
      return saveAsDocument();
    }

    if (isTauri()) {
      const document = await invoke<MarkdownDocumentData>('save_markdown_file', {
        path: documentPath.value,
        content: content.value,
      });
      replaceDocument(document);
      return true;
    }

    downloadTextFile(displayName.value, content.value);
    savedContent.value = content.value;
    return true;
  }

  async function saveAsDocument(): Promise<boolean> {
    if (isTauri()) {
      const path = await invoke<string | null>('choose_markdown_output');
      if (!path) {
        return false;
      }

      const document = await invoke<MarkdownDocumentData>('save_markdown_file', {
        path,
        content: content.value,
      });
      replaceDocument(document);
      return true;
    }

    downloadTextFile(displayName.value, content.value);
    savedContent.value = content.value;
    return true;
  }

  return {
    content,
    documentPath,
    documentName,
    isDirty,
    createNewDocument,
    openDocument,
    openBrowserFile,
    saveDocument,
    saveAsDocument,
  };
}

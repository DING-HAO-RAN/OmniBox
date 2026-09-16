<template>
  <section class="markdown-editor-pane" aria-label="Markdown 编辑器">
    <div class="markdown-panel-heading">
      <div>
        <span class="markdown-panel-kicker">SOURCE</span>
        <h2>Markdown 编辑</h2>
      </div>
      <span class="markdown-editor-count">{{ lineCount }} 行 · {{ characterCount }} 字符</span>
    </div>

    <textarea
      class="markdown-editor-textarea"
      :value="modelValue"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      aria-label="Markdown 源文档"
      placeholder="# 开始编写文档\n\n支持标题、列表、表格、代码块、任务清单和图片。"
      @input="handleInput"
    ></textarea>

    <div class="markdown-editor-footer">
      <span>支持 GFM 语法</span>
      <span>Ctrl / Cmd + S 保存</span>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const props = defineProps<{
  modelValue: string;
}>();

const lineCount = computed(() => Math.max(1, props.modelValue.split(/\r?\n/u).length));
const characterCount = computed(() => props.modelValue.length);

function handleInput(event: Event): void {
  emit('update:modelValue', (event.target as HTMLTextAreaElement).value);
}
</script>

<style scoped>
.markdown-editor-pane {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex: 1 1 50%;
  flex-direction: column;
  background: rgba(11, 16, 24, 0.72);
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
  color: #60a5fa;
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

.markdown-editor-count {
  flex-shrink: 0;
  color: rgba(203, 213, 225, 0.52);
  font-family: Consolas, "Cascadia Mono", monospace;
  font-size: 10px;
}

.markdown-editor-textarea {
  min-height: 0;
  flex: 1;
  width: 100%;
  box-sizing: border-box;
  resize: none;
  border: 0;
  outline: none;
  padding: 18px;
  color: #dbeafe;
  background: transparent;
  caret-color: #60a5fa;
  font-family: Consolas, "Cascadia Mono", "Microsoft YaHei", monospace;
  font-size: 13px;
  line-height: 1.75;
  tab-size: 2;
  user-select: text;
}

.markdown-editor-textarea::selection {
  color: #eff6ff;
  background: rgba(37, 99, 235, 0.42);
}

.markdown-editor-textarea::placeholder {
  color: rgba(148, 163, 184, 0.5);
}

.markdown-editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 16px;
  color: rgba(148, 163, 184, 0.58);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 10px;
}
</style>

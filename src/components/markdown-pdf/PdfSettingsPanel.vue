<template>
  <aside class="pdf-settings-panel" aria-label="PDF 文件设置">
    <div class="pdf-settings-heading">
      <div>
        <span class="markdown-panel-kicker">LAYOUT</span>
        <h2>PDF 设置</h2>
      </div>
      <button type="button" class="pdf-settings-reset" @click="$emit('reset')">恢复默认</button>
    </div>

    <div class="pdf-settings-scroll win11-scrollbar">
      <section class="pdf-setting-section">
        <h3>页面</h3>
        <div class="pdf-setting-grid">
          <label class="pdf-setting-field pdf-setting-field-wide">
            <span>纸张</span>
            <select :value="modelValue.pageSize" @change="updatePageSize">
              <option v-for="page in pageSizes" :key="page.value" :value="page.value">{{ page.label }}</option>
            </select>
          </label>
          <label class="pdf-setting-field">
            <span>方向</span>
            <select :value="modelValue.orientation" @change="updateOrientation">
              <option value="portrait">纵向</option>
              <option value="landscape">横向</option>
            </select>
          </label>
          <template v-if="modelValue.pageSize === 'custom'">
            <label class="pdf-setting-field"><span>宽度 (mm)</span><input :value="modelValue.customWidthMm" type="number" min="50" max="1000" step="1" @input="updateNumber('customWidthMm', $event)"></label>
            <label class="pdf-setting-field"><span>高度 (mm)</span><input :value="modelValue.customHeightMm" type="number" min="50" max="1000" step="1" @input="updateNumber('customHeightMm', $event)"></label>
          </template>
        </div>
      </section>

      <section class="pdf-setting-section">
        <div class="pdf-setting-section-title-row">
          <h3>页边距</h3>
          <label class="pdf-checkbox pdf-checkbox-small"><input v-model="marginSync" type="checkbox"><span>四边联动</span></label>
        </div>
        <div class="pdf-margin-grid">
          <label class="pdf-setting-field"><span>上</span><input :value="modelValue.marginTopMm" type="number" min="0" max="100" step="0.5" @input="updateMargin('marginTopMm', $event)"></label>
          <label class="pdf-setting-field"><span>右</span><input :value="modelValue.marginRightMm" type="number" min="0" max="100" step="0.5" @input="updateMargin('marginRightMm', $event)"></label>
          <label class="pdf-setting-field"><span>下</span><input :value="modelValue.marginBottomMm" type="number" min="0" max="100" step="0.5" @input="updateMargin('marginBottomMm', $event)"></label>
          <label class="pdf-setting-field"><span>左</span><input :value="modelValue.marginLeftMm" type="number" min="0" max="100" step="0.5" @input="updateMargin('marginLeftMm', $event)"></label>
        </div>
      </section>

      <section class="pdf-setting-section">
        <h3>正文与层级</h3>
        <div class="pdf-setting-grid">
          <label class="pdf-setting-field pdf-setting-field-wide"><span>基础字体</span><input list="pdf-base-fonts" :value="modelValue.baseFontFamily" @input="updateBaseFont"><datalist id="pdf-base-fonts"><option v-for="font in fontOptions" :key="font.value" :value="font.value" :label="font.label"></option></datalist></label>
          <label class="pdf-setting-field"><span>字号 (pt)</span><input :value="modelValue.baseFontSizePt" type="number" min="6" max="48" step="0.5" @input="updateNumber('baseFontSizePt', $event)"></label>
          <label class="pdf-setting-field"><span>行高</span><input :value="modelValue.lineHeight" type="number" min="1" max="3" step="0.05" @input="updateNumber('lineHeight', $event)"></label>
          <label class="pdf-setting-field"><span>段后距 (pt)</span><input :value="modelValue.paragraphSpacingPt" type="number" min="0" max="48" step="0.5" @input="updateNumber('paragraphSpacingPt', $event)"></label>
          <label class="pdf-setting-field"><span>标题缩放</span><input :value="modelValue.headingScale" type="number" min="0.6" max="1.6" step="0.05" @input="updateNumber('headingScale', $event)"></label>
        </div>
      </section>

      <section class="pdf-setting-section">
        <h3>颜色与代码</h3>
        <div class="pdf-setting-grid">
          <label class="pdf-setting-field"><span>正文颜色</span><span class="pdf-color-input"><input :value="modelValue.textColor" type="color" @input="updateColor('textColor', $event)"><code>{{ modelValue.textColor }}</code></span></label>
          <label class="pdf-setting-field"><span>链接颜色</span><span class="pdf-color-input"><input :value="modelValue.linkColor" type="color" @input="updateColor('linkColor', $event)"><code>{{ modelValue.linkColor }}</code></span></label>
          <label class="pdf-setting-field pdf-setting-field-wide"><span>代码字体</span><input list="pdf-code-fonts" :value="modelValue.codeFontFamily" @input="updateCodeFont"><datalist id="pdf-code-fonts"><option v-for="font in codeFontOptions" :key="font.value" :value="font.value" :label="font.label"></option></datalist></label>
          <label class="pdf-setting-field"><span>代码主题</span><select :value="modelValue.codeTheme" @change="updateCodeTheme"><option value="light">浅色</option><option value="dark">深色</option></select></label>
          <label class="pdf-setting-field"><span>图片最大宽度 (%)</span><input :value="modelValue.imageMaxWidthPercent" type="number" min="25" max="100" step="5" @input="updateNumber('imageMaxWidthPercent', $event)"></label>
        </div>
      </section>

      <section class="pdf-setting-section">
        <h3>页眉与页脚</h3>
        <label class="pdf-checkbox"><input :checked="modelValue.showHeaderFooter" type="checkbox" @change="updateBoolean('showHeaderFooter', $event)"><span>显示自定义页眉 / 页脚</span></label>
        <div v-if="modelValue.showHeaderFooter" class="pdf-setting-stack">
          <label class="pdf-setting-field pdf-setting-field-wide"><span>页眉文本</span><input :value="modelValue.headerText" type="text" placeholder="例如：项目文档" @input="updateText('headerText', $event)"></label>
          <label class="pdf-setting-field pdf-setting-field-wide"><span>页脚文本</span><input :value="modelValue.footerText" type="text" placeholder="例如：内部资料" @input="updateText('footerText', $event)"></label>
          <label class="pdf-checkbox"><input :checked="modelValue.showPageNumber" type="checkbox" @change="updateBoolean('showPageNumber', $event)"><span>使用 PDF 引擎添加页码</span></label>
        </div>
        <p class="pdf-setting-hint">页码由桌面版 WebView2 打印引擎添加；浏览器预览不会伪造页码。</p>
      </section>

      <section class="pdf-setting-section">
        <h3>输出</h3>
        <label class="pdf-checkbox"><input :checked="modelValue.showBackgrounds" type="checkbox" @change="updateBoolean('showBackgrounds', $event)"><span>打印背景颜色与背景图</span></label>
        <label class="pdf-setting-field pdf-setting-field-wide pdf-setting-scale-field"><span>打印缩放 ({{ Math.round(modelValue.scaleFactor * 100) }}%)</span><input :value="modelValue.scaleFactor" type="range" min="0.5" max="1.5" step="0.05" @input="updateNumber('scaleFactor', $event)"></label>
      </section>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import type { CodeTheme, PageOrientation, PageSizePreset, PdfSettings } from '../../types/markdown-pdf';

const props = defineProps<{ modelValue: PdfSettings }>();
const emit = defineEmits<{
  'update:modelValue': [value: PdfSettings];
  reset: [];
}>();

const marginSync = ref(false);
const pageSizes: Array<{ value: PageSizePreset; label: string }> = [
  { value: 'a3', label: 'A3 · 297 × 420 mm' },
  { value: 'a4', label: 'A4 · 210 × 297 mm' },
  { value: 'a5', label: 'A5 · 148 × 210 mm' },
  { value: 'letter', label: 'Letter · 8.5 × 11 in' },
  { value: 'legal', label: 'Legal · 8.5 × 14 in' },
  { value: 'custom', label: '自定义尺寸' },
];
const fontOptions = [
  { value: '"Microsoft YaHei", "Segoe UI", sans-serif', label: '微软雅黑 / Segoe UI' },
  { value: '"SimSun", "宋体", serif', label: '宋体' },
  { value: '"SimHei", "黑体", sans-serif', label: '黑体' },
  { value: '"Segoe UI", sans-serif', label: 'Segoe UI' },
  { value: 'Arial, sans-serif', label: 'Arial' },
  { value: 'Georgia, serif', label: 'Georgia' },
  { value: '"Times New Roman", serif', label: 'Times New Roman' },
];
const codeFontOptions = [
  { value: 'Consolas, "Cascadia Mono", monospace', label: 'Consolas / Cascadia Mono' },
  { value: '"Cascadia Mono", Consolas, monospace', label: 'Cascadia Mono' },
  { value: '"Courier New", monospace', label: 'Courier New' },
  { value: 'monospace', label: '系统等宽字体' },
];

function updateSetting<K extends keyof PdfSettings>(key: K, value: PdfSettings[K]): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value });
}
function numberFromEvent(event: Event): number {
  const value = Number((event.target as HTMLInputElement).value);
  return Number.isFinite(value) ? value : 0;
}
function updateNumber(key: keyof PdfSettings, event: Event): void {
  updateSetting(key, numberFromEvent(event) as never);
}
function updateText(key: keyof PdfSettings, event: Event): void {
  updateSetting(key, (event.target as HTMLInputElement).value as never);
}
function updateColor(key: 'textColor' | 'linkColor', event: Event): void {
  updateSetting(key, (event.target as HTMLInputElement).value);
}
function updateBoolean(key: 'showBackgrounds' | 'showHeaderFooter' | 'showPageNumber', event: Event): void {
  updateSetting(key, (event.target as HTMLInputElement).checked);
}
function updatePageSize(event: Event): void {
  updateSetting('pageSize', (event.target as HTMLSelectElement).value as PageSizePreset);
}
function updateOrientation(event: Event): void {
  updateSetting('orientation', (event.target as HTMLSelectElement).value as PageOrientation);
}
function updateBaseFont(event: Event): void {
  updateSetting('baseFontFamily', (event.target as HTMLInputElement).value);
}
function updateCodeFont(event: Event): void {
  updateSetting('codeFontFamily', (event.target as HTMLInputElement).value);
}
function updateCodeTheme(event: Event): void {
  updateSetting('codeTheme', (event.target as HTMLSelectElement).value as CodeTheme);
}
function updateMargin(key: 'marginTopMm' | 'marginRightMm' | 'marginBottomMm' | 'marginLeftMm', event: Event): void {
  const value = numberFromEvent(event);
  if (marginSync.value) {
    emit('update:modelValue', { ...props.modelValue, marginTopMm: value, marginRightMm: value, marginBottomMm: value, marginLeftMm: value });
    return;
  }
  updateSetting(key, value);
}
</script>

<style scoped>
.pdf-settings-panel { width: 276px; min-width: 276px; min-height: 0; display: flex; flex-direction: column; background: rgba(17, 24, 36, .8); border: 1px solid rgba(255,255,255,.08); border-radius: 14px; overflow: hidden; }
.pdf-settings-heading { min-height: 58px; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 12px 14px; border-bottom: 1px solid rgba(255,255,255,.07); }
.markdown-panel-kicker { display: block; margin-bottom: 3px; color: #c084fc; font-size: 9px; font-weight: 700; letter-spacing: .16em; }
h2, h3 { margin: 0; color: #f3f4f6; font-weight: 650; } h2 { font-size: 13px; } h3 { margin-bottom: 11px; font-size: 11px; }
.pdf-settings-reset { padding: 4px 7px; color: #94a3b8; background: transparent; border: 1px solid rgba(255,255,255,.12); border-radius: 6px; font-size: 10px; cursor: pointer; }
.pdf-settings-reset:hover { color: #e2e8f0; border-color: rgba(255,255,255,.24); }
.pdf-settings-scroll { min-height: 0; flex: 1; overflow-y: auto; padding: 3px 14px 18px; }
.pdf-setting-section { padding: 15px 0; border-bottom: 1px solid rgba(255,255,255,.07); } .pdf-setting-section:last-child { border-bottom: 0; }
.pdf-setting-section-title-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; } .pdf-setting-section-title-row h3 { margin-bottom: 11px; }
.pdf-setting-grid, .pdf-margin-grid { display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 9px; }
.pdf-setting-stack { display: flex; flex-direction: column; gap: 9px; margin-top: 10px; }
.pdf-setting-field { min-width: 0; display: flex; flex-direction: column; gap: 5px; color: #94a3b8; font-size: 10px; } .pdf-setting-field-wide { grid-column: 1 / -1; }
.pdf-setting-field input[type='number'], .pdf-setting-field input[type='text'], .pdf-setting-field select { width: 100%; min-width: 0; box-sizing: border-box; padding: 6px 7px; color: #e2e8f0; background: rgba(2,6,23,.42); border: 1px solid rgba(255,255,255,.1); border-radius: 6px; outline: none; font-size: 11px; }
.pdf-setting-field input:focus, .pdf-setting-field select:focus { border-color: rgba(96,165,250,.75); box-shadow: 0 0 0 2px rgba(59,130,246,.14); }
.pdf-checkbox { display: flex; align-items: center; gap: 7px; color: #cbd5e1; font-size: 10px; cursor: pointer; } .pdf-checkbox input { margin: 0; accent-color: #3b82f6; } .pdf-checkbox-small { color: #94a3b8; font-size: 9px; }
.pdf-color-input { display: flex; align-items: center; gap: 5px; } .pdf-color-input input[type='color'] { width: 28px; height: 25px; padding: 2px; background: rgba(2,6,23,.42); border: 1px solid rgba(255,255,255,.12); border-radius: 5px; cursor: pointer; } .pdf-color-input code { color: #94a3b8; font-family: Consolas, monospace; font-size: 9px; }
.pdf-setting-hint { margin: 10px 0 0; color: rgba(148,163,184,.62); font-size: 9px; line-height: 1.5; } .pdf-setting-scale-field { margin-top: 11px; } .pdf-setting-scale-field input[type='range'] { width: 100%; accent-color: #60a5fa; }
</style>

// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import { renderMarkdown } from './markdownRenderer';

describe('markdownRenderer', () => {
  it('renders common Markdown and removes unsafe HTML and links', () => {
    const fence = String.fromCharCode(96).repeat(3);
    const markdown = [
      '# 标题',
      '',
      '正文 **加粗**。',
      '',
      '| 名称 | 值 |',
      '| --- | --- |',
      '| 示例 | 1 |',
      '',
      '![本地图片](images/example.png)',
      '',
      '[危险链接](javascript:alert(1))',
      '',
      "<script>alert('unsafe')</script>",
      '',
      `${fence}js`,
      'const value = 1;',
      fence,
    ].join('\n');
    const result = renderMarkdown(markdown);

    expect(result.html).toContain('<h1>标题</h1>');
    expect(result.html).toContain('<table>');
    expect(result.html).toContain('language-js');
    expect(result.html).toContain('images/example.png');
    expect(result.localImageSources).toEqual(['images/example.png']);
    expect(result.html).not.toContain('<script');
    expect(result.html).not.toContain('javascript:');
  });
});

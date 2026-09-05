import { describe, expect, it } from 'vitest';
import { extractTemplateTokens, renderTemplatePreview } from './templateVars';

describe('extractTemplateTokens', () => {
  it('extracts every {{var}} token in order of first appearance, de-duplicated', () => {
    expect(extractTemplateTokens('EC vượt ngưỡng đêm: {{ec}} mS/cm lúc {{time}}, ec lại là {{ec}}')).toEqual([
      'ec',
      'time',
    ]);
  });

  it('returns an empty array when there are no tokens', () => {
    expect(extractTemplateTokens('không có biến nào ở đây')).toEqual([]);
  });

  it('tolerates extra whitespace inside the braces', () => {
    expect(extractTemplateTokens('giá trị {{  ph_target_now }}')).toEqual(['ph_target_now']);
  });
});

describe('renderTemplatePreview', () => {
  it('substitutes known tokens from the sample map', () => {
    expect(renderTemplatePreview('EC: {{ec}} lúc {{time}}', { ec: '1.8', time: '22:05' })).toBe(
      'EC: 1.8 lúc 22:05',
    );
  });

  it('leaves unknown tokens untouched (visible as {{token}} in the preview)', () => {
    expect(renderTemplatePreview('Giá trị: {{unknown_var}}', {})).toBe('Giá trị: {{unknown_var}}');
  });
});

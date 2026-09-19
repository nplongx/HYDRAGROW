// src/lib/design-lint/contrast.test.ts
import { describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { contrastRatio } from './contrast';

describe('WCAG AA contrast — token pairs actually used as body text', () => {
  const AA_NORMAL_TEXT = 4.5;

  it('faint text on page-bg meets AA (4.5:1)', () => {
    expect(contrastRatio('#556d5e', '#dcf0dc')).toBeGreaterThanOrEqual(AA_NORMAL_TEXT);
  });

  it('faint text on surface meets AA (4.5:1)', () => {
    expect(contrastRatio('#556d5e', '#ffffff')).toBeGreaterThanOrEqual(AA_NORMAL_TEXT);
  });

  it('primary text on page-bg meets AA (already passing, regression guard)', () => {
    expect(contrastRatio('#14532d', '#dcf0dc')).toBeGreaterThanOrEqual(AA_NORMAL_TEXT);
  });

  it('the live --color-faint token in design-system/tokens.css meets AA on current backgrounds', () => {
    const css = fs.readFileSync(
      path.resolve(__dirname, '../../design-system/tokens.css'),
      'utf-8',
    );
    const match = css.match(/--color-faint:\s*(#[0-9a-fA-F]{6})/);
    expect(match).not.toBeNull();
    const faintHex = match![1];
    expect(contrastRatio(faintHex, '#f7f7f7')).toBeGreaterThanOrEqual(AA_NORMAL_TEXT);
    expect(contrastRatio(faintHex, '#ffffff')).toBeGreaterThanOrEqual(AA_NORMAL_TEXT);
  });
});

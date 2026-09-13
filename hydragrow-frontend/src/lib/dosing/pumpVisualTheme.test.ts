// hydragrow-frontend/src/lib/dosing/pumpVisualTheme.test.ts
import { describe, expect, it } from 'vitest';
import { PUMP_VISUAL_THEME, pumpThemeFor, type PumpThemeKey } from './pumpVisualTheme';

describe('pumpVisualTheme', () => {
  it('có đúng 4 theme: nutrient, phUp, phDown, aqua', () => {
    expect(Object.keys(PUMP_VISUAL_THEME).sort()).toEqual(
      ['aqua', 'nutrient', 'phDown', 'phUp'].sort(),
    );
  });

  it('phUp và phDown phải khác hue nhau (không trùng màu 2 bơm pH)', () => {
    expect(PUMP_VISUAL_THEME.phUp.hue).not.toBe(PUMP_VISUAL_THEME.phDown.hue);
  });

  it.each([
    ['PUMP_A', 'nutrient'],
    ['PUMP_B', 'nutrient'],
    ['PH_UP', 'phUp'],
    ['PH_DOWN', 'phDown'],
    ['WATER_PUMP_IN', 'aqua'],
    ['WATER_PUMP_OUT', 'aqua'],
    ['OSAKA', 'aqua'],
    ['MIST', 'aqua'],
    ['MIX', 'aqua'],
  ] as const)('pumpThemeFor(%s) → %s', (pumpId, expected: PumpThemeKey) => {
    expect(pumpThemeFor(pumpId)).toBe(expected);
  });

  it('pumpId không xác định → fallback aqua (không throw)', () => {
    expect(pumpThemeFor('UNKNOWN_PUMP')).toBe('aqua');
  });
});

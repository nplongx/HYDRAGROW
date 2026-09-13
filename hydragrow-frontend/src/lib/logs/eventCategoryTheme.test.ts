// hydragrow-frontend/src/lib/logs/eventCategoryTheme.test.ts
import { describe, expect, it } from 'vitest';
import { EVENT_CATEGORY_THEME, type EventCategoryThemeKey } from './eventCategoryTheme';

describe('eventCategoryTheme', () => {
  it('định nghĩa đủ 6 category keys', () => {
    const keys: EventCategoryThemeKey[] = ['ecDosing', 'phDosing', 'water', 'warning', 'automation', 'device'];
    expect(Object.keys(EVENT_CATEGORY_THEME).sort()).toEqual(keys.sort());
  });

  it('mỗi theme có icon và badge không rỗng', () => {
    Object.values(EVENT_CATEGORY_THEME).forEach((theme) => {
      expect(theme.icon).toBeTruthy();
      expect(theme.badge).toBeTruthy();
    });
  });
});

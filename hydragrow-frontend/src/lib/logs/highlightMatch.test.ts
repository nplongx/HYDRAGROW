// hydragrow-frontend/src/lib/logs/highlightMatch.test.ts
import { describe, expect, it } from 'vitest';
import { splitByMatch } from './highlightMatch';

describe('splitByMatch', () => {
  it('không có query → trả về 1 đoạn, matched=false', () => {
    expect(splitByMatch('Châm dinh dưỡng A', '')).toEqual([
      { text: 'Châm dinh dưỡng A', matched: false },
    ]);
  });

  it('query khớp giữa chuỗi → tách 3 đoạn, không phân biệt hoa/thường', () => {
    expect(splitByMatch('Châm dinh dưỡng A', 'DINH')).toEqual([
      { text: 'Châm ', matched: false },
      { text: 'dinh', matched: true },
      { text: ' dưỡng A', matched: false },
    ]);
  });

  it('query không khớp → trả về nguyên chuỗi, matched=false', () => {
    expect(splitByMatch('Châm dinh dưỡng A', 'xyz')).toEqual([
      { text: 'Châm dinh dưỡng A', matched: false },
    ]);
  });
});

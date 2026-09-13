import { describe, expect, it } from 'vitest';
import { pushHealthSample, MAX_HEALTH_SAMPLES } from './useHealthHistory';

describe('pushHealthSample', () => {
  it('thêm mẫu mới vào cuối mảng', () => {
    const result = pushHealthSample([1, 2, 3], 4);
    expect(result).toEqual([1, 2, 3, 4]);
  });

  it('bỏ giá trị null/undefined, không thêm vào buffer', () => {
    expect(pushHealthSample([1, 2], null)).toEqual([1, 2]);
    expect(pushHealthSample([1, 2], undefined)).toEqual([1, 2]);
  });

  it(`giới hạn tối đa ${MAX_HEALTH_SAMPLES} mẫu — mẫu cũ nhất bị đẩy ra`, () => {
    const full = Array.from({ length: MAX_HEALTH_SAMPLES }, (_, i) => i);
    const result = pushHealthSample(full, 999);
    expect(result).toHaveLength(MAX_HEALTH_SAMPLES);
    expect(result[result.length - 1]).toBe(999);
    expect(result[0]).toBe(1);
  });
});

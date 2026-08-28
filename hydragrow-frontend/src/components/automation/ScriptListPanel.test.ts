import { describe, expect, it } from 'vitest';
import { canLoadIntoBuilder } from './ScriptListPanel';

describe('canLoadIntoBuilder', () => {
  it('is true when ir_json is present', () => {
    expect(canLoadIntoBuilder({ ir_json: { kind: 'alert' } as never })).toBe(true);
  });
  it('is false for a hand-written script with no ir_json', () => {
    expect(canLoadIntoBuilder({ ir_json: null })).toBe(false);
  });
});

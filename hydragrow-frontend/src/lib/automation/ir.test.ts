import { describe, expect, it } from 'vitest';
import { AutomationIrSchema } from './ir';

describe('AutomationIrSchema', () => {
  it('accepts a minimal valid alert IR', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
      actions: [{ type: 'alert', level: 'warning', message: 'pH cao' }],
      nodes: [],
      edges: [],
    };
    expect(() => AutomationIrSchema.parse(ir)).not.toThrow();
  });

  it('rejects an unknown operator', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '~=', value: 7.5 }],
      actions: [{ type: 'alert', level: 'warning', message: 'x' }],
      nodes: [],
      edges: [],
    };
    expect(() => AutomationIrSchema.parse(ir)).toThrow();
  });

  it('rejects recipe_override IR with an alert action', () => {
    const ir = {
      kind: 'recipe_override',
      trigger: { type: 'fsm' },
      conditions: [{ sensor: 'elapsed_sec', operator: '>=', value: 86400 }],
      actions: [{ type: 'alert', level: 'info', message: 'wrong action for this kind' }],
      nodes: [],
      edges: [],
    };
    expect(() => AutomationIrSchema.parse(ir)).toThrow();
  });
});

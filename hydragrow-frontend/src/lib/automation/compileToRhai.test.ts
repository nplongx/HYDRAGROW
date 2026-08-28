import { describe, expect, it } from 'vitest';
import { compileToRhai } from './compileToRhai';
import type { AutomationIr } from './ir';

describe('compileToRhai', () => {
  it('compiles an alert IR with AND-joined conditions', () => {
    const ir: AutomationIr = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [
        { sensor: 'ph', operator: '>', value: 7.5 },
        { sensor: 'ec', operator: '<', value: 1.2 },
      ],
      actions: [{ type: 'alert', level: 'warning', message: 'Water chemistry abnormal' }],
      nodes: [],
      edges: [],
    };
    const rhai = compileToRhai(ir);
    expect(rhai).toContain('fn main(input)');
    expect(rhai).toContain('input.ph > 7.5');
    expect(rhai).toContain('input.ec < 1.2');
    expect(rhai).toContain('"level": "warning"');
    expect(rhai).toContain('"message": "Water chemistry abnormal"');
  });

  it('compiles a recipe_override IR to a target_stage_index map', () => {
    const ir: AutomationIr = {
      kind: 'recipe_override',
      trigger: { type: 'fsm' },
      conditions: [{ sensor: 'elapsed_sec', operator: '>=', value: 86400 }],
      actions: [{ type: 'advance_stage', targetStageOffset: 1, reason: 'Đủ 24h' }],
      nodes: [],
      edges: [],
    };
    const rhai = compileToRhai(ir);
    expect(rhai).toContain('input.elapsed_sec >= 86400');
    expect(rhai).toContain('"target_stage_index": input.stage_index + 1');
    expect(rhai).toContain('"reason": "Đủ 24h"');
  });

  it('escapes double quotes in user-supplied strings', () => {
    const ir: AutomationIr = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
      actions: [{ type: 'alert', level: 'info', message: 'pH is "high"' }],
      nodes: [],
      edges: [],
    };
    const rhai = compileToRhai(ir);
    expect(rhai).toContain('pH is \\"high\\"');
  });
});

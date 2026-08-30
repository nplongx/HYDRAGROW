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

  describe('end_season compilation', () => {
    it('compiles an end_season action with an explicit action key', () => {
      const source = compileToRhai({
        kind: 'recipe_override',
        trigger: { type: 'fsm' },
        conditions: [{ sensor: 'stage_index', operator: '==', value: 3 }],
        actions: [{ type: 'end_season', reason: 'Hoàn thành mùa vụ' }],
        nodes: [],
        edges: [],
      });
      expect(source).toContain('"action": "end_season"');
      expect(source).toContain('"reason": "Hoàn thành mùa vụ"');
    });

    it('still compiles advance_stage with an explicit action key (forward-compat, không đổi hành vi backend)', () => {
      const source = compileToRhai({
        kind: 'recipe_override',
        trigger: { type: 'fsm' },
        conditions: [],
        actions: [{ type: 'advance_stage', targetStageOffset: 1, reason: 'x' }],
        nodes: [],
        edges: [],
      });
      expect(source).toContain('"action": "advance_stage"');
      expect(source).toContain('"target_stage_index"');
    });
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

describe('action_command compilation', () => {
  it('compiles a dose action with snake_case keys matching eval_action_command', () => {
    const source = compileToRhai({
      kind: 'action_command',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
      actions: [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }],
      nodes: [],
      edges: [],
    });
    expect(source).toContain('"action": "dose"');
    expect(source).toContain('"pump": "PH_DOWN"');
    expect(source).toContain('"dose_ml": 3');
    expect(source).toContain('"pwm": 80');
  });

  it('compiles a water_off action without a duration_sec key', () => {
    const source = compileToRhai({
      kind: 'action_command',
      trigger: { type: 'sensor' },
      conditions: [],
      actions: [{ type: 'water_off', pump: 'WATER_PUMP_IN' }],
      nodes: [],
      edges: [],
    });
    expect(source).toContain('"action": "water_off"');
    expect(source).not.toContain('duration_sec');
  });

  it('compiles emergency_stop with no other fields', () => {
    const source = compileToRhai({
      kind: 'action_command',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 9.0 }],
      actions: [{ type: 'emergency_stop' }],
      nodes: [],
      edges: [],
    });
    expect(source).toContain('"action": "emergency_stop"');
  });
});

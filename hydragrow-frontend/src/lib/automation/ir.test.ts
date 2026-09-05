import { describe, expect, it } from "vitest";
import { AutomationIrSchema, AutomationNodeSchema, ConditionSchema } from "./ir";

describe("ConditionSchema range mode and windowSec", () => {
  it('condition instant (mặc định) không cần field mode/windowSec', () => {
    expect(ConditionSchema.safeParse({ sensor: 'ph', operator: '>', value: 7.5 }).success).toBe(true);
  });

  it('condition time-window mean yêu cầu windowSec dương', () => {
    const ok = ConditionSchema.safeParse({ sensor: 'ph', operator: '>', value: 6.5, mode: 'mean', windowSec: 900 });
    expect(ok.success).toBe(true);
  });

  it('condition time-window thiếu windowSec bị từ chối', () => {
    const bad = ConditionSchema.safeParse({ sensor: 'ph', operator: '>', value: 6.5, mode: 'mean' });
    expect(bad.success).toBe(false);
  });
});

describe("AutomationIrSchema", () => {
  it('parses a legacy flat Condition[] (no groups) exactly as before', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [
        { sensor: 'ph', operator: '>', value: 7.5 },
        { sensor: 'ec', operator: '<', value: 1.2 },
      ],
      actions: [{ type: 'alert', level: 'warning', message: 'x' }],
      nodes: [],
      edges: [],
    };
    const result = AutomationIrSchema.safeParse(ir);
    expect(result.success).toBe(true);
    expect(result.data?.conditions).toEqual([
      { sensor: 'ph', operator: '>', value: 7.5, mode: 'instant' },
      { sensor: 'ec', operator: '<', value: 1.2, mode: 'instant' },
    ]);
  });

  it('accepts a nested AND/OR condition group matching the Figma example', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [
        {
          op: 'or',
          children: [
            { sensor: 'ph', operator: '<', value: 5.5 },
            { sensor: 'ph', operator: '>', value: 7.5 },
          ],
        },
        { sensor: 'ec', operator: '>', value: 3.0 },
      ],
      actions: [{ type: 'alert', level: 'warning', message: 'x' }],
      nodes: [],
      edges: [],
    };
    const result = AutomationIrSchema.safeParse(ir);
    expect(result.success).toBe(true);
  });

  it('rejects a group with an empty children array', () => {
    const result = AutomationIrSchema.safeParse({
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ op: 'and', children: [] }],
      actions: [{ type: 'alert', level: 'warning', message: 'x' }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(false);
  });

  it('rejects an unknown op value', () => {
    const result = AutomationIrSchema.safeParse({
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ op: 'xor', children: [{ sensor: 'ph', operator: '>', value: 7 }] }],
      actions: [{ type: 'alert', level: 'warning', message: 'x' }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(false);
  });

  it("accepts a minimal valid alert IR", () => {
    const ir = {
      kind: "alert",
      trigger: { type: "sensor" },
      conditions: [{ sensor: "ph", operator: ">", value: 7.5 }],
      actions: [{ type: "alert", level: "warning", message: "pH cao" }],
      nodes: [],
      edges: [],
    };
    expect(() => AutomationIrSchema.parse(ir)).not.toThrow();
  });

  describe('end_season IR', () => {
    it('accepts an end_season action under kind=recipe_override', () => {
      const result = AutomationIrSchema.safeParse({
        kind: 'recipe_override',
        trigger: { type: 'fsm' },
        conditions: [{ sensor: 'stage_index', operator: '==', value: 3 }],
        actions: [{ type: 'end_season', reason: 'Hoàn thành mùa vụ' }],
        nodes: [],
        edges: [],
      });
      expect(result.success).toBe(true);
    });

    it('rejects end_season action under kind=alert', () => {
      const result = AutomationIrSchema.safeParse({
        kind: 'alert',
        trigger: { type: 'sensor' },
        conditions: [],
        actions: [{ type: 'end_season', reason: 'x' }],
        nodes: [],
        edges: [],
      });
      expect(result.success).toBe(false);
    });
  });

  it('rejects an unknown operator', () => {
    const ir = {
      kind: "alert",
      trigger: { type: "sensor" },
      conditions: [{ sensor: "ph", operator: "~=", value: 7.5 }],
      actions: [{ type: "alert", level: "warning", message: "x" }],
      nodes: [],
      edges: [],
    };
    expect(() => AutomationIrSchema.parse(ir)).toThrow();
  });

  it("rejects recipe_override IR with an alert action", () => {
    const ir = {
      kind: "recipe_override",
      trigger: { type: "fsm" },
      conditions: [{ sensor: "elapsed_sec", operator: ">=", value: 86400 }],
      actions: [
        { type: "alert", level: "info", message: "wrong action for this kind" },
      ],
      nodes: [],
      edges: [],
    };
    expect(() => AutomationIrSchema.parse(ir)).toThrow();
  });
});

describe("action_command IR", () => {
  it("accepts a valid dose action", () => {
    const result = AutomationIrSchema.safeParse({
      kind: "action_command",
      trigger: { type: "sensor" },
      conditions: [{ sensor: "ph", operator: ">", value: 7.5 }],
      actions: [{ type: "dose", pump: "PH_DOWN", doseMl: 3, pwm: 80 }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("accepts a valid water_on action", () => {
    const result = AutomationIrSchema.safeParse({
      kind: "action_command",
      trigger: { type: "sensor" },
      conditions: [{ sensor: "water_level", operator: "<", value: 20 }],
      actions: [{ type: "water_on", pump: "WATER_PUMP_IN", durationSec: 30 }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("accepts a valid emergency_stop action with no conditions", () => {
    const result = AutomationIrSchema.safeParse({
      kind: "action_command",
      trigger: { type: "sensor" },
      conditions: [{ sensor: "ph", operator: ">", value: 9.0 }],
      actions: [{ type: "emergency_stop" }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("rejects an alert action under kind=action_command", () => {
    const result = AutomationIrSchema.safeParse({
      kind: "action_command",
      trigger: { type: "sensor" },
      conditions: [],
      actions: [{ type: "alert", level: "warning", message: "x" }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(false);
  });

  it("rejects pwm outside 1-100", () => {
    const result = AutomationIrSchema.safeParse({
      kind: "action_command",
      trigger: { type: "sensor" },
      conditions: [],
      actions: [{ type: "dose", pump: "PH_DOWN", doseMl: 3, pwm: 150 }],
      nodes: [],
      edges: [],
    });
    expect(result.success).toBe(false);
  });
});

it('validates AutomationNodeSchema including trigger type', () => {
  const node = {
    id: '1',
    type: 'sensor',
    position: { x: 0, y: 0 },
    data: { key: 'value' },
  };
  expect(AutomationNodeSchema.parse(node)).toEqual(node);

  const triggerNode = {
    id: 'trigger',
    type: 'trigger',
    position: { x: 250, y: 0 },
    data: {},
  };
  expect(AutomationNodeSchema.parse(triggerNode)).toEqual(triggerNode);
});

it('AutomationIrSchema: next_flow_ids defaults to [] when absent', () => {
  const ir = {
    kind: 'alert',
    trigger: { type: 'sensor' },
    conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
    actions: [{ type: 'alert', level: 'info', message: 'test' }],
    nodes: [],
    edges: [],
    // next_flow_ids vắng mặt
  };
  const result = AutomationIrSchema.safeParse(ir);
  expect(result.success).toBe(true);
  expect(result.data?.next_flow_ids).toEqual([]);
});

it('AutomationIrSchema: accepts valid cron trigger', () => {
  const ir = {
    kind: 'alert',
    trigger: { type: 'cron', cronExpression: '0 0 7 * * *' },
    conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
    actions: [{ type: 'alert', level: 'info', message: 'test' }],
    nodes: [],
    edges: [],
  };
  const result = AutomationIrSchema.safeParse(ir);
  expect(result.success).toBe(true);
  expect(result.data?.trigger).toEqual({
    type: 'cron',
    cronExpression: '0 0 7 * * *',
    timezone: 'Asia/Ho_Chi_Minh',
  });
});

it('AutomationIrSchema: rejects cron trigger with empty expression', () => {
  const ir = {
    kind: 'alert',
    trigger: { type: 'cron', cronExpression: '' },
    conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
    actions: [{ type: 'alert', level: 'info', message: 'test' }],
    nodes: [],
    edges: [],
  };
  const result = AutomationIrSchema.safeParse(ir);
  expect(result.success).toBe(false);
});

describe('config node type + Condition.valueVariable + chainConfig', () => {
  it('AutomationNodeSchema accepts type "config"', () => {
    const node = {
      id: 'cfg-1',
      type: 'config',
      position: { x: 0, y: 0 },
      data: { variant: 'read', configKey: 'ph_target', saveToVariable: 'ph_target_now' },
    };
    expect(AutomationNodeSchema.safeParse(node).success).toBe(true);
  });

  it('ConditionSchema accepts an optional valueVariable alongside the numeric value', () => {
    const result = ConditionSchema.safeParse({
      sensor: 'ph',
      operator: '>',
      value: 0,
      valueVariable: 'ph_target_now',
    });
    expect(result.success).toBe(true);
    expect(result.data?.valueVariable).toBe('ph_target_now');
  });

  it('ConditionSchema omits valueVariable when not provided (back-compat)', () => {
    const result = ConditionSchema.safeParse({ sensor: 'ph', operator: '>', value: 7.2 });
    expect(result.success).toBe(true);
    expect(result.data?.valueVariable).toBeUndefined();
  });

  it('rejects an empty-string valueVariable', () => {
    const result = ConditionSchema.safeParse({
      sensor: 'ph',
      operator: '>',
      value: 0,
      valueVariable: '',
    });
    expect(result.success).toBe(false);
  });

  it('AutomationIrSchema defaults chainConfig.passContextVariables to false when absent', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
      actions: [{ type: 'alert', level: 'info', message: 'test' }],
      nodes: [],
      edges: [],
    };
    const result = AutomationIrSchema.safeParse(ir);
    expect(result.success).toBe(true);
    expect(result.data?.chainConfig).toEqual({ passContextVariables: false, iterationLimit: 5 });
  });

  it('AutomationIrSchema accepts an explicit chainConfig.passContextVariables', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
      actions: [{ type: 'alert', level: 'info', message: 'test' }],
      nodes: [],
      edges: [],
      chainConfig: { passContextVariables: true },
    };
    const result = AutomationIrSchema.safeParse(ir);
    expect(result.success).toBe(true);
    expect(result.data?.chainConfig).toEqual({ passContextVariables: true, iterationLimit: 5 });
  });
});

describe('contextReads + configOverwrite + chainConfig.iterationLimit', () => {
  it('AutomationIrSchema defaults contextReads to [] and configOverwrite to undefined', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
      actions: [{ type: 'alert', level: 'info', message: 'test' }],
      nodes: [],
      edges: [],
    };
    const result = AutomationIrSchema.safeParse(ir);
    expect(result.success).toBe(true);
    expect(result.data?.contextReads).toEqual([]);
    expect(result.data?.configOverwrite).toBeUndefined();
    expect(result.data?.chainConfig).toEqual({ passContextVariables: false, iterationLimit: 5 });
  });

  it('accepts explicit contextReads and configOverwrite', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
      actions: [{ type: 'alert', level: 'info', message: 'test' }],
      nodes: [],
      edges: [],
      contextReads: [{ configKey: 'ph_target', saveToVariable: 'ph_target_now' }],
      configOverwrite: {
        configKey: 'ec_target',
        value: '1.8',
        readOriginalBeforeWrite: true,
        restoreMode: 'on_condition_false',
      },
    };
    const result = AutomationIrSchema.safeParse(ir);
    expect(result.success).toBe(true);
    expect(result.data?.contextReads).toEqual([{ configKey: 'ph_target', saveToVariable: 'ph_target_now' }]);
    expect(result.data?.configOverwrite?.configKey).toBe('ec_target');
  });

  it('rejects a contextRead with an empty saveToVariable', () => {
    const ir = {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
      actions: [{ type: 'alert', level: 'info', message: 'test' }],
      nodes: [],
      edges: [],
      contextReads: [{ configKey: 'ph_target', saveToVariable: '' }],
    };
    expect(AutomationIrSchema.safeParse(ir).success).toBe(false);
  });
});

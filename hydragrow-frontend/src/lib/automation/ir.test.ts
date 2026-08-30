import { describe, expect, it } from "vitest";
import { AutomationIrSchema } from "./ir";

describe("AutomationIrSchema", () => {
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

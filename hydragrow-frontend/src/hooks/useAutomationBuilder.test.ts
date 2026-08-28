import { describe, expect, it } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { fieldsForKind, useAutomationBuilder } from './useAutomationBuilder';
import { SENSOR_FIELDS, FSM_FIELDS } from '../lib/automation/ir';

describe('fieldsForKind', () => {
  it('returns SENSOR_FIELDS for alert', () => {
    expect(fieldsForKind('alert')).toBe(SENSOR_FIELDS);
  });
  it('returns FSM_FIELDS for recipe_override', () => {
    expect(fieldsForKind('recipe_override')).toBe(FSM_FIELDS);
  });
});

describe('useAutomationBuilder', () => {
  it('seeds a 3-node alert graph by default', () => {
    const { result } = renderHook(() => useAutomationBuilder());
    expect(result.current.kind).toBe('alert');
    expect(result.current.nodes).toHaveLength(3);
    expect(result.current.nodes.map((n) => n.type)).toEqual(['sensor', 'condition', 'action']);
  });

  it('setKind resets the graph so no stale action survives a kind switch', () => {
    const { result } = renderHook(() => useAutomationBuilder());
    act(() => {
      result.current.updateNodeData('3', {
        actions: [{ type: 'alert', level: 'warning', message: 'x' }],
      });
    });
    act(() => result.current.setKind('recipe_override'));
    const actionNode = result.current.nodes.find((n) => n.type === 'action');
    expect(actionNode?.data.actions).toEqual([]);
  });

  it('addNode appends a condition node with empty seed data', () => {
    const { result } = renderHook(() => useAutomationBuilder());
    act(() => result.current.addNode('condition'));
    expect(result.current.nodes).toHaveLength(4);
    expect(result.current.nodes[result.current.nodes.length - 1]).toMatchObject({ type: 'condition', data: { conditions: [] } });
  });

  it('loadFromIr with nodes switches to reactflow mode and restores the graph', () => {
    const { result } = renderHook(() => useAutomationBuilder());
    act(() =>
      result.current.loadFromIr({
        kind: 'alert',
        trigger: { type: 'sensor' },
        conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
        actions: [{ type: 'alert', level: 'info', message: 'x' }],
        nodes: [{ id: 'a', type: 'sensor', position: { x: 0, y: 0 }, data: {} }],
        edges: [],
      }),
    );
    expect(result.current.mode).toBe('reactflow');
    expect(result.current.nodes).toEqual([{ id: 'a', type: 'sensor', position: { x: 0, y: 0 }, data: {} }]);
  });

  it('loadFromIr with empty nodes switches to blockly mode and restores conditions/actions', () => {
    const { result } = renderHook(() => useAutomationBuilder());
    act(() =>
      result.current.loadFromIr({
        kind: 'recipe_override',
        trigger: { type: 'fsm' },
        conditions: [{ sensor: 'elapsed_sec', operator: '>=', value: 10 }],
        actions: [{ type: 'advance_stage', targetStageOffset: 1, reason: 'x' }],
        nodes: [],
        edges: [],
      }),
    );
    expect(result.current.mode).toBe('blockly');
    expect(result.current.blocklyResult.conditions).toEqual([{ sensor: 'elapsed_sec', operator: '>=', value: 10 }]);
  });
});

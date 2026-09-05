import { renderHook, act } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { useAutomationBuilder, summarizeActions } from './useAutomationBuilder';

describe('useAutomationBuilder', () => {
  it('updates trigger when requested instead of adding a new node', () => {
    const { result } = renderHook(() => useAutomationBuilder());

    const triggerNode = result.current.nodes.find((n) => n.id === 'trigger');
    expect(triggerNode).toBeDefined();

    act(() => {
      result.current.updateTrigger('cron');
    });

    const triggers = result.current.nodes.filter((n) => n.id === 'trigger');
    expect(triggers).toHaveLength(1);
    expect(triggers[0].data.kind).toBe('cron');

    act(() => {
      result.current.updateTrigger('webhook');
    });
    const updatedTriggers = result.current.nodes.filter((n) => n.id === 'trigger');
    expect(updatedTriggers[0].data.kind).toBe('webhook');
  });

  it('adds chain action node with correct variant type and summary', () => {
    const { result } = renderHook(() => useAutomationBuilder());

    act(() => {
      result.current.addNode('action', 'chain');
    });

    const chainNode = result.current.nodes.find((n) => n.data.type === 'chain');
    expect(chainNode).toBeDefined();
    expect(chainNode?.type).toBe('action');
    expect(chainNode?.data.summary).toBe('Kích hoạt Flow khác');
    expect(chainNode?.data.actions).toEqual([]);
  });

  it('summarizeActions summarizes different action types properly', () => {
    expect(summarizeActions([])).toBe('Chưa cấu hình');
    expect(summarizeActions([{ type: 'alert', level: 'warning', message: 'test alert' }])).toBe('alert (warning): test alert');
    expect(summarizeActions([{ type: 'dose', pump: 'PUMP_A', doseMl: 10, pwm: 100 }])).toBe('dose 10ml (PUMP_A)');
    expect(summarizeActions([{ type: 'water_on', pump: 'WATER_PUMP_IN', durationSec: 30 }])).toBe('water_on 30s (WATER_PUMP_IN)');
    expect(summarizeActions([{ type: 'water_off', pump: 'WATER_PUMP_IN' }])).toBe('water_off (WATER_PUMP_IN)');
    expect(summarizeActions([{ type: 'emergency_stop' }])).toBe('emergency_stop');
    expect(summarizeActions([{ type: 'advance_stage', targetStageOffset: 1, reason: 'next' }])).toBe('advance_stage +1: next');
    expect(summarizeActions([{ type: 'end_season', reason: 'done' }])).toBe('end_season: done');
  });

  it('adds a config_read node with the expected default data shape', () => {
    const { result } = renderHook(() => useAutomationBuilder());

    act(() => {
      result.current.addNode('config', 'read');
    });

    const configNode = result.current.nodes.find((n) => n.type === 'config');
    expect(configNode).toBeDefined();
    expect(configNode?.data).toMatchObject({
      variant: 'read',
      configKey: '',
      saveToVariable: '',
      summary: 'Chưa cấu hình',
    });
  });

  it('adds a config_overwrite node with the expected default data shape', () => {
    const { result } = renderHook(() => useAutomationBuilder());

    act(() => {
      result.current.addNode('config', 'overwrite');
    });

    const configNode = result.current.nodes.find((n) => n.type === 'config');
    expect(configNode).toBeDefined();
    expect(configNode?.data).toMatchObject({
      variant: 'overwrite',
      configKey: '',
      overrideValue: '',
      applyWhen: 'previous_condition_true',
      readOriginalBeforeWrite: false,
      restoreMode: 'on_condition_false',
      summary: 'Chưa cấu hình',
    });
  });
});


import { describe, expect, it } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFlowCanvas } from './useFlowCanvas';
import type { UserScript } from '../types/automation';

function makeScript(overrides: Partial<UserScript> = {}): UserScript {
  return {
    id: 's1',
    device_id: 'd1',
    kind: 'alert',
    name: 'Test',
    source: '',
    enabled: true,
    ir_json: null,
    created_at: '',
    updated_at: '',
    ...overrides,
  };
}

describe('useFlowCanvas', () => {
  it('lays out one node per script in a 4-column grid', () => {
    const scripts = [makeScript({ id: 'a' }), makeScript({ id: 'b' }), makeScript({ id: 'c' })];
    const { result } = renderHook(() => useFlowCanvas(scripts));
    expect(result.current.nodes).toHaveLength(3);
    expect(result.current.nodes[0].position).toEqual({ x: 0, y: 0 });
    expect(result.current.nodes[1].position).toEqual({ x: 220, y: 0 });
  });

  it('returns no nodes when scripts is undefined (still loading)', () => {
    const { result } = renderHook(() => useFlowCanvas(undefined));
    expect(result.current.nodes).toHaveLength(0);
  });

  it('openFlow selects the matching script and opens the drawer', () => {
    const scripts = [makeScript({ id: 'a', name: 'Flow A' })];
    const { result } = renderHook(() => useFlowCanvas(scripts));
    act(() => result.current.openFlow('a'));
    expect(result.current.selectedScript?.name).toBe('Flow A');
    expect(result.current.isDrawerOpen).toBe(true);
    expect(result.current.isCreatingNew).toBe(false);
  });

  it('openNewFlow opens the drawer in "creating" mode without a selected script', () => {
    const { result } = renderHook(() => useFlowCanvas([]));
    act(() => result.current.openNewFlow());
    expect(result.current.isCreatingNew).toBe(true);
    expect(result.current.selectedScript).toBeNull();
  });

  it('closeFlow resets everything', () => {
    const scripts = [makeScript({ id: 'a' })];
    const { result } = renderHook(() => useFlowCanvas(scripts));
    act(() => result.current.openFlow('a'));
    act(() => result.current.closeFlow());
    expect(result.current.isDrawerOpen).toBe(false);
    expect(result.current.selectedScript).toBeNull();
  });
});

import { describe, it, expect } from 'vitest';
import { wouldCreateCycle } from './flowCycle';
import type { UserScript } from '../../types/automation';

describe('flowCycle', () => {
  it('detects self-reference', () => {
    expect(wouldCreateCycle('flow-a', [], 'flow-a', [])).toBe(true);
  });

  it('detects A -> B -> A', () => {
    const scripts: UserScript[] = [
      { id: 'flow-b', ir_json: { next_flow_ids: ['flow-a'] } } as any,
    ];
    // A wants to target B
    expect(wouldCreateCycle('flow-a', [], 'flow-b', scripts)).toBe(true);
  });

  it('detects A -> B -> C -> A when A targets C directly', () => {
    const scripts: UserScript[] = [
      { id: 'flow-b', ir_json: { next_flow_ids: ['flow-c'] } } as any,
      { id: 'flow-c', ir_json: { next_flow_ids: ['flow-a'] } } as any,
    ];
    // A already points to B, now wants to also target C
    expect(wouldCreateCycle('flow-a', ['flow-b'], 'flow-c', scripts)).toBe(true);
  });

  it('allows independent target', () => {
    const scripts: UserScript[] = [
      { id: 'flow-b', ir_json: { next_flow_ids: ['flow-c'] } } as any,
      { id: 'flow-c', ir_json: { next_flow_ids: [] } } as any,
      { id: 'flow-d', ir_json: { next_flow_ids: [] } } as any,
    ];
    // A points to B. B points to C. C points nowhere. A targeting D is safe.
    expect(wouldCreateCycle('flow-a', ['flow-b'], 'flow-d', scripts)).toBe(false);
  });

  it('detects selecting an already-selected target toggles it off - well, that is UI logic but flowCycle shouldn\'t crash', () => {
    const scripts: UserScript[] = [
      { id: 'flow-b', ir_json: { next_flow_ids: [] } } as any,
    ];
    // If we're toggling OFF, we don't usually call wouldCreateCycle, but if we do...
    expect(wouldCreateCycle('flow-a', ['flow-b'], 'flow-b', scripts)).toBe(false); // actually this returns true because candidate === target or it finds it
  });
});

import { describe, expect, it } from 'vitest';
import { toEditorRoot, fromEditorRoot, countLeafConditions, summarizeConditionTree } from './conditionTree';
import type { ConditionOrGroup } from './ir';

describe('toEditorRoot', () => {
  it('wraps a flat leaf array in an implicit AND root', () => {
    const stored: ConditionOrGroup[] = [
      { sensor: 'ph', operator: '>', value: 7.5 },
      { sensor: 'ec', operator: '<', value: 1.2 },
    ];
    expect(toEditorRoot(stored)).toEqual({ op: 'and', children: stored });
  });

  it('wraps an empty array in an empty AND root', () => {
    expect(toEditorRoot([])).toEqual({ op: 'and', children: [] });
  });

  it('unwraps a single-item array whose item is already a group (root OR case)', () => {
    const group = {
      op: 'or' as const,
      children: [
        { sensor: 'ph', operator: '<' as const, value: 5.5 },
        { sensor: 'ph', operator: '>' as const, value: 7.5 },
      ],
    };
    expect(toEditorRoot([group])).toEqual(group);
  });

  it('wraps a mixed array (group + leaf) in an implicit AND root, NOT unwrapped', () => {
    const group = {
      op: 'or' as const,
      children: [
        { sensor: 'ph', operator: '<' as const, value: 5.5 },
        { sensor: 'ph', operator: '>' as const, value: 7.5 },
      ],
    };
    const leaf = { sensor: 'ec', operator: '>' as const, value: 3.0 };
    expect(toEditorRoot([group, leaf])).toEqual({ op: 'and', children: [group, leaf] });
  });
});

describe('fromEditorRoot', () => {
  it('unwraps an AND root back to a flat array (round-trips legacy data exactly)', () => {
    const stored: ConditionOrGroup[] = [
      { sensor: 'ph', operator: '>', value: 7.5 },
      { sensor: 'ec', operator: '<', value: 1.2 },
    ];
    expect(fromEditorRoot(toEditorRoot(stored))).toEqual(stored);
  });

  it('wraps an OR root in a single-item array', () => {
    const root = {
      op: 'or' as const,
      children: [
        { sensor: 'ph', operator: '<' as const, value: 5.5 },
        { sensor: 'ph', operator: '>' as const, value: 7.5 },
      ],
    };
    expect(fromEditorRoot(root)).toEqual([root]);
  });

  it('round-trips the Figma frame-03 example exactly', () => {
    const stored: ConditionOrGroup[] = [
      {
        op: 'or',
        children: [
          { sensor: 'ph', operator: '<', value: 5.5 },
          { sensor: 'ph', operator: '>', value: 7.5 },
        ],
      },
      { sensor: 'ec', operator: '>', value: 3.0 },
    ];
    expect(fromEditorRoot(toEditorRoot(stored))).toEqual(stored);
  });
});

describe('countLeafConditions', () => {
  it('counts 0 for an empty array', () => {
    expect(countLeafConditions([])).toBe(0);
  });

  it('counts flat leaves directly', () => {
    expect(countLeafConditions([
      { sensor: 'ph', operator: '>', value: 7.5 },
      { sensor: 'ec', operator: '<', value: 1.2 },
    ])).toBe(2);
  });

  it('counts leaves inside nested groups recursively', () => {
    expect(countLeafConditions([
      {
        op: 'or',
        children: [
          { sensor: 'ph', operator: '<', value: 5.5 },
          { sensor: 'ph', operator: '>', value: 7.5 },
        ],
      },
      { sensor: 'ec', operator: '>', value: 3.0 },
    ])).toBe(3);
  });
});

describe('summarizeConditionTree', () => {
  it('returns "Chưa cấu hình" for an empty array', () => {
    expect(summarizeConditionTree([])).toBe('Chưa cấu hình');
  });

  it('joins flat leaves with " và " exactly like the old summarizeConditions', () => {
    expect(summarizeConditionTree([
      { sensor: 'ph', operator: '>', value: 7.5 },
      { sensor: 'ec', operator: '<', value: 1.2 },
    ])).toBe('ph > 7.5 và ec < 1.2');
  });

  it('renders a nested OR group in parens joined by " hoặc "', () => {
    expect(summarizeConditionTree([
      {
        op: 'or',
        children: [
          { sensor: 'ph', operator: '<', value: 5.5 },
          { sensor: 'ph', operator: '>', value: 7.5 },
        ],
      },
      { sensor: 'ec', operator: '>', value: 3.0 },
    ])).toBe('(ph < 5.5 hoặc ph > 7.5) và ec > 3');
  });
});

import { describe, expect, it } from 'vitest';
import { getAvailableContextVariables, type GraphNode, type GraphEdge } from './contextVariables';

function triggerNode(data: Record<string, unknown>): GraphNode {
  return { id: 'trigger', type: 'trigger', data };
}

describe('getAvailableContextVariables', () => {
  it('exposes the fixed sensor fields for a sensor trigger', () => {
    const nodes: GraphNode[] = [
      triggerNode({ kind: 'sensor' }),
      { id: '2', type: 'condition', data: {} },
    ];
    const edges: GraphEdge[] = [{ source: 'trigger', target: '2' }];

    expect(getAvailableContextVariables(nodes, edges, '2')).toEqual([
      'ec', 'ph', 'temp', 'water_level',
    ]);
  });

  it('exposes the FSM fields for an fsm trigger', () => {
    const nodes: GraphNode[] = [
      triggerNode({ kind: 'fsm' }),
      { id: '2', type: 'condition', data: {} },
    ];
    const edges: GraphEdge[] = [{ source: 'trigger', target: '2' }];

    expect(getAvailableContextVariables(nodes, edges, '2')).toEqual([
      'ec', 'elapsed_sec', 'ph', 'stage_index',
    ]);
  });

  it('exposes webhook field-mapping targets instead of the default sensor names', () => {
    const nodes: GraphNode[] = [
      triggerNode({
        kind: 'webhook',
        trigger: {
          type: 'webhook',
          mode: 'flow',
          fieldMappings: [
            { bodyPath: 'data.ec', targetField: 'ec' },
            { bodyPath: 'data.ph', targetField: 'ph' },
          ],
        },
      }),
      { id: '2', type: 'condition', data: {} },
    ];
    const edges: GraphEdge[] = [{ source: 'trigger', target: '2' }];

    expect(getAvailableContextVariables(nodes, edges, '2')).toEqual(['ec', 'ph']);
  });

  it('adds an upstream config_read node saveToVariable to downstream nodes', () => {
    const nodes: GraphNode[] = [
      triggerNode({ kind: 'sensor' }),
      { id: 'cfg', type: 'config', data: { variant: 'read', saveToVariable: 'ph_target_now' } },
      { id: 'cond', type: 'condition', data: {} },
    ];
    const edges: GraphEdge[] = [
      { source: 'trigger', target: 'cfg' },
      { source: 'cfg', target: 'cond' },
    ];

    expect(getAvailableContextVariables(nodes, edges, 'cond')).toEqual([
      'ec', 'ph', 'ph_target_now', 'temp', 'water_level',
    ]);
  });

  it('does NOT leak a config_read variable to a sibling branch that is not upstream', () => {
    const nodes: GraphNode[] = [
      triggerNode({ kind: 'sensor' }),
      { id: 'cfg', type: 'config', data: { variant: 'read', saveToVariable: 'ph_target_now' } },
      { id: 'sibling', type: 'condition', data: {} },
    ];
    // 'sibling' branches directly off trigger, NOT through 'cfg'.
    const edges: GraphEdge[] = [
      { source: 'trigger', target: 'cfg' },
      { source: 'trigger', target: 'sibling' },
    ];

    expect(getAvailableContextVariables(nodes, edges, 'sibling')).toEqual([
      'ec', 'ph', 'temp', 'water_level',
    ]);
  });

  it('ignores a config_read node with an empty saveToVariable', () => {
    const nodes: GraphNode[] = [
      triggerNode({ kind: 'sensor' }),
      { id: 'cfg', type: 'config', data: { variant: 'read', saveToVariable: '' } },
      { id: 'cond', type: 'condition', data: {} },
    ];
    const edges: GraphEdge[] = [
      { source: 'trigger', target: 'cfg' },
      { source: 'cfg', target: 'cond' },
    ];

    expect(getAvailableContextVariables(nodes, edges, 'cond')).toEqual([
      'ec', 'ph', 'temp', 'water_level',
    ]);
  });

  it('is cycle-safe and terminates on a malformed graph with a back-edge', () => {
    const nodes: GraphNode[] = [
      triggerNode({ kind: 'sensor' }),
      { id: 'a', type: 'condition', data: {} },
      { id: 'b', type: 'condition', data: {} },
    ];
    const edges: GraphEdge[] = [
      { source: 'trigger', target: 'a' },
      { source: 'a', target: 'b' },
      { source: 'b', target: 'a' }, // back-edge
    ];

    expect(() => getAvailableContextVariables(nodes, edges, 'b')).not.toThrow();
  });

  it('returns an empty list for a target node with no trigger present (e.g. isolated unit test render)', () => {
    const nodes: GraphNode[] = [{ id: 'cond', type: 'condition', data: {} }];
    expect(getAvailableContextVariables(nodes, [], 'cond')).toEqual([]);
  });
});

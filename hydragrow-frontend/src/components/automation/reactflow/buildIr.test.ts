import { describe, expect, it } from 'vitest';
import type { Edge, Node } from '@xyflow/react';
import { buildIrFromGraph } from './buildIr';

describe('buildIrFromGraph', () => {
  it('assembles conditions/actions from condition and action nodes, ignoring layout-only nodes', () => {
    const nodes: Node[] = [
      { id: '1', type: 'sensor', position: { x: 0, y: 0 }, data: {} },
      {
        id: '2',
        type: 'condition',
        position: { x: 0, y: 100 },
        data: { conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }], summary: 'ph > 7.5' },
      },
      {
        id: '3',
        type: 'action',
        position: { x: 0, y: 200 },
        data: {
          actions: [{ type: 'alert', level: 'warning', message: 'pH cao' }],
          summary: 'alert: pH cao',
        },
      },
    ];
    const edges: Edge[] = [
      { id: 'e1', source: '1', target: '2' },
      { id: 'e2', source: '2', target: '3' },
    ];

    const ir = buildIrFromGraph({ kind: 'alert', nodes, edges });

    expect(ir.conditions).toEqual([{ sensor: 'ph', operator: '>', value: 7.5 }]);
    expect(ir.actions).toEqual([{ type: 'alert', level: 'warning', message: 'pH cao' }]);
    expect(ir.nodes).toHaveLength(3);
    expect(ir.edges).toHaveLength(2);
  });

  it('sets trigger type to sensor for action_command kind (not fsm)', () => {
    const ir = buildIrFromGraph({ kind: 'action_command', nodes: [], edges: [] });
    expect(ir.trigger.type).toBe('sensor');
  });

  it('passes nextFlowIds through to the IR', () => {
    const ir = buildIrFromGraph({
      kind: 'alert',
      nodes: [],
      edges: [],
      nextFlowIds: ['id-a', 'id-b'],
    });
    expect(ir.next_flow_ids).toEqual(['id-a', 'id-b']);
  });

  it('defaults nextFlowIds to empty array when omitted', () => {
    const ir = buildIrFromGraph({ kind: 'alert', nodes: [], edges: [] });
    expect(ir.next_flow_ids).toEqual([]);
  });
});

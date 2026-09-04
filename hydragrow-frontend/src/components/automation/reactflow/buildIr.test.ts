import { describe, expect, it } from 'vitest';
import type { Edge, Node } from '@xyflow/react';
import { buildIrFromGraph } from './buildIr';

describe('buildIrFromGraph', () => {
  it('flatMaps a condition-node whose data.conditions already contains a group, unchanged', () => {
    const nodes: Node[] = [
      {
        id: '2',
        type: 'condition',
        position: { x: 0, y: 100 },
        data: {
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
          summary: '(ph < 5.5 hoặc ph > 7.5) và ec > 3',
        },
      },
    ];
    const ir = buildIrFromGraph({ kind: 'alert', nodes, edges: [] });
    expect(ir.conditions).toEqual(nodes[0].data.conditions);
  });

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

  it('correctly builds IR from edited condition and action nodes', () => {
    const nodes: Node[] = [
      { id: 'trigger', type: 'trigger', position: { x: 0, y: 0 }, data: { kind: 'sensor' } },
      {
        id: '2',
        type: 'condition',
        position: { x: 0, y: 100 },
        data: {
          conditions: [
            { sensor: 'temperature', operator: '>=', value: 30 },
            { sensor: 'humidity', operator: '<=', value: 50 },
          ],
          summary: 'temperature >= 30 và humidity <= 50',
        },
      },
      {
        id: '3',
        type: 'action',
        position: { x: 0, y: 200 },
        data: {
          actions: [{ type: 'dose', pump: 'PUMP_A', doseMl: 10, pwm: 100 }],
          summary: 'dose 10ml (PUMP_A)',
        },
      },
    ];

    const ir = buildIrFromGraph({ kind: 'action_command', nodes, edges: [] });
    expect(ir.conditions).toEqual([
      { sensor: 'temperature', operator: '>=', value: 30 },
      { sensor: 'humidity', operator: '<=', value: 50 },
    ]);
    expect(ir.actions).toEqual([{ type: 'dose', pump: 'PUMP_A', doseMl: 10, pwm: 100 }]);
  });
});

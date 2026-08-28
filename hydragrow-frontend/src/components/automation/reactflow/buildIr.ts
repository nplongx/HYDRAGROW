import type { Edge, Node } from '@xyflow/react';
import type { Action, AutomationIr, Condition } from '../../../lib/automation/ir';

export function buildIrFromGraph(params: {
  kind: AutomationIr['kind'];
  nodes: Node[];
  edges: Edge[];
}): AutomationIr {
  const { kind, nodes, edges } = params;

  const conditions: Condition[] = nodes
    .filter((n) => n.type === 'condition')
    .flatMap((n) => (n.data as { conditions?: Condition[] }).conditions ?? []);

  const actions: Action[] = nodes
    .filter((n) => n.type === 'action')
    .flatMap((n) => (n.data as { actions?: Action[] }).actions ?? []);

  return {
    kind,
    trigger: { type: kind === 'alert' ? 'sensor' : 'fsm' },
    conditions,
    actions,
    nodes: nodes.map((n) => ({
      id: n.id,
      type: n.type as 'sensor' | 'condition' | 'delay' | 'action',
      position: n.position,
      data: n.data as Record<string, unknown>,
    })),
    edges: edges.map((e) => ({ id: e.id, source: e.source, target: e.target })),
  };
}

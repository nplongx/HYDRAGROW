import type { Edge, Node } from '@xyflow/react';
import type { Action, AutomationIr, ChainConfig, Condition } from '../../../lib/automation/ir';

export function buildIrFromGraph(params: {
  kind: AutomationIr['kind'];
  nodes: Node[];
  edges: Edge[];
  nextFlowIds?: string[];
  chainConfig?: ChainConfig;
}): AutomationIr {
  const { kind, nodes, edges, nextFlowIds, chainConfig } = params;

  const conditions: Condition[] = nodes
    .filter((n) => n.type === 'condition')
    .flatMap((n) => (n.data as { conditions?: Condition[] }).conditions ?? []);

  const actions: Action[] = nodes
    .filter((n) => n.type === 'action')
    .flatMap((n) => (n.data as { actions?: Action[] }).actions ?? []);

  return {
    kind,
    // recipe_override chạy trên FSM transition (fsm.rs); alert và
    // action_command đều chạy trên sensor MQTT data (sensors.rs) — xem
    // hydragrow-backend/src/models/script.rs::ScriptKind.
    trigger: { type: kind === 'recipe_override' ? 'fsm' : 'sensor' },
    conditions,
    actions,
    nodes: nodes.map((n) => ({
      id: n.id,
      type: n.type as 'sensor' | 'condition' | 'delay' | 'action' | 'config',
      position: n.position,
      data: n.data as Record<string, unknown>,
    })),
    edges: edges.map((e) => ({ id: e.id, source: e.source, target: e.target })),
    next_flow_ids: nextFlowIds ?? [],
    chainConfig: chainConfig ?? { passContextVariables: false },
  };
}

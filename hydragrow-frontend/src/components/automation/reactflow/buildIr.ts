import type { Edge, Node } from '@xyflow/react';
import type { Action, AutomationIr, ChainConfig, Condition } from '../../../lib/automation/ir';
import { collectConfigDirectives } from '../../../lib/automation/configDirectives';

export function buildIrFromGraph(params: {
  kind: AutomationIr['kind'];
  nodes: Node[];
  edges: Edge[];
  nextFlowIds?: string[];
  chainConfig?: Partial<ChainConfig>;
}): AutomationIr {
  const { kind, nodes, edges, nextFlowIds, chainConfig } = params;

  const conditions: Condition[] = nodes
    .filter((n) => n.type === 'condition')
    .flatMap((n) => (n.data as { conditions?: Condition[] }).conditions ?? []);

  const actions: Action[] = nodes
    .filter((n) => n.type === 'action')
    .flatMap((n) => (n.data as { actions?: Action[] }).actions ?? []);

  const { contextReads, configOverwrite } = collectConfigDirectives(
    nodes.map((n) => ({ id: n.id, type: n.type, data: n.data as Record<string, unknown> })),
  );

  // Trigger node lưu sẵn dạng ĐÚNG shape của AutomationIr['trigger'] trong
  // node.data.trigger — trước bản sửa này, field đó bị bỏ qua hoàn toàn.
  const triggerNode = nodes.find((n) => n.type === 'trigger');
  const savedTrigger = (triggerNode?.data as { trigger?: AutomationIr['trigger'] } | undefined)?.trigger;
  const trigger: AutomationIr['trigger'] =
    savedTrigger ?? { type: kind === 'recipe_override' ? 'fsm' : 'sensor' };

  return {
    kind,
    trigger,
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
    chainConfig: {
      passContextVariables: chainConfig?.passContextVariables ?? false,
      iterationLimit: chainConfig?.iterationLimit ?? 5,
    },
    contextReads,
    configOverwrite,
  };
}

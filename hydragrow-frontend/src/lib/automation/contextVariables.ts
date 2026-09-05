import { FSM_FIELDS, SENSOR_FIELDS, type WebhookTriggerConfig } from './ir';

/** Minimal shape this module needs from a React Flow node — deliberately
 * looser than `@xyflow/react`'s `Node` so callers can pass either canvas
 * nodes or the persisted `AutomationNode` IR shape without adapting them. */
export interface GraphNode {
  id: string;
  type?: string;
  data: Record<string, unknown>;
}

export interface GraphEdge {
  source: string;
  target: string;
}

/** Walks the canvas graph backwards from `targetNodeId` and returns the
 * sorted, de-duplicated list of variable names available in the execution
 * context at that point: the trigger's default fields (sensor/fsm) or its
 * webhook field-mapping targets, plus any `saveToVariable` declared by an
 * upstream `config` node with `variant: 'read'`.
 *
 * Pure and synchronous — no network/DOM access — safe to call on every
 * render. Cycle-safe: a malformed graph with a back-edge terminates instead
 * of looping forever. */
export function getAvailableContextVariables(
  nodes: GraphNode[],
  edges: GraphEdge[],
  targetNodeId: string,
): string[] {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const incoming = new Map<string, string[]>();
  for (const e of edges) {
    incoming.set(e.target, [...(incoming.get(e.target) ?? []), e.source]);
  }

  const visited = new Set<string>();
  const ancestors: GraphNode[] = [];
  const stack = [...(incoming.get(targetNodeId) ?? [])];
  while (stack.length > 0) {
    const id = stack.pop()!;
    if (visited.has(id)) continue;
    visited.add(id);
    const node = byId.get(id);
    if (node) {
      ancestors.push(node);
      stack.push(...(incoming.get(id) ?? []));
    }
  }

  const variables = new Set<string>();

  const trigger = ancestors.find((n) => n.id === 'trigger' || n.type === 'trigger');
  if (trigger) {
    const kind = trigger.data.kind as 'sensor' | 'fsm' | 'cron' | 'webhook' | undefined;
    if (kind === 'webhook') {
      const webhookConfig = trigger.data.trigger as WebhookTriggerConfig | undefined;
      for (const mapping of webhookConfig?.fieldMappings ?? []) {
        if (mapping.targetField) variables.add(mapping.targetField);
      }
    } else if (kind === 'fsm') {
      FSM_FIELDS.forEach((f) => variables.add(f));
    } else if (kind === 'sensor' || kind === undefined) {
      // Default trigger kind (no explicit `kind` set yet) behaves like sensor.
      SENSOR_FIELDS.forEach((f) => variables.add(f));
    }
    // 'cron' contributes no live variables — matches the design (a schedule
    // trigger carries no sensor/webhook payload into the context).
  }

  for (const node of ancestors) {
    if (node.type === 'config' && node.data.variant === 'read') {
      const saveToVariable = node.data.saveToVariable;
      if (typeof saveToVariable === 'string' && saveToVariable.length > 0) {
        variables.add(saveToVariable);
      }
    }
  }

  return Array.from(variables).sort();
}

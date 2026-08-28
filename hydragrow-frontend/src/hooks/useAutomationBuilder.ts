import { useCallback, useMemo, useState } from 'react';
import type { Edge, Node } from '@xyflow/react';
import { addEdge, useEdgesState, useNodesState, type Connection } from '@xyflow/react';
import { FSM_FIELDS, SENSOR_FIELDS, type Action, type AutomationIr, type Condition } from '../lib/automation/ir';

export type BuilderMode = 'reactflow' | 'blockly';

function seedNodes(): Node[] {
  return [
    { id: '1', type: 'sensor', position: { x: 250, y: 0 }, data: {} },
    { id: '2', type: 'condition', position: { x: 250, y: 120 }, data: { conditions: [], summary: 'Chưa cấu hình' } },
    { id: '3', type: 'action', position: { x: 250, y: 240 }, data: { actions: [], summary: 'Chưa cấu hình' } },
  ];
}
const SEED_EDGES: Edge[] = [
  { id: 'e1-2', source: '1', target: '2' },
  { id: 'e2-3', source: '2', target: '3' },
];

/** Sensor field list valid for the given automation kind (mirrors backend
 * ScriptSensorInput / ScriptFsmInput). */
export function fieldsForKind(kind: AutomationIr['kind']): readonly string[] {
  return kind === 'alert' ? SENSOR_FIELDS : FSM_FIELDS;
}

let nextNodeId = 100;

export function useAutomationBuilder() {
  const [kind, setKindState] = useState<AutomationIr['kind']>('alert');
  const [mode, setMode] = useState<BuilderMode>('reactflow');
  const [nodes, setNodes, onNodesChange] = useNodesState(seedNodes());
  const [edges, setEdges, onEdgesChange] = useEdgesState(SEED_EDGES);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [blocklyResult, setBlocklyResult] = useState<{ conditions: Condition[]; actions: Action[] }>({
    conditions: [],
    actions: [],
  });

  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(connection, eds)),
    [setEdges],
  );

  /** Switching kind resets the canvas so a stale action from the other kind
   * never survives to fail AutomationIrSchema's refine rule. */
  const setKind = useCallback(
    (next: AutomationIr['kind']) => {
      setKindState(next);
      setNodes(seedNodes());
      setEdges(SEED_EDGES);
      setSelectedNodeId(null);
      setBlocklyResult({ conditions: [], actions: [] });
    },
    [setNodes, setEdges],
  );

  const updateNodeData = useCallback(
    (nodeId: string, data: Record<string, unknown>) => {
      setNodes((nds) => nds.map((n) => (n.id === nodeId ? { ...n, data: { ...n.data, ...data } } : n)));
    },
    [setNodes],
  );

  const addNode = useCallback(
    (type: 'condition' | 'action') => {
      const id = String(nextNodeId++);
      const data = type === 'condition' ? { conditions: [], summary: 'Chưa cấu hình' } : { actions: [], summary: 'Chưa cấu hình' };
      setNodes((nds) => [...nds, { id, type, position: { x: 250 + nds.length * 40, y: 360 }, data }]);
    },
    [setNodes],
  );

  /** Restore a previously-saved IR into the builder. IRs saved from the
   * React Flow canvas have `nodes.length > 0`; IRs saved from Blockly don't
   * (Blockly has no canvas positions), so that's what selects the mode. */
  const loadFromIr = useCallback(
    (ir: AutomationIr) => {
      setKindState(ir.kind);
      setSelectedNodeId(null);
      if (ir.nodes.length > 0) {
        setNodes(ir.nodes);
        setEdges(ir.edges);
        setMode('reactflow');
      } else {
        setNodes(seedNodes());
        setEdges(SEED_EDGES);
        setBlocklyResult({ conditions: ir.conditions, actions: ir.actions });
        setMode('blockly');
      }
    },
    [setNodes, setEdges],
  );

  const selectedNode = useMemo(() => nodes.find((n) => n.id === selectedNodeId) ?? null, [nodes, selectedNodeId]);

  return {
    kind,
    setKind,
    mode,
    setMode,
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    selectedNodeId,
    setSelectedNodeId,
    selectedNode,
    updateNodeData,
    addNode,
    blocklyResult,
    setBlocklyResult,
    loadFromIr,
  };
}

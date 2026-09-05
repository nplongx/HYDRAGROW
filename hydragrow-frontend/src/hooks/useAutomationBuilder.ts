import { useCallback, useMemo, useState } from "react";
import type { Edge, Node } from "@xyflow/react";
import {
  addEdge,
  useEdgesState,
  useNodesState,
  type Connection,
} from "@xyflow/react";
import {
  FSM_FIELDS,
  SENSOR_FIELDS,
  type Action,
  type AutomationIr,
  type ConditionOrGroup,
} from "../lib/automation/ir";
import { summarizeConditionTree } from "../lib/automation/conditionTree";

function seedNodes(): Node[] {
  return [
    { id: "trigger", type: "trigger", position: { x: 80, y: 150 }, data: {} },
    {
      id: "2",
      type: "condition",
      position: { x: 330, y: 150 },
      data: { conditions: [], summary: "Chưa cấu hình" },
    },
    {
      id: "3",
      type: "action",
      position: { x: 580, y: 150 },
      data: { actions: [], summary: "Chưa cấu hình" },
    },
  ];
}
const SEED_EDGES: Edge[] = [
  { id: "etrigger-2", source: "trigger", target: "2" },
  { id: "e2-3", source: "2", target: "3" },
];

/** Summarizes an array of actions for node summaries and IR previews. */
export function summarizeActions(actions: Action[]): string {
  if (actions.length === 0) return "Chưa cấu hình";
  return actions
    .map((a) => {
      switch (a.type) {
        case "alert":
          return `alert (${a.level}): ${a.message}`;
        case "advance_stage":
          return `advance_stage ${a.targetStageOffset >= 0 ? "+" : ""}${a.targetStageOffset}: ${a.reason}`;
        case "end_season":
          return `end_season: ${a.reason}`;
        case "dose":
          return `dose ${a.doseMl}ml (${a.pump})`;
        case "water_on":
          return `water_on ${a.durationSec}s (${a.pump})`;
        case "water_off":
          return `water_off (${a.pump})`;
        case "emergency_stop":
          return "emergency_stop";
        case "config_override":
          return `ghi đè ${a.key} -> ${a.value}`;
        default:
          return "unknown_action";
      }
    })
    .join(", ");
}

/** Rebuild the seed graph but with a single condition-node and single
 * action-node pre-populated from a flat IR that predates the graph canvas
 * (every script saved via the old Blockly editor has `nodes: []`). Multiple
 * actions collapse into one action-node's `actions` array — this matches
 * `buildIrFromGraph`'s own flattening (it concatenates every action-node's
 * `actions` array), so re-saving without any edits round-trips exactly. */
function synthesizeGraphFromFlatIr(
  conditions: ConditionOrGroup[],
  actions: Action[],
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [
    { id: "trigger", type: "trigger", position: { x: 80, y: 150 }, data: {} },
    {
      id: "2",
      type: "condition",
      position: { x: 330, y: 150 },
      data: { conditions, summary: summarizeConditionTree(conditions) },
    },
    {
      id: "3",
      type: "action",
      position: { x: 580, y: 150 },
      data: { actions, summary: summarizeActions(actions) },
    },
  ];
  return { nodes, edges: SEED_EDGES };
}

/** Sensor field list valid for the given automation kind (mirrors backend
 * ScriptSensorInput / ScriptFsmInput). */
export function fieldsForKind(kind: AutomationIr["kind"]): readonly string[] {
  return kind === "recipe_override" ? FSM_FIELDS : SENSOR_FIELDS;
}

let nextNodeId = 100;

export function useAutomationBuilder() {
  const [kind, setKindState] = useState<AutomationIr["kind"]>("alert");
  const [nodes, setNodes, onNodesChange] = useNodesState(seedNodes());
  const [edges, setEdges, onEdgesChange] = useEdgesState(SEED_EDGES);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(connection, eds)),
    [setEdges],
  );

  const setKind = useCallback(
    (next: AutomationIr["kind"]) => {
      setKindState(next);
      setNodes(seedNodes());
      setEdges(SEED_EDGES);
      setSelectedNodeId(null);
    },
    [setNodes, setEdges],
  );

  const updateNodeData = useCallback(
    (nodeId: string, data: Record<string, unknown>) => {
      setNodes((nds) =>
        nds.map((n) =>
          n.id === nodeId ? { ...n, data: { ...n.data, ...data } } : n,
        ),
      );
    },
    [setNodes],
  );

  const updateTrigger = useCallback(
    (type: "sensor" | "fsm" | "cron" | "webhook") => {
      setNodes((nds) =>
        nds.map((n) =>
          n.id === "trigger" || n.type === "trigger"
            ? { ...n, data: { ...n.data, kind: type } }
            : n,
        ),
      );
    },
    [setNodes],
  );

  const addNode = useCallback(
    (
      type: "condition" | "condition_group" | "action" | "config",
      variant?: string,
    ) => {
      const id = String(nextNodeId++);
      const data =
        type === "action"
          ? {
              ...(variant ? { type: variant } : {}),
              actions: [],
              summary:
                variant === "chain" ? "Kích hoạt Flow khác" : "Chưa cấu hình",
            }
          : type === "condition_group"
            ? {
                conditions: [{ op: "and", children: [] }],
                summary: "Chưa cấu hình",
              }
            : type === "config"
              ? variant === "overwrite"
                ? {
                    variant: "overwrite",
                    configKey: "",
                    overrideValue: "",
                    applyWhen: "previous_condition_true",
                    readOriginalBeforeWrite: false,
                    restoreMode: "on_condition_false",
                    summary: "Chưa cấu hình",
                  }
                : {
                    variant: "read",
                    configKey: "",
                    saveToVariable: "",
                    summary: "Chưa cấu hình",
                  }
              : {
                  ...(variant ? { type: variant } : {}),
                  conditions: [],
                  summary: "Chưa cấu hình",
                };
      const nodeType = type === "condition_group" ? "condition" : type;
      setNodes((nds) => [
        ...nds,
        {
          id,
          type: nodeType,
          position: { x: 80 + nds.length * 240, y: 150 },
          data,
        },
      ]);
    },
    [setNodes],
  );

  /** Restore a previously-saved IR into the builder. Ensure trigger node always exists. */
  const loadFromIr = useCallback(
    (ir: AutomationIr) => {
      setKindState(ir.kind);
      setSelectedNodeId(null);
      if (ir.nodes.length > 0) {
        let loadedNodes = ir.nodes as Node[];
        if (
          !loadedNodes.some((n) => n.id === "trigger" || n.type === "trigger")
        ) {
          loadedNodes = [
            {
              id: "trigger",
              type: "trigger",
              position: { x: 250, y: 0 },
              data: {},
            },
            ...loadedNodes,
          ];
        }
        setNodes(loadedNodes);
        setEdges(ir.edges);
      } else {
        const { nodes: synthesized, edges: synthesizedEdges } =
          synthesizeGraphFromFlatIr(ir.conditions, ir.actions);
        setNodes(synthesized);
        setEdges(synthesizedEdges);
      }
    },
    [setNodes, setEdges],
  );

  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId],
  );

  return {
    kind,
    setKind,
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
    updateTrigger,
    loadFromIr,
  };
}

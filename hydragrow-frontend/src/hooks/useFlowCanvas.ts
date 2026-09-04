import { useState, useMemo } from "react";
import { Edge, Node } from "@xyflow/react";
import type { UserScript } from "../types/automation";
import { Bell, Activity, Zap } from "lucide-react";

export function useFlowCanvas(scripts: UserScript[]) {
  const [selectedScript, setSelectedScript] = useState<
    UserScript | "new" | null
  >(null);

  const { nodes, edges } = useMemo(() => {
    const layoutNodes: Node[] = [];
    const layoutEdges: Edge[] = [];

    // Simple grid layout for summary nodes
    scripts.forEach((script, idx) => {
      layoutNodes.push({
        id: script.id,
        type: "flowSummary",
        position: {
          x: 100 + (idx % 3) * 350,
          y: 100 + Math.floor(idx / 3) * 200,
        },
        data: { script },
      });

      // Construct directed animated edges based on next_flow_ids
      // (Using the same logic conceptually as wouldCreateCycle but here we just need to render existing edges)
      if (script.ir_json?.next_flow_ids) {
        script.ir_json.next_flow_ids.forEach((nextId) => {
          // Only create edges to existing scripts
          if (scripts.some((s) => s.id === nextId)) {
            layoutEdges.push({
              id: `${script.id}->${nextId}`,
              source: script.id,
              target: nextId,
              animated: true,
              style: { stroke: '#10b981', strokeWidth: 2, strokeDasharray: '5 5' },
            });
          }
        });
      }
    });

    return { nodes: layoutNodes, edges: layoutEdges };
  }, [scripts]);

  const openEditor = (script: UserScript | "new") => setSelectedScript(script);
  const closeEditor = () => setSelectedScript(null);

  const getTriggerIconAndColor = (script: UserScript) => {
    if (!script.ir_json || !script.ir_json.nodes)
      return {
        icon: Activity,
        color: "text-emerald-800/70",
        bg: "bg-emerald-100/60",
        label: "Không có trigger",
      };

    const triggerNode = script.ir_json.nodes.find((n) => n.id === "trigger");
    if (!triggerNode)
      return {
        icon: Activity,
        color: "text-emerald-800/70",
        bg: "bg-emerald-100/60",
        label: "Không có trigger",
      };

    // Default mapped from node kind
    if (script.kind === "alert")
      return {
        icon: Bell,
        color: "text-amber-500",
        bg: "bg-amber-100",
        label: "Cảnh báo",
      };

    return {
      icon: Zap,
      color: "text-sky-600",
      bg: "bg-sky-100",
      label: "Hành động",
    };
  };

  return {
    nodes,
    edges,
    selectedScript,
    openEditor,
    closeEditor,
    getTriggerIconAndColor,
  };
}

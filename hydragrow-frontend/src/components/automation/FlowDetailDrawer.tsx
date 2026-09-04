import { useEffect, useState } from "react";
import toast from "react-hot-toast";
import { ReactFlow, Background, Controls } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { AUTOMATION_NODE_TYPES } from "./reactflow/nodeTypes";
import { NodePalette } from "./reactflow/NodePalette";
import { NodeEditorPanel } from "./reactflow/NodeEditorPanel";
import { buildIrFromGraph } from "./reactflow/buildIr";
import { TestPanel } from "./reactflow/TestPanel";
import { fieldsForKind } from "../../hooks/useAutomationBuilder";
import { FlowEditorHeader } from "./FlowEditorHeader";
import { FlowEditorFooter } from "./FlowEditorFooter";
import { NextFlowSelector } from "./NextFlowSelector";
import { useAutomationBuilder } from "../../hooks/useAutomationBuilder";
import { AutomationIrSchema, type AutomationIr } from "../../lib/automation/ir";
import { compileToRhai } from "../../lib/automation/compileToRhai";
import type { UserScript } from "../../types/automation";
import {
  useAutomationScripts,
  useCreateAutomationScript,
  useDeleteAutomationScript,
  useUpdateAutomationScript,
  useValidateAutomationScript,
} from "../../hooks/useAutomationScripts";

export interface FlowDetailDrawerProps {
  deviceId: string;
  /** 'new' khi tạo Flow mới; một `UserScript` khi mở chi tiết Flow đã lưu. */
  script: UserScript | "new";
  onClose: () => void;
}

export function FlowDetailDrawer({
  deviceId,
  script,
  onClose,
}: FlowDetailDrawerProps) {
  const isNew = script === "new";
  const [name, setName] = useState(isNew ? "Flow mới" : script.name);
  const [enabled, setEnabled] = useState(isNew ? true : script.enabled);
  const [showTestPanel, setShowTestPanel] = useState(false);
  const [nextFlowIds, setNextFlowIds] = useState<string[]>(
    isNew ? [] : (script.ir_json?.next_flow_ids ?? []),
  );
  const builder = useAutomationBuilder();
  const { data: allScripts } = useAutomationScripts(deviceId);
  const otherScripts = (allScripts ?? []).filter(
    (s) => isNew || s.id !== script.id,
  );

  const toggleNextFlow = (id: string) => {
    setNextFlowIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  };

  useEffect(() => {
    if (!isNew && script.ir_json) {
      builder.loadFromIr(script.ir_json);
    } else {
      builder.setKind("alert");
    }
    // Seed once per Flow opened — not on every builder state change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isNew, !isNew && (script as UserScript).id]);

  const validateScript = useValidateAutomationScript(deviceId);
  const createScript = useCreateAutomationScript(deviceId);
  const updateScript = useUpdateAutomationScript(
    deviceId,
    isNew ? "" : script.id,
  );
  const deleteScript = useDeleteAutomationScript(deviceId);

  const handleSave = async () => {
    const ir: AutomationIr = buildIrFromGraph({
      kind: builder.kind,
      nodes: builder.nodes,
      edges: builder.edges,
      nextFlowIds,
    });
    const parsed = AutomationIrSchema.safeParse(ir);
    if (!parsed.success) {
      toast.error(`IR không hợp lệ: ${parsed.error.issues[0]?.message}`);
      return;
    }
    const source = compileToRhai(parsed.data);
    const validation = await validateScript.mutateAsync({
      id: isNew ? undefined : script.id,
      kind: parsed.data.kind,
      name,
      source,
      ir_json: parsed.data,
      next_flow_ids: nextFlowIds,
    });
    if (!validation.valid) {
      toast.error(`Script không hợp lệ: ${validation.error}`);
      return;
    }
    if (isNew) {
      await createScript.mutateAsync({
        kind: parsed.data.kind,
        name,
        source,
        enabled,
        ir_json: parsed.data,
      });
    } else {
      await updateScript.mutateAsync({
        kind: parsed.data.kind,
        name,
        source,
        enabled,
        ir_json: parsed.data,
      });
    }
    toast.success("Đã lưu Flow");
    onClose();
  };

  const handleDelete = () => {
    if (isNew) return;
    if (!confirm(`Xóa Flow "${script.name}"?`)) return;
    deleteScript.mutate(script.id, { onSuccess: onClose });
  };

  return (
    <div data-testid="flow-detail-drawer" className="flex h-full flex-col gap-2 p-4 overflow-y-auto">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-emerald-950">
          {isNew ? "Flow mới" : `Sửa: ${script.name}`}
        </h2>
        <button className="text-sm text-emerald-700/70" onClick={onClose}>
          Đóng
        </button>
      </div>

      <FlowEditorHeader
        name={name}
        kind={builder.kind}
        enabled={enabled}
        onChange={(updates) => {
          if (updates.name !== undefined) setName(updates.name);
          if (updates.kind !== undefined) builder.setKind(updates.kind);
          if (updates.enabled !== undefined) setEnabled(updates.enabled);
        }}
      />

      <NodePalette onAddNode={builder.addNode} onUpdateTrigger={builder.updateTrigger} />

      <div className="flex flex-1 flex-col overflow-hidden rounded-2xl border border-emerald-100 lg:flex-row">
        <div className="min-h-[16rem] flex-1">
          <ReactFlow
            nodes={builder.nodes}
            edges={builder.edges}
            nodeTypes={AUTOMATION_NODE_TYPES}
            onNodesChange={builder.onNodesChange}
            onEdgesChange={builder.onEdgesChange}
            onConnect={builder.onConnect}
            onNodeClick={(_, node) => builder.setSelectedNodeId(node.id)}
            fitView
            className="h-full w-full"
          >
            <Background />
            <Controls />
          </ReactFlow>

          {showTestPanel && (
            <div className="absolute right-0 top-0 h-full w-96 shadow-xl z-20 flex flex-col border-l">
              <TestPanel
                deviceId={deviceId}
                ir={buildIrFromGraph({
                  kind: builder.kind,
                  nodes: builder.nodes,
                  edges: builder.edges,
                  nextFlowIds,
                })}
                fields={fieldsForKind(builder.kind)}
              />
            </div>
          )}
        </div>
        {builder.selectedNode && (
          <NodeEditorPanel
            kind={builder.kind}
            node={builder.selectedNode}
            onChange={builder.updateNodeData}
            onClose={() => builder.setSelectedNodeId(null)}
          />
        )}
      </div>

      {otherScripts.length > 0 && (
        <NextFlowSelector
          scripts={otherScripts}
          selectedIds={nextFlowIds}
          currentScriptId={isNew ? null : script.id}
          onToggle={(id) => toggleNextFlow(id)}
          allScripts={allScripts ?? []}
        />
      )}

      <FlowEditorFooter
        isNew={isNew}
        pending={createScript.isPending || updateScript.isPending}
        onDelete={handleDelete}
        onTest={() => setShowTestPanel(!showTestPanel)}
        onSave={handleSave}
      />
    </div>
  );
}

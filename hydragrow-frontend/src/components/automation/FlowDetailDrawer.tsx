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
import { ConfigNodeInspector } from "./reactflow/ConfigNodeInspector";
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
  const [passContextVariables, setPassContextVariables] = useState<boolean>(
    isNew ? false : (script.ir_json?.chainConfig?.passContextVariables ?? false),
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
      setNextFlowIds(script.ir_json.next_flow_ids ?? []);
      setPassContextVariables(script.ir_json.chainConfig?.passContextVariables ?? false);
    } else {
      builder.setKind("alert");
      setNextFlowIds([]);
      setPassContextVariables(false);
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
      chainConfig: { passContextVariables },
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

  const [showAuditModal, setShowAuditModal] = useState(false);
  const isConfigNode = builder.selectedNode?.type === "config";
  const isConfigOverwrite = isConfigNode && builder.selectedNode?.data?.variant === "overwrite";

  return (
    <div data-testid="flow-detail-drawer" className="flex h-full flex-col p-4 overflow-y-auto bg-slate-50/40">
      {/* Top Header matching Reference 02 */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-3 mb-2 border-b border-emerald-100 bg-white p-3 rounded-2xl shadow-2xs">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-semibold text-emerald-950">
            {isNew ? "Flow mới" : `Sửa: ${script.name}`}
          </h2>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="ui-input font-bold text-base text-emerald-950 px-2.5 py-1 w-52"
            placeholder="Tên Flow..."
          />
          <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-md bg-indigo-100 text-indigo-800 border border-indigo-200">
            {builder.kind.toUpperCase()}
          </span>
          <label className="flex items-center gap-1.5 text-xs text-emerald-900 cursor-pointer ml-2">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="rounded text-emerald-600 focus:ring-emerald-500"
            />
            <span>Đang bật</span>
          </label>
        </div>

        <div className="flex items-center gap-2">
          {!isNew && (
            <button
              type="button"
              onClick={handleDelete}
              className="px-3 py-1.5 rounded-xl border border-rose-200 bg-rose-50 text-rose-700 text-xs font-semibold hover:bg-rose-100 transition-colors cursor-pointer"
            >
              Xóa Flow
            </button>
          )}
          <button
            type="button"
            onClick={() => setShowTestPanel(!showTestPanel)}
            className="px-3 py-1.5 rounded-xl border border-slate-200 bg-white text-slate-700 text-xs font-semibold hover:bg-slate-50 transition-colors flex items-center gap-1.5 cursor-pointer"
          >
            Chạy thử
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={createScript.isPending || updateScript.isPending}
            className="px-4 py-1.5 rounded-xl bg-emerald-600 text-white text-xs font-bold hover:bg-emerald-700 transition-colors shadow-2xs cursor-pointer"
          >
            Lưu Flow
          </button>
          <button
            type="button"
            onClick={onClose}
            className="px-3 py-1.5 rounded-xl border border-slate-200 text-slate-600 text-xs font-semibold hover:bg-slate-100 transition-colors cursor-pointer"
          >
            Đóng ✕
          </button>
        </div>
      </div>

      <NodePalette onAddNode={builder.addNode} onUpdateTrigger={builder.updateTrigger} />

      <div className="flex flex-1 flex-col lg:flex-row overflow-hidden rounded-2xl border border-emerald-100 bg-white relative my-2 min-h-[420px]">
        <div className="h-full w-full flex-1 relative">
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
            <div className="absolute right-0 top-0 h-full w-96 shadow-2xl z-30 flex flex-col border-l border-emerald-100 bg-white">
              <div className="flex items-center justify-between p-2 border-b">
                <span className="text-xs font-bold text-slate-500 uppercase px-2">Dry Run Simulator</span>
                <button
                  type="button"
                  onClick={() => setShowTestPanel(false)}
                  className="p-1 rounded text-slate-400 hover:text-slate-600"
                >
                  ✕
                </button>
              </div>
              <TestPanel
                deviceId={deviceId}
                ir={buildIrFromGraph({
                  kind: builder.kind,
                  nodes: builder.nodes,
                  edges: builder.edges,
                  nextFlowIds,
                  chainConfig: { passContextVariables },
                })}
                fields={fieldsForKind(builder.kind)}
              />
            </div>
          )}
        </div>

        {/* Node Editor Panel (handles Trigger, Condition, Config Read, Config Overwrite, Action) */}
        {builder.selectedNode && (
          <NodeEditorPanel
            kind={builder.kind}
            node={builder.selectedNode}
            nodes={builder.nodes}
            edges={builder.edges}
            onChange={builder.updateNodeData}
            onClose={() => builder.setSelectedNodeId(null)}
            onOpenAuditModal={() => setShowAuditModal(true)}
          />
        )}

        {/* Specialized Config Node Inspector Modal (Opened on demand for audit & safety visualization) */}
        {showAuditModal && isConfigOverwrite && builder.selectedNode && (
          <ConfigNodeInspector
            initialKey={(builder.selectedNode.data?.configKey as string) ?? "ec_target"}
            initialValue={Number(builder.selectedNode.data?.overrideValue ?? 1.8)}
            onSave={(updated: { configKey: string; overrideValue: number; applyMode: string; autoRestore: boolean }) => {
              builder.updateNodeData(builder.selectedNode!.id, {
                ...builder.selectedNode!.data,
                configKey: updated.configKey,
                overrideValue: updated.overrideValue,
                applyMode: updated.applyMode,
                autoRestore: updated.autoRestore,
                summary: `Ghi đè ${updated.configKey} -> ${updated.overrideValue}`,
              });
              setShowAuditModal(false);
            }}
            onClose={() => setShowAuditModal(false)}
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
          passContextVariables={passContextVariables}
          onTogglePassContext={setPassContextVariables}
        />
      )}
    </div>
  );
}

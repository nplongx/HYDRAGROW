import { useEffect, useState } from "react";
import toast from "react-hot-toast";
import { ReactFlow, Background, Controls } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { AUTOMATION_NODE_TYPES } from "./reactflow/nodeTypes";
import { NodeEditorPanel } from "./reactflow/NodeEditorPanel";
import { buildIrFromGraph } from "./reactflow/buildIr";
import { TestPanel } from "./reactflow/TestPanel";
import { fieldsForKind } from "../../hooks/useAutomationBuilder";
import { ConfigNodeInspector } from "./reactflow/ConfigNodeInspector";
import { NextFlowSelector } from "./NextFlowSelector";
import { WebhookAndChainPanel } from "./WebhookAndChainPanel";
import { RuleBuilder } from "./RuleBuilder";
import type { WebhookTriggerConfig } from "../../lib/automation/ir";
import { useAutomationBuilder } from "../../hooks/useAutomationBuilder";
import { AutomationIrSchema, type AutomationIr } from "../../lib/automation/ir";
import { compileToRhai } from "../../lib/automation/compileToRhai";
import { summarizeConditionTree } from "../../lib/automation/conditionTree";
import type { UserScript } from "../../types/automation";
import {
  useAutomationScripts,
  useConfigOverrides,
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
  const [enabled, setEnabled] = useState(isNew ? false : script.enabled);
  const [showTestPanel, setShowTestPanel] = useState(false);
  const [safetyReviewed, setSafetyReviewed] = useState(false);
  const [validationResult, setValidationResult] = useState<{ valid: boolean; error?: string } | null>(null);
  const [nextFlowIds, setNextFlowIds] = useState<string[]>(
    isNew ? [] : (script.ir_json?.next_flow_ids ?? []),
  );
  const [passContextVariables, setPassContextVariables] = useState<boolean>(
    isNew
      ? false
      : (script.ir_json?.chainConfig?.passContextVariables ?? false),
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
      setPassContextVariables(
        script.ir_json.chainConfig?.passContextVariables ?? false,
      );
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
      setValidationResult({ valid: false, error: parsed.error.issues[0]?.message });
      toast.error(`IR không hợp lệ: ${parsed.error.issues[0]?.message}`);
      return;
    }
    if (!name.trim()) {
      setValidationResult({ valid: false, error: "Tên automation không được để trống." });
      toast.error("Tên automation không được để trống.");
      return;
    }
    if (parsed.data.conditions.length === 0) {
      setValidationResult({ valid: false, error: "Phải có ít nhất một điều kiện." });
      toast.error("Phải có ít nhất một điều kiện.");
      return;
    }
    const isMutating = parsed.data.kind === "action_command" || parsed.data.kind === "recipe_override";
    if (enabled && isMutating && !safetyReviewed) {
      toast.error("Phải hoàn tất Safety Review trước khi bật automation có tác động.");
      return;
    }
    const source = compileToRhai(parsed.data);
    let validation;
    try {
      validation = await validateScript.mutateAsync({
        id: isNew ? undefined : script.id,
        kind: parsed.data.kind,
        name: name.trim(),
        source,
        ir_json: parsed.data,
        next_flow_ids: nextFlowIds,
      });
    } catch {
      setValidationResult({ valid: false, error: "Không thể xác thực với backend." });
      toast.error("Không thể xác thực automation với backend.");
      return;
    }
    setValidationResult(validation);
    if (!validation.valid) {
      toast.error(`Script không hợp lệ: ${validation.error}`);
      return;
    }
    if (isNew) {
      await createScript.mutateAsync({
        kind: parsed.data.kind,
        name: name.trim(),
        source,
        enabled,
        ir_json: parsed.data,
      });
    } else {
      await updateScript.mutateAsync({
        kind: parsed.data.kind,
        name: name.trim(),
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

  const selectedNode = builder.selectedNode;
  const selectedData = (selectedNode?.data ?? {}) as Record<string, unknown>;
  const isConfigOverwrite =
    selectedNode?.type === "config" && selectedData?.variant === "overwrite";

  const conditionSummary = summarizeConditionTree(
    (builder.nodes ?? [])
      .filter((n) => n.type === "condition" || n.type === "condition_group")
      .flatMap((n) =>
        Array.isArray((n.data as Record<string, unknown>)?.conditions)
          ? ((n.data as Record<string, unknown>).conditions as Parameters<
              typeof summarizeConditionTree
            >[0])
          : [],
      ),
  );

  const { data: configOverridesData } = useConfigOverrides(deviceId);
  const selectedConfigKey = selectedData?.configKey as string | undefined;
  const auditLogsForSelectedKey = (configOverridesData?.history ?? []).filter(
    (l) => !selectedConfigKey || l.configKey === selectedConfigKey,
  );

  const triggerNode = builder.nodes.find((n) => n.type === "trigger");
  const triggerConfig = triggerNode?.data?.trigger as
    WebhookTriggerConfig | undefined;
  const isWebhookTrigger = triggerConfig?.type === "webhook";
  const configOverwriteNode = builder.nodes.find(
    (n) =>
      n.type === "config" &&
      (n.data as Record<string, unknown>)?.variant === "overwrite",
  );
  const configOverwriteSummary = configOverwriteNode
    ? `${(configOverwriteNode.data as Record<string, unknown>)?.configKey} → ${(configOverwriteNode.data as Record<string, unknown>)?.overrideValue}`
    : undefined;

  return (
    <div
      data-testid="flow-detail-drawer"
      className="flex h-full flex-col overflow-y-auto bg-surface-muted/40"
    >
      {/* Top Header matching Reference 02 */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-3 mb-2 border-b border-line bg-white p-3 rounded-2xl shadow-2xs">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-semibold text-text">
            {isNew ? "Flow mới" : `Sửa: ${script.name}`}
          </h2>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="ui-input font-bold text-base text-text px-2.5 py-1 w-52"
            placeholder="Tên Flow..."
          />
          <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-md bg-config-soft text-config border border-info">
            {builder.kind.toUpperCase()}
          </span>
          <span className="text-[11px] font-semibold rounded-full px-2.5 py-1 bg-soft text-text-muted">
            {enabled ? "Đang bật" : "Tạm dừng"}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {!isNew && (
            <button
              type="button"
              onClick={handleDelete}
              className="px-3 py-1.5 rounded-xl border border-fault bg-danger-bg text-error text-xs font-semibold hover:bg-danger-bg transition-colors cursor-pointer"
            >
              Xóa Flow
            </button>
          )}
          <button
            type="button"
            onClick={() => setShowTestPanel(!showTestPanel)}
            className="px-3 py-1.5 rounded-xl border border-line bg-white text-text-secondary text-xs font-semibold hover:bg-surface-muted transition-colors flex items-center gap-1.5 cursor-pointer"
          >
            Mô phỏng — không gửi lệnh
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={createScript.isPending || updateScript.isPending}
            className="px-4 py-1.5 rounded-xl bg-config text-white text-xs font-bold hover:bg-config transition-colors shadow-2xs cursor-pointer"
          >
            Lưu Flow
          </button>
          <button
            type="button"
            onClick={onClose}
            className="px-3 py-1.5 rounded-xl border border-line text-text-muted text-xs font-semibold hover:bg-surface-muted transition-colors cursor-pointer"
          >
            Đóng ✕
          </button>
        </div>
      </div>

      <RuleBuilder builder={builder} />

      <div className="shrink-0 border-t border-line bg-white p-3 sm:p-4">
        <div className="mx-auto max-w-3xl space-y-3">
          <div className="flex items-center gap-3">
            <span className="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-white text-xs font-bold">4</span>
            <div>
              <h3 className="font-bold text-primary-deep">Xác nhận</h3>
              <p className="text-xs text-text-muted">Validation · Dry Run · Safety Review</p>
            </div>
          </div>
          <div className="grid gap-3 lg:grid-cols-3">
            <div className="rounded-xl border border-line bg-soft p-3 text-xs">
              <div className="font-semibold text-primary-deep">Validation</div>
              <div className="mt-1 text-text-muted">{validationResult?.valid ? "Đã validate backend" : validationResult?.error ?? "Chưa validate"}</div>
            </div>
            <div className="rounded-xl border border-line bg-soft p-3 text-xs">
              <div className="font-semibold text-primary-deep">Dry Run</div>
              <div className="mt-1 text-text-muted">Mô phỏng — không gửi lệnh</div>
            </div>
            <div className="rounded-xl border border-line bg-soft p-3 text-xs">
              <div className="font-semibold text-primary-deep">Safety Review</div>
              <div className="mt-1 text-text-muted">{builder.kind === "alert" ? "Không có mutation vật lý" : safetyReviewed ? "Đã review" : "Bắt buộc trước khi bật"}</div>
            </div>
          </div>
          {builder.kind !== "alert" && (
            <label className="flex items-start gap-2 rounded-xl border border-warning bg-warning-bg p-3 text-xs text-warning">
              <input type="checkbox" checked={safetyReviewed} onChange={(e) => setSafetyReviewed(e.target.checked)} className="mt-0.5" />
              <span>Tôi đã kiểm tra station, capability, Safety Gate/interlock và control authority. Automation không bypass safety.</span>
            </label>
          )}
          <div className="flex flex-wrap gap-2">
            <button type="button" onClick={() => setShowTestPanel(true)} className="rounded-xl border border-primary bg-white px-3 py-2 text-xs font-semibold text-primary">Chạy mô phỏng</button>
            <button type="button" onClick={() => setEnabled((v) => !v)} className="rounded-xl border border-line bg-white px-3 py-2 text-xs font-semibold">{enabled ? "Tạm dừng" : "Bật automation"}</button>
          </div>
        </div>
      </div>

      <div className="shrink-0 border-t border-line bg-surface-muted/40 px-4 py-2 text-[11px] font-bold uppercase tracking-wider text-text-muted">
        Advanced Canvas · inspection / graph authoring
      </div>
      <div className="shrink-0 h-[420px] flex flex-col lg:flex-row overflow-hidden border-t border-line bg-white relative">
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
            <div className="absolute right-0 top-0 h-full w-96 shadow-2xl z-30 flex flex-col border-l border-line bg-white">
              <div className="flex items-center justify-between p-2 border-b">
                <span className="text-xs font-bold text-text-muted uppercase px-2">
                  Dry Run Simulator
                </span>
                <button
                  type="button"
                  onClick={() => setShowTestPanel(false)}
                  className="p-1 rounded text-text-muted hover:text-text-muted"
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

        {/* Node Editor Panel (handles Trigger, Condition, Config Read, Action) */}
        {builder.selectedNode && !isConfigOverwrite && (
          <NodeEditorPanel
            kind={builder.kind}
            node={builder.selectedNode}
            nodes={builder.nodes}
            edges={builder.edges}
            availableFlows={otherScripts}
            onChange={builder.updateNodeData}
            onClose={() => builder.setSelectedNodeId(null)}
          />
        )}

        {/* Config·Overwrite nodes open ConfigNodeInspector directly */}
        {isConfigOverwrite && builder.selectedNode && (
          <ConfigNodeInspector
            deviceId={deviceId}
            initialKey={(selectedData.configKey as string) ?? "ec_target"}
            initialValue={(() => {
              const n = Number(selectedData.overrideValue ?? 1.8);
              return Number.isNaN(n) ? 1.8 : n;
            })()}
            initialAutoRestore={
              (selectedData.readOriginalBeforeWrite as boolean) ?? true
            }
            initialPriority={Number(selectedData.priority ?? 0)}
            conditionSummary={conditionSummary}
            auditLogs={auditLogsForSelectedKey}
            onSave={(updated: {
              configKey: string;
              overrideValue: number;
              autoRestore: boolean;
              priority: number;
            }) => {
              builder.updateNodeData(builder.selectedNode!.id, {
                ...builder.selectedNode!.data,
                configKey: updated.configKey,
                overrideValue: updated.overrideValue,
                autoRestore: updated.autoRestore,
                readOriginalBeforeWrite: updated.autoRestore,
                priority: updated.priority,
                summary: `Ghi đè ${updated.configKey} -> ${updated.overrideValue}`,
              });
              builder.setSelectedNodeId(null);
            }}
            onClose={() => builder.setSelectedNodeId(null)}
          />
        )}
      </div>

      {isWebhookTrigger ? (
        <WebhookAndChainPanel
          webhookUrl={
            (triggerNode?.data as Record<string, unknown>)?.endpoint as
              string | undefined
          }
          mode={triggerConfig?.mode ?? "flow"}
          onModeChange={(mode) =>
            builder.updateNodeData(triggerNode!.id, {
              ...triggerNode!.data,
              trigger: { ...triggerConfig, type: "webhook", mode },
            })
          }
          mappings={triggerConfig?.fieldMappings ?? []}
          onMappingsChange={(fieldMappings) =>
            builder.updateNodeData(triggerNode!.id, {
              ...triggerNode!.data,
              trigger: { ...triggerConfig, type: "webhook", fieldMappings },
            })
          }
          currentScriptName={isNew ? name : script.name}
          currentScriptKind={builder.kind}
          configOverwriteSummary={configOverwriteSummary}
          scripts={otherScripts}
          selectedNextFlowIds={nextFlowIds}
          onToggleNextFlow={(id) => toggleNextFlow(id)}
        />
      ) : (
        otherScripts.length > 0 && (
          <NextFlowSelector
            scripts={otherScripts}
            selectedIds={nextFlowIds}
            currentScriptId={isNew ? null : script.id}
            onToggle={(id) => toggleNextFlow(id)}
            allScripts={allScripts ?? []}
            passContextVariables={passContextVariables}
            onTogglePassContext={setPassContextVariables}
          />
        )
      )}
    </div>
  );
}

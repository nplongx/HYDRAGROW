import { useMemo } from "react";
import type { Action, AutomationIr, ConditionGroup, ConditionOrGroup } from "../../lib/automation/ir";
import { fieldsForKind, summarizeActions } from "../../hooks/useAutomationBuilder";
import { summarizeConditionTree } from "../../lib/automation/conditionTree";
import { ConditionGroupEditor } from "./reactflow/ConditionGroupEditor";
import type { Node } from "@xyflow/react";

type BuilderLike = {
  kind: AutomationIr["kind"];
  setKind: (kind: AutomationIr["kind"]) => void;
  nodes: Node[];
  updateNodeData: (nodeId: string, data: Record<string, unknown>) => void;
};

function ensureNode(builder: BuilderLike, type: string, fallbackId: string) {
  return builder.nodes.find((n) => n.type === type) ?? builder.nodes.find((n) => n.id === fallbackId);
}

function updateCondition(builder: BuilderLike, next: ConditionOrGroup[]) {
  const node = ensureNode(builder, "condition", "2");
  if (!node) return;
  builder.updateNodeData(node.id, { ...node.data, conditions: next, summary: summarizeConditionTree(next) });
}

function updateAction(builder: BuilderLike, action: Action) {
  const node = ensureNode(builder, "action", "3");
  if (!node) return;
  builder.updateNodeData(node.id, { ...node.data, actions: [action], summary: summarizeActions([action]) });
}

export function RuleBuilder({ builder }: { builder: BuilderLike }) {
  const triggerNode = builder.nodes.find((n) => n.type === "trigger");
  const conditionNode = ensureNode(builder, "condition", "2");
  const actionNode = ensureNode(builder, "action", "3");
  const trigger = (triggerNode?.data.trigger as AutomationIr["trigger"] | undefined)
    ?? { type: builder.kind === "recipe_override" ? "fsm" : "sensor" };
  const conditions = (conditionNode?.data.conditions as ConditionOrGroup[] | undefined) ?? [];
  const action = ((actionNode?.data.actions as Action[] | undefined) ?? [])[0];
  const fields = fieldsForKind(builder.kind);
  const wrapperGroup = useMemo<ConditionGroup>(() => ({ op: "and", children: conditions }), [conditions]);
  const mutating = builder.kind === "action_command" || builder.kind === "recipe_override";

  const setTrigger = (type: "sensor" | "fsm" | "cron" | "webhook") => {
    if (!triggerNode) return;
    const next: AutomationIr["trigger"] = type === "cron"
      ? { type: "cron", cronExpression: "0 0 7 * * *", timezone: "Asia/Ho_Chi_Minh" }
      : type === "webhook"
        ? { type: "webhook", mode: "flow", fieldMappings: [] }
        : { type };
    builder.updateNodeData(triggerNode.id, { ...triggerNode.data, kind: type, trigger: next });
  };

  const actionType = action?.type ?? (builder.kind === "alert" ? "alert" : builder.kind === "recipe_override" ? "advance_stage" : "dose");

  const setKind = (kind: AutomationIr["kind"]) => {
    builder.setKind(kind);
    const nextAction: Action = kind === "alert"
      ? { type: "alert", level: "warning", message: "Automation được kích hoạt" }
      : kind === "recipe_override"
        ? { type: "advance_stage", targetStageOffset: 1, reason: "Automation" }
        : { type: "dose", pump: "PUMP_A", doseMl: 10, pwm: 60 };
    requestAnimationFrame(() => updateAction(builder, nextAction));
  };

  return (
    <div className="flex-1 overflow-y-auto bg-surface-muted/40 p-3 sm:p-4">
      <div className="mx-auto max-w-3xl space-y-4 pb-24">
        <section className="rounded-2xl border border-line bg-white p-4 shadow-sm">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="text-[11px] font-bold uppercase tracking-wider text-primary">Ý định</div>
              <h3 className="mt-1 text-base font-bold text-primary-deep">Rule Automation</h3>
            </div>
            <select aria-label="Ý định automation" className="ui-input text-xs" value={builder.kind} onChange={(e) => setKind(e.target.value as AutomationIr["kind"])}>
              <option value="alert">Cảnh báo</option>
              <option value="action_command">Điều khiển thiết bị</option>
              <option value="recipe_override">Điều chỉnh giai đoạn</option>
            </select>
          </div>
          <p className="mt-2 text-xs text-text-muted">Chỉ các capability hiện có trong IR/runtime được cho phép thực thi.</p>
        </section>

        <section className="rounded-2xl border border-line bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-3">
            <span className="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-white text-xs font-bold">1</span>
            <div><h3 className="font-bold text-primary-deep">Khi nào?</h3><p className="text-xs text-text-muted">Trigger</p></div>
          </div>
          <div className="grid gap-3 sm:grid-cols-3">
            {(["sensor", "fsm", "cron", "webhook"] as const).map((type) => (
              <button key={type} type="button" onClick={() => setTrigger(type)}
                className={trigger.type === type ? "rounded-xl border border-primary bg-pill p-3 text-left text-xs" : "rounded-xl border border-line p-3 text-left text-xs hover:bg-soft"}>
                <div className="font-bold text-primary-deep">{type === "sensor" ? "Cảm biến" : type === "fsm" ? "Giai đoạn FSM" : type === "cron" ? "Lịch Cron" : "Webhook"}</div>
                <div className="mt-1 text-text-muted">{type === "sensor" ? "EC, pH, nhiệt độ, mực nước" : type === "fsm" ? "Sự kiện giai đoạn canh tác" : type === "cron" ? "Chạy theo lịch định kỳ" : "Nhận event từ nguồn bên ngoài"}</div>
              </button>
            ))}
          </div>
          {trigger.type === "cron" && (
            <div className="mt-3 grid gap-3 sm:grid-cols-[1fr_auto]">
              <input className="ui-input font-mono text-xs" aria-label="Cron expression" value={trigger.cronExpression}
                onChange={(e) => triggerNode && builder.updateNodeData(triggerNode.id, { ...triggerNode.data, kind: "cron", trigger: { type: "cron", cronExpression: e.target.value, timezone: trigger.timezone } })} />
              <span className="rounded-xl bg-soft px-3 py-2 text-xs text-text-muted">Asia/Ho_Chi_Minh</span>
            </div>
          )}
          {trigger.type === "webhook" && triggerNode && (
            <div className="mt-3 rounded-xl border border-line bg-soft p-3">
              <label className="text-xs font-semibold text-primary-deep">Chế độ webhook</label>
              <select
                className="ui-input mt-2 w-full text-xs"
                value={trigger.mode}
                onChange={(e) => builder.updateNodeData(triggerNode.id, {
                  ...triggerNode.data,
                  kind: "webhook",
                  trigger: { ...trigger, mode: e.target.value as "flow" | "direct" },
                })}
              >
                <option value="flow">Flow</option>
                <option value="direct">Direct</option>
              </select>
              <p className="mt-2 text-[11px] text-text-muted">Field mapping và endpoint chi tiết nằm trong Advanced Canvas.</p>
            </div>
          )}
        </section>

        <section className="rounded-2xl border border-line bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-3">
            <span className="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-white text-xs font-bold">2</span>
            <div><h3 className="font-bold text-primary-deep">Điều kiện?</h3><p className="text-xs text-text-muted">Conditions · AND/OR chỉ xuất hiện khi cần</p></div>
          </div>
          <ConditionGroupEditor group={wrapperGroup} fields={fields} availableVariables={[]} isRoot onChange={(next) => updateCondition(builder, next.children)} />
          {conditions.length === 0 && <p className="mt-3 rounded-xl border border-warning bg-warning-bg p-3 text-xs text-warning">Chưa có điều kiện. Rule chưa thể validate/enable.</p>}
        </section>

        <section className="rounded-2xl border border-line bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-3">
            <span className="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-white text-xs font-bold">3</span>
            <div><h3 className="font-bold text-primary-deep">Làm gì?</h3><p className="text-xs text-text-muted">Action · requested impact, not measured state</p></div>
          </div>

          {builder.kind === "alert" && (
            <div className="grid gap-3">
              <select className="ui-input text-xs" aria-label="Mức cảnh báo" value={action?.type === "alert" ? action.level : "warning"}
                onChange={(e) => updateAction(builder, { type: "alert", level: e.target.value as "info" | "warning" | "error", message: action?.type === "alert" ? action.message : "Automation được kích hoạt" })}>
                <option value="info">Info</option><option value="warning">Warning</option><option value="error">Critical</option>
              </select>
              <textarea className="ui-input text-xs" rows={3} aria-label="Nội dung cảnh báo" value={action?.type === "alert" ? action.message : ""}
                onChange={(e) => updateAction(builder, { type: "alert", level: action?.type === "alert" ? action.level : "warning", message: e.target.value })} />
            </div>
          )}

          {builder.kind === "recipe_override" && (
            <div className="grid gap-3 sm:grid-cols-2">
              <select className="ui-input text-xs" value={actionType}
                onChange={(e) => updateAction(builder, e.target.value === "end_season" ? { type: "end_season", reason: "Automation" } : { type: "advance_stage", targetStageOffset: 1, reason: "Automation" })}>
                <option value="advance_stage">Chuyển giai đoạn</option><option value="end_season">Kết thúc vụ</option>
              </select>
              <input className="ui-input text-xs" placeholder="Lý do" value={action?.type === "advance_stage" || action?.type === "end_season" ? action.reason : ""}
                onChange={(e) => updateAction(builder, action?.type === "end_season" ? { type: "end_season", reason: e.target.value } : { type: "advance_stage", targetStageOffset: 1, reason: e.target.value })} />
            </div>
          )}

          {builder.kind === "action_command" && (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <select className="ui-input text-xs" value={action?.type ?? "dose"} aria-label="Action"
                onChange={(e) => {
                  const type = e.target.value;
                  if (type === "water_on") updateAction(builder, { type, pump: "WATER_PUMP_IN", durationSec: 30 });
                  else if (type === "water_off") updateAction(builder, { type, pump: "WATER_PUMP_IN" });
                  else updateAction(builder, { type: "dose", pump: "PUMP_A", doseMl: 10, pwm: 60 });
                }}>
                <option value="dose">Châm định lượng</option><option value="water_on">Bật bơm nước</option><option value="water_off">Tắt bơm nước</option>
              </select>
              {action?.type === "dose" && <>
                <select className="ui-input text-xs" value={action.pump} onChange={(e) => updateAction(builder, { ...action, pump: e.target.value as "PUMP_A" | "PUMP_B" | "PH_UP" | "PH_DOWN" })}>
                  <option value="PUMP_A">PUMP_A</option><option value="PUMP_B">PUMP_B</option><option value="PH_UP">PH_UP</option><option value="PH_DOWN">PH_DOWN</option>
                </select>
                <div className="grid grid-cols-2 gap-2">
                  <input className="ui-input text-xs" type="number" min={1} aria-label="doseMl" value={action.doseMl} onChange={(e) => updateAction(builder, { ...action, doseMl: Number(e.target.value) })} />
                  <input className="ui-input text-xs" type="number" min={1} max={100} aria-label="PWM" value={action.pwm} onChange={(e) => updateAction(builder, { ...action, pwm: Number(e.target.value) })} />
                </div>
              </>}
              {action?.type === "water_on" && <>
                <select className="ui-input text-xs" value={action.pump} onChange={(e) => updateAction(builder, { ...action, pump: e.target.value as "WATER_PUMP_IN" | "WATER_PUMP_OUT" })}><option value="WATER_PUMP_IN">WATER_PUMP_IN</option><option value="WATER_PUMP_OUT">WATER_PUMP_OUT</option></select>
                <input className="ui-input text-xs" type="number" min={1} aria-label="durationSec" value={action.durationSec} onChange={(e) => updateAction(builder, { ...action, durationSec: Number(e.target.value) })} />
              </>}
              {action?.type === "water_off" && <select className="ui-input text-xs" value={action.pump} onChange={(e) => updateAction(builder, { ...action, pump: e.target.value as "WATER_PUMP_IN" | "WATER_PUMP_OUT" })}><option value="WATER_PUMP_IN">WATER_PUMP_IN</option><option value="WATER_PUMP_OUT">WATER_PUMP_OUT</option></select>}
            </div>
          )}
          {mutating && <div className="mt-3 rounded-xl border border-warning bg-warning-bg p-3 text-xs text-warning">Hành động vật lý sẽ đi qua Control Authority và Safety Gate. Automation không tắt/bypass safety.</div>}
        </section>
      </div>
    </div>
  );
}

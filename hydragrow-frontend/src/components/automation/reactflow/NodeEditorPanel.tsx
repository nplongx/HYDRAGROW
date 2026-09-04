import { useState } from "react";
import type {
  AutomationIr,
  WebhookTriggerConfig,
} from "../../../lib/automation/ir";
import { fieldsForKind } from "../../../hooks/useAutomationBuilder";
import { ConditionGroupEditor } from "./ConditionGroupEditor";
import { WebhookFieldMappingEditor } from "./WebhookFieldMappingEditor";

export interface NodeEditorPanelProps {
  kind: AutomationIr["kind"];
  node: { id: string; type?: string; data: Record<string, unknown> };
  onChange: (nodeId: string, data: Record<string, unknown>) => void;
  onClose: () => void;
}

export function NodeEditorPanel({
  kind,
  node,
  onChange,
  onClose,
}: NodeEditorPanelProps) {
  const fields = fieldsForKind(kind);
  const [triggerTab, setTriggerTab] = useState<"sensor" | "fsm" | "cron" | "webhook">("sensor");

  if (node.type === "trigger" || node.id === "trigger") {
    const currentKind = (node.data.kind as "sensor" | "fsm" | "cron" | "webhook") || triggerTab;
    const cronExp = (node.data.expression as string) || "0 7 * * *";
    const isCronValid = /^(\*|\d+|\*\/\d+|\d+-\d+|\d+(,\d+)*)(\s+(\*|\d+|\*\/\d+|\d+-\d+|\d+(,\d+)*)){4}$/.test(cronExp) && cronExp !== 'invalid cron';

    return (
      <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Trigger</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
        </div>
        <div className="mb-3 flex border-b border-emerald-100 text-xs">
          {(["sensor", "fsm", "cron", "webhook"] as const).map((tab) => (
            <button
              key={tab}
              className={`flex-1 py-1 text-center font-medium capitalize ${currentKind === tab ? "border-b-2 border-emerald-600 text-emerald-900" : "text-emerald-700/60 hover:text-emerald-800"}`}
              onClick={() => {
                setTriggerTab(tab);
                onChange(node.id, { kind: tab });
              }}
            >
              {tab}
            </button>
          ))}
        </div>
        {currentKind === "sensor" && (
          <p className="text-xs text-emerald-800/80">Kích hoạt dựa trên dữ liệu cảm biến MQTT từ thiết bị.</p>
        )}
        {currentKind === "fsm" && (
          <p className="text-xs text-emerald-800/80">Kích hoạt khi chuyển đổi trạng thái FSM (chuyển phase).</p>
        )}
        {currentKind === "cron" && (
          <div className="flex flex-col gap-3">
            <h4 className="font-medium text-sm text-emerald-950">Cấu hình Lịch (Cron)</h4>
            <div>
              <label className="mb-1 block text-xs font-medium text-emerald-950">Lịch dựng sẵn</label>
              <select
                className="ui-input w-full text-xs"
                onChange={(e) => {
                  const val = e.target.value;
                  if (val === "daily_7am") {
                    onChange(node.id, {
                      kind: "cron",
                      expression: "0 0 7 * * *",
                      trigger: { type: "cron", cronExpression: "0 0 7 * * *", timezone: "Asia/Ho_Chi_Minh" },
                    });
                  } else if (val === "hourly") {
                    onChange(node.id, {
                      kind: "cron",
                      expression: "0 0 * * * *",
                      trigger: { type: "cron", cronExpression: "0 0 * * * *", timezone: "Asia/Ho_Chi_Minh" },
                    });
                  } else if (val === "weekly_mon") {
                    onChange(node.id, {
                      kind: "cron",
                      expression: "0 0 7 * * 1",
                      trigger: { type: "cron", cronExpression: "0 0 7 * * 1", timezone: "Asia/Ho_Chi_Minh" },
                    });
                  }
                }}
                defaultValue=""
              >
                <option value="" disabled>-- Chọn lịch mẫu --</option>
                <option value="daily_7am">Mỗi ngày lúc 07:00</option>
                <option value="hourly">Mỗi giờ (phút thứ 0)</option>
                <option value="weekly_mon">Mỗi thứ Hai lúc 07:00</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-xs text-emerald-800/75 font-medium">Biểu thức Cron (6 fields: s m h dom mon dow)</label>
              <input
                type="text"
                value={(node.data.trigger as any)?.cronExpression || cronExp}
                onChange={(e) => {
                  const val = e.target.value;
                  onChange(node.id, {
                    kind: "cron",
                    expression: val,
                    trigger: { type: "cron", cronExpression: val, timezone: "Asia/Ho_Chi_Minh" },
                  });
                }}
                className="ui-input text-xs font-mono"
                placeholder="0 0 7 * * *"
              />
            </div>
            <div className="rounded bg-emerald-50/70 p-2 text-xs text-emerald-850">
              <span className="font-semibold text-emerald-950">Múi giờ:</span> Asia/Ho_Chi_Minh (GMT+7)
            </div>
            <p className="text-xs text-emerald-800/80 leading-relaxed">Kích hoạt định kỳ chính xác theo lịch biểu hệ thống.</p>
          </div>
        )}
        {currentKind === "webhook" && (
          <WebhookFieldMappingEditor
            config={(node.data.trigger as WebhookTriggerConfig | undefined) ?? { type: "webhook", mode: "flow", fieldMappings: [] }}
            onChange={(cfg) => onChange(node.id, { ...node.data, kind: "webhook", trigger: cfg })}
          />
        )}
      </div>
    );
  }

  if (node.type === "condition" || node.type === "condition_group") {
    return (
      <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Điều kiện</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
        </div>
        <ConditionGroupEditor
          group={
            node.type === "condition_group"
              ? (node.data as any)
              : { op: "and", children: [node.data as any] }
          }
          fields={fields}
          onChange={(g) => onChange(node.id, g as any)}
          isRoot={true}
        />
      </div>
    );
  }

  if (node.type === "action") {
    const current = node.data as any;
    const setAction = (updates: any) => onChange(node.id, updates);

    if (current?.type === "chain") {
      return (
        <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Hành động — Kích hoạt Flow khác</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <p className="text-xs text-emerald-800/80 mb-2">Để chọn Flow cần kích hoạt tiếp theo, vui lòng sử dụng phần "Flow kế tiếp" bên dưới biểu đồ.</p>
          <div className="rounded border border-blue-100 bg-blue-50 p-2 text-center text-xs text-blue-800">Node này chỉ có tính chất minh họa trực quan trên sơ đồ Flow.</div>
        </div>
      );
    }

    if (kind === "alert") {
      return (
        <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Alert</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <label className="mb-2 block text-xs text-emerald-800/75">
            Level
            <select
              className="ui-input mt-1"
              value={current?.level ?? "info"}
              onChange={(e) => setAction({ ...node.data, level: e.target.value })}
            >
              <option value="info">info</option>
              <option value="warning">warning</option>
              <option value="error">error</option>
            </select>
          </label>
          <label className="mb-2 block text-xs text-emerald-800/75">
            Title (optional)
            <input className="ui-input mt-1" value={current?.title ?? ""} onChange={(e) => setAction({ ...node.data, title: e.target.value })} />
          </label>
          <label className="block text-xs text-emerald-800/75">
            Message
            <input className="ui-input mt-1" value={current?.message ?? ""} onChange={(e) => setAction({ ...node.data, message: e.target.value })} />
          </label>
        </div>
      );
    }

    if (kind === "recipe_override") {
      const isEndSeason = current?.type === "end_season";
      return (
        <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Recipe</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <label className="mb-2 block text-xs text-emerald-800/75">
            Loại hành động
            <select
              className="ui-input mt-1"
              value={isEndSeason ? "end_season" : "advance_stage"}
              onChange={(e) => setAction(e.target.value === "end_season" ? { type: "end_season", reason: current?.reason ?? "" } : { type: "advance_stage", targetStageOffset: 1, reason: "" })}
            >
              <option value="advance_stage">advance_stage</option>
              <option value="end_season">end_season</option>
            </select>
          </label>
          {!isEndSeason ? (
            <>
              <label className="mb-2 block text-xs text-emerald-800/75">
                Target stage offset
                <input type="number" className="ui-input mt-1" value={current?.targetStageOffset ?? 1} onChange={(e) => setAction({ ...current, targetStageOffset: Number(e.target.value) })} />
              </label>
              <label className="block text-xs text-emerald-800/75">
                Reason
                <input className="ui-input mt-1" value={current?.reason ?? ""} onChange={(e) => setAction({ ...current, reason: e.target.value })} />
              </label>
            </>
          ) : (
            <label className="block text-xs text-emerald-800/75">
              Reason
              <input className="ui-input mt-1" value={current?.reason ?? ""} onChange={(e) => setAction({ ...current, reason: e.target.value })} />
            </label>
          )}
        </div>
      );
    }

    // action_command
    const isWater = current?.type === "water_on" || current?.type === "water_off";
    const isDose = current?.type === "dose";
    const actionVal = isDose ? "dose" : isWater ? current.type : "emergency_stop";

    return (
      <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Action — Điều khiển</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
        </div>
        <label className="mb-2 block text-xs text-emerald-800/75">
          Loại hành động
          <select
            className="ui-input mt-1"
            value={actionVal}
            onChange={(e) => {
              const next = e.target.value;
              if (next === "dose") setAction({ type: "dose", pump: "PUMP_A", doseMl: 1, pwm: 100 });
              else if (next === "water_on") setAction({ type: "water_on", pump: "WATER_PUMP_IN", durationSec: 10 });
              else if (next === "water_off") setAction({ type: "water_off", pump: "WATER_PUMP_IN" });
              else setAction({ type: "emergency_stop" });
            }}
          >
            <option value="dose">dose</option>
            <option value="water_on">water_on</option>
            <option value="water_off">water_off</option>
            <option value="emergency_stop">emergency_stop</option>
          </select>
        </label>

        {isDose && (
          <>
            <label className="mb-2 block text-xs text-emerald-800/75">
              Bơm
              <select className="ui-input mt-1" value={current.pump ?? "PUMP_A"} onChange={(e) => setAction({ ...current, pump: e.target.value })}>
                <option value="PUMP_A">PUMP_A</option>
                <option value="PUMP_B">PUMP_B</option>
                <option value="PH_UP">PH_UP</option>
                <option value="PH_DOWN">PH_DOWN</option>
              </select>
            </label>
            <label className="mb-2 block text-xs text-emerald-800/75">Liều (ml) <input type="number" className="ui-input mt-1" value={current.doseMl ?? 1} onChange={(e) => setAction({ ...current, doseMl: Number(e.target.value) })} /></label>
            <label className="block text-xs text-emerald-800/75">PWM (%) <input type="number" className="ui-input mt-1" value={current.pwm ?? 100} onChange={(e) => setAction({ ...current, pwm: Number(e.target.value) })} /></label>
          </>
        )}

        {isWater && (
          <>
            <label className="mb-2 block text-xs text-emerald-800/75">
              Bơm/van
              <select className="ui-input mt-1" value={current.pump ?? "WATER_PUMP_IN"} onChange={(e) => setAction({ ...current, pump: e.target.value })}>
                <option value="WATER_PUMP_IN">WATER_PUMP_IN</option>
                <option value="WATER_PUMP_OUT">WATER_PUMP_OUT</option>
                <option value="MIST_VALVE">MIST_VALVE</option>
                <option value="OSAKA_PUMP">OSAKA_PUMP</option>
              </select>
            </label>
            {actionVal === "water_on" && (
              <label className="block text-xs text-emerald-800/75">Thời gian (giây) <input type="number" className="ui-input mt-1" value={current.durationSec ?? 10} onChange={(e) => setAction({ ...current, durationSec: Number(e.target.value) })} /></label>
            )}
          </>
        )}
      </div>
    );
  }

  return null;
}

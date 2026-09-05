import { useState } from "react";
import type {
  Action,
  AutomationIr,
  ConditionOrGroup,
  WebhookTriggerConfig,
} from "../../../lib/automation/ir";
import {
  fieldsForKind,
  summarizeActions,
} from "../../../hooks/useAutomationBuilder";
import {
  toEditorRoot,
  fromEditorRoot,
  summarizeConditionTree,
} from "../../../lib/automation/conditionTree";
import { ConditionGroupEditor } from "./ConditionGroupEditor";
import { WebhookFieldMappingEditor } from "./WebhookFieldMappingEditor";

import { getAvailableContextVariables } from "../../../lib/automation/contextVariables";
import { extractTemplateTokens, renderTemplatePreview } from "../../../lib/automation/templateVars";
import { VariableCombobox } from "./VariableCombobox";
import {
  Badge,
  ConfigCard,
  FieldGroup,
  Chip,
  ChipsRow,
  Segmented,
  ToggleRow,
  SafeNote,
} from "./ConfigPanelUI";

const BUILTIN_TEMPLATE_TOKENS = ["time"] as const;

export const DEVICE_CONFIG_KEYS = [
  "ec_target",
  "ec_tolerance",
  "ph_target",
  "ph_tolerance",
  "control_mode",
  "is_enabled",
  "delay_between_a_and_b_sec",
];

export interface NodeEditorPanelProps {
  kind: AutomationIr["kind"];
  node: { id: string; type?: string; data: Record<string, unknown> };
  nodes?: Array<{ id: string; type?: string; data: Record<string, unknown> }>;
  edges?: Array<{ id: string; source: string; target: string }>;
  onChange: (nodeId: string, data: Record<string, unknown>) => void;
  onClose: () => void;
}

export function NodeEditorPanel({
  kind,
  node,
  nodes,
  edges,
  onChange,
  onClose,
}: NodeEditorPanelProps) {
  const fields = fieldsForKind(kind);
  const [triggerTab, setTriggerTab] = useState<"sensor" | "fsm" | "cron" | "webhook">("sensor");
  const availableVariables = getAvailableContextVariables(
    nodes ?? [],
    edges ?? [],
    node.id,
  );

  if (node.type === "trigger" || node.id === "trigger") {
    const currentKind = (node.data.kind as "sensor" | "fsm" | "cron" | "webhook") || triggerTab;
    const cronExp = (node.data.expression as string) || "0 0 7 * * *";
    const tabBadge: Record<typeof currentKind, string> = {
      sensor: "TRIGGER · SENSOR",
      fsm: "TRIGGER · FSM",
      cron: "TRIGGER · CRON",
      webhook: "TRIGGER · WEBHOOK",
    };

    return (
      <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
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
        <ConfigCard tone="sky">
          <Badge tone="sky">{tabBadge[currentKind]}</Badge>
          {currentKind === "sensor" && (
            <p className="text-xs text-emerald-800/80">Kích hoạt dựa trên dữ liệu cảm biến MQTT từ thiết bị.</p>
          )}
          {currentKind === "fsm" && (
            <p className="text-xs text-emerald-800/80">Kích hoạt khi chuyển đổi trạng thái FSM (chuyển phase).</p>
          )}
          {currentKind === "cron" && (
            <div className="flex flex-col gap-3">
              <FieldGroup label="Lịch dựng sẵn">
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
              </FieldGroup>
              <FieldGroup label="Biểu thức Cron (6 fields: s m h dom mon dow)">
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
              </FieldGroup>
              <div className="rounded-lg bg-emerald-50/70 p-2 text-xs text-emerald-850">
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
        </ConfigCard>
      </div>
    );
  }

  if (node.type === "condition" || node.type === "condition_group") {
    const rawConditions = Array.isArray(node.data?.conditions)
      ? (node.data.conditions as ConditionOrGroup[])
      : [];
    const rootGroup = toEditorRoot(rawConditions);

    return (
      <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Điều kiện</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
        </div>
        <ConfigCard tone="amber">
          <Badge tone="amber">CONDITION</Badge>
          <ConditionGroupEditor
            group={rootGroup}
            fields={fields}
            availableVariables={availableVariables}
            onChange={(g) => {
              const conditions = fromEditorRoot(g);
              onChange(node.id, {
                ...node.data,
                conditions,
                summary: summarizeConditionTree(conditions),
              });
            }}
            isRoot={true}
          />
        </ConfigCard>
      </div>
    );
  }

  if (node.type === "action") {
    const storedActions = Array.isArray(node.data?.actions)
      ? (node.data.actions as Action[])
      : [];
    const firstAction = storedActions[0];
    const setAction = (updates: Action) =>
      onChange(node.id, {
        ...node.data,
        actions: [updates],
        summary: summarizeActions([updates]),
      });

    if ((node.data as any)?.type === "chain" || (firstAction as any)?.type === "chain") {
      return (
        <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Hành động — Kích hoạt Flow khác</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <ConfigCard tone="emerald">
            <Badge tone="emerald">ACTION · CHAIN (SƠ ĐỒ)</Badge>
            <p className="text-xs text-emerald-800/80 mb-2">Để chọn Flow cần kích hoạt tiếp theo, vui lòng sử dụng phần "Flow kế tiếp" bên dưới biểu đồ.</p>
            <div className="rounded border border-sky-100 bg-sky-50 p-2 text-center text-xs text-sky-800">Node này chỉ có tính chất minh họa trực quan trên sơ đồ Flow.</div>
          </ConfigCard>
        </div>
      );
    }

    if (kind === "alert") {
      const alertAct = firstAction?.type === "alert" ? firstAction : undefined;
      const level = alertAct?.level ?? "info";
      const title = alertAct?.title ?? "";
      const message = alertAct?.message ?? "";

      return (
        <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Alert</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <ConfigCard tone="emerald">
            <Badge tone="emerald">ACTION · ALERT</Badge>
            <FieldGroup label="Mức độ">
              <Segmented
                options={[
                  { value: "info", label: "Info" },
                  { value: "warning", label: "Warning" },
                  { value: "error", label: "Critical" },
                ]}
                value={level}
                onChange={(v) =>
                  setAction({
                    type: "alert",
                    level: v as "info" | "warning" | "error",
                    title,
                    message,
                  })
                }
              />
            </FieldGroup>
            <FieldGroup label="Title (optional)">
              <input
                className="ui-input mt-1"
                value={title}
                onChange={(e) =>
                  setAction({
                    type: "alert",
                    level,
                    title: e.target.value,
                    message,
                  })
                }
              />
            </FieldGroup>
            <FieldGroup label="Message">
              <textarea
                aria-label="Message"
                className="ui-input mt-1 w-full"
                rows={3}
                value={message}
                onChange={(e) =>
                  setAction({
                    type: "alert",
                    level,
                    title,
                    message: e.target.value,
                  })
                }
              />
            </FieldGroup>
            {availableVariables.length > 0 && (
              <ChipsRow>
                {availableVariables.map((v) => (
                  <button
                    key={v}
                    type="button"
                    aria-label={v}
                    className="rounded-full border border-emerald-200 bg-emerald-50 px-2 py-0.5 text-[10px] font-medium text-emerald-800 hover:bg-emerald-100"
                    onClick={() =>
                      setAction({
                        type: "alert",
                        level,
                        title,
                        message: `${message}{{${v}}}`,
                      })
                    }
                  >
                    {v}
                  </button>
                ))}
              </ChipsRow>
            )}
            {(() => {
              const tokens = extractTemplateTokens(message);
              const knownTokens = new Set([...availableVariables, ...BUILTIN_TEMPLATE_TOKENS]);
              const unknownTokens = tokens.filter((t) => !knownTokens.has(t));
              const sample: Record<string, string> = {};
              for (const v of availableVariables) sample[v] = `⟨${v}⟩`;
              for (const t of BUILTIN_TEMPLATE_TOKENS) sample[t] = `⟨${t}⟩`;
              return (
                <div className="rounded bg-emerald-50/70 p-2 text-[11px] text-emerald-900">
                  <span className="font-semibold">Xem trước:</span> {renderTemplatePreview(message, sample)}
                  {unknownTokens.length > 0 && (
                    <p className="mt-1 text-amber-700">
                      Biến chưa xác định: {unknownTokens.join(", ")}
                    </p>
                  )}
                </div>
              );
            })()}
          </ConfigCard>
        </div>
      );
    }

    if (kind === "recipe_override") {
      const isEndSeason = firstAction?.type === "end_season";
      const reason =
        firstAction?.type === "end_season" || firstAction?.type === "advance_stage"
          ? firstAction.reason
          : "";
      const targetStageOffset =
        firstAction?.type === "advance_stage" ? firstAction.targetStageOffset : 1;

      return (
        <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Recipe</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <ConfigCard tone="emerald">
            <Badge tone="emerald">ACTION · RECIPE</Badge>
            <FieldGroup label="Loại hành động">
              <select
                className="ui-input mt-1"
                value={isEndSeason ? "end_season" : "advance_stage"}
                onChange={(e) =>
                  setAction(
                    e.target.value === "end_season"
                      ? { type: "end_season", reason }
                      : { type: "advance_stage", targetStageOffset: 1, reason }
                  )
                }
              >
                <option value="advance_stage">advance_stage</option>
                <option value="end_season">end_season</option>
              </select>
            </FieldGroup>
            {!isEndSeason ? (
              <>
                <FieldGroup label="Target stage offset">
                  <input
                    type="number"
                    className="ui-input mt-1"
                    value={targetStageOffset}
                    onChange={(e) =>
                      setAction({
                        type: "advance_stage",
                        targetStageOffset: Number(e.target.value),
                        reason,
                      })
                    }
                  />
                </FieldGroup>
                <FieldGroup label="Reason">
                  <input
                    className="ui-input mt-1"
                    value={reason}
                    onChange={(e) =>
                      setAction({
                        type: "advance_stage",
                        targetStageOffset,
                        reason: e.target.value,
                      })
                    }
                  />
                </FieldGroup>
              </>
            ) : (
              <FieldGroup label="Reason">
                <input
                  className="ui-input mt-1"
                  value={reason}
                  onChange={(e) =>
                    setAction({
                      type: "end_season",
                      reason: e.target.value,
                    })
                  }
                />
              </FieldGroup>
            )}
          </ConfigCard>
        </div>
      );
    }

    // action_command
    const isWater =
      firstAction?.type === "water_on" || firstAction?.type === "water_off";
    const isDose = firstAction?.type === "dose";
    const actionVal = isDose
      ? "dose"
      : isWater
        ? firstAction.type
        : "emergency_stop";

    const dosePump = firstAction?.type === "dose" ? firstAction.pump : "PUMP_A";
    const doseMl = firstAction?.type === "dose" ? firstAction.doseMl : 1;
    const dosePwm = firstAction?.type === "dose" ? firstAction.pwm : 100;

    const waterPump =
      firstAction?.type === "water_on" || firstAction?.type === "water_off"
        ? firstAction.pump
        : "WATER_PUMP_IN";
    const waterDuration =
      firstAction?.type === "water_on" ? firstAction.durationSec : 10;

    return (
      <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Action — Điều khiển</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
        </div>
        <ConfigCard tone="emerald">
          <Badge tone="emerald">ACTION · {actionVal.toUpperCase()}</Badge>
          <FieldGroup label="Loại hành động">
            <select
              className="ui-input mt-1"
              value={actionVal}
              onChange={(e) => {
                const next = e.target.value;
                if (next === "dose")
                  setAction({
                    type: "dose",
                    pump: "PUMP_A",
                    doseMl: 1,
                    pwm: 100,
                  });
                else if (next === "water_on")
                  setAction({
                    type: "water_on",
                    pump: "WATER_PUMP_IN",
                    durationSec: 10,
                  });
                else if (next === "water_off")
                  setAction({
                    type: "water_off",
                    pump: "WATER_PUMP_IN",
                  });
                else setAction({ type: "emergency_stop" });
              }}
            >
              <option value="dose">dose</option>
              <option value="water_on">water_on</option>
              <option value="water_off">water_off</option>
              <option value="emergency_stop">emergency_stop</option>
            </select>
          </FieldGroup>

          {isDose && (
            <>
              <FieldGroup label="Bơm">
                <select
                  className="ui-input mt-1"
                  value={dosePump}
                  onChange={(e) =>
                    setAction({
                      type: "dose",
                      pump: e.target.value as "PUMP_A" | "PUMP_B" | "PH_UP" | "PH_DOWN",
                      doseMl,
                      pwm: dosePwm,
                    })
                  }
                >
                  <option value="PUMP_A">PUMP_A</option>
                  <option value="PUMP_B">PUMP_B</option>
                  <option value="PH_UP">PH_UP</option>
                  <option value="PH_DOWN">PH_DOWN</option>
                </select>
              </FieldGroup>
              <FieldGroup label="Liều (ml)">
                <input
                  type="number"
                  className="ui-input mt-1"
                  value={doseMl}
                  onChange={(e) =>
                    setAction({
                      type: "dose",
                      pump: dosePump,
                      doseMl: Number(e.target.value),
                      pwm: dosePwm,
                    })
                  }
                />
              </FieldGroup>
              <FieldGroup label="PWM (%)">
                <input
                  type="number"
                  className="ui-input mt-1"
                  value={dosePwm}
                  onChange={(e) =>
                    setAction({
                      type: "dose",
                      pump: dosePump,
                      doseMl,
                      pwm: Number(e.target.value),
                    })
                  }
                />
              </FieldGroup>
            </>
          )}

          {isWater && (
            <>
              <FieldGroup label="Bơm/van">
                <select
                  className="ui-input mt-1"
                  value={waterPump}
                  onChange={(e) => {
                    const pump = e.target.value as "WATER_PUMP_IN" | "WATER_PUMP_OUT" | "MIST_VALVE" | "OSAKA_PUMP";
                    setAction(
                      actionVal === "water_on"
                        ? {
                            type: "water_on",
                            pump,
                            durationSec: waterDuration,
                          }
                        : {
                            type: "water_off",
                            pump,
                          }
                    );
                  }}
                >
                  <option value="WATER_PUMP_IN">WATER_PUMP_IN</option>
                  <option value="WATER_PUMP_OUT">WATER_PUMP_OUT</option>
                  <option value="MIST_VALVE">MIST_VALVE</option>
                  <option value="OSAKA_PUMP">OSAKA_PUMP</option>
                </select>
              </FieldGroup>
              {actionVal === "water_on" && (
                <FieldGroup label="Thời gian (giây)">
                  <input
                    type="number"
                    className="ui-input mt-1"
                    value={waterDuration}
                    onChange={(e) =>
                      setAction({
                        type: "water_on",
                        pump: waterPump,
                        durationSec: Number(e.target.value),
                      })
                    }
                  />
                </FieldGroup>
              )}
            </>
          )}
        </ConfigCard>
      </div>
    );
  }

  if (node.type === "config") {
    const variant = (node.data?.variant as string) === "overwrite" ? "overwrite" : "read";
    if (variant === "read") {
      return (
        <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Config — Đọc</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
          </div>
          <ConfigCard tone="indigo">
            <Badge tone="indigo">CONFIG · ĐỌC (MỚI)</Badge>
            <FieldGroup label="Config key">
              <select
                className="ui-input"
                value={(node.data?.configKey as string) ?? ""}
                onChange={(e) => onChange(node.id, { ...node.data, configKey: e.target.value })}
              >
                <option value="">-- Chọn key --</option>
                {DEVICE_CONFIG_KEYS.map((k) => (
                  <option key={k} value={k}>{k}</option>
                ))}
              </select>
            </FieldGroup>
            <FieldGroup label="Lưu vào biến">
              <input
                type="text"
                className="ui-input"
                placeholder="vd: ph_target_now"
                value={(node.data?.saveToVariable as string) ?? ""}
                onChange={(e) => onChange(node.id, { ...node.data, saveToVariable: e.target.value })}
              />
            </FieldGroup>
            <SafeNote>Chỉ đọc — không thay đổi trạng thái thiết bị</SafeNote>
          </ConfigCard>
        </div>
      );
    }
    return (
      <div className="w-80 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Config — Ghi đè</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>Đóng</button>
        </div>
        <ConfigCard tone="indigo" emphasized>
          <Badge tone="indigo">CONFIG · GHI ĐÈ (MỚI)</Badge>
          <FieldGroup label="Config key">
            <select
              className="ui-input"
              value={(node.data?.configKey as string) ?? ""}
              onChange={(e) => onChange(node.id, { ...node.data, configKey: e.target.value })}
            >
              <option value="">-- Chọn key --</option>
              {DEVICE_CONFIG_KEYS.map((k) => (
                <option key={k} value={k}>{k}</option>
              ))}
            </select>
          </FieldGroup>
          <FieldGroup label="Giá trị ghi đè">
            <VariableCombobox
              id={`cfg-${node.id}-override`}
              ariaLabel="Giá trị ghi đè"
              value={String(node.data?.overrideValue ?? "")}
              onChange={(val) => onChange(node.id, { ...node.data, overrideValue: val })}
              availableVariables={availableVariables}
              placeholder="vd: 1.8 hoặc chọn biến..."
            />
          </FieldGroup>
          <FieldGroup label="Thời điểm áp dụng">
            <select
              className="ui-input"
              value={(node.data?.applyWhen as string) ?? "previous_condition_true"}
              onChange={(e) => onChange(node.id, { ...node.data, applyWhen: e.target.value })}
            >
              <option value="previous_condition_true">Khi điều kiện trước đúng</option>
              <option value="always">Luôn áp dụng</option>
            </select>
          </FieldGroup>
          <ToggleRow
            label="Đọc giá trị gốc trước khi ghi (rollback an toàn)"
            checked={Boolean(node.data?.readOriginalBeforeWrite ?? true)}
            onChange={(v) => onChange(node.id, { ...node.data, readOriginalBeforeWrite: v })}
          />
          <FieldGroup label="Chế độ khôi phục">
            <select
              className="ui-input"
              value={(node.data?.restoreMode as string) ?? "on_flow_exit"}
              onChange={(e) => onChange(node.id, { ...node.data, restoreMode: e.target.value })}
            >
              <option value="manual">Thủ công (không tự khôi phục)</option>
              <option value="on_flow_exit">Khi Flow kết thúc</option>
              <option value="on_condition_false">Khi điều kiện không còn đúng</option>
            </select>
          </FieldGroup>
        </ConfigCard>
      </div>
    );
  }

  return null;
}

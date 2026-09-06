import { useState } from "react";
import type {
  Action,
  AutomationIr,
  ConditionOrGroup,
  ComparisonOperator,
} from "../../../lib/automation/ir";
import {
  fieldsForKind,
  summarizeActions,
} from "../../../hooks/useAutomationBuilder";
import {
  summarizeConditionTree,
} from "../../../lib/automation/conditionTree";

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
  InputWithSuffix,
  InputWithButton,
  PillsSelector,
  DashedTag,
  InspectorShell,
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
  availableFlows?: Array<{ id: string; name: string }>;
  onChange: (nodeId: string, data: Record<string, unknown>) => void;
  onClose: () => void;
  onOpenAuditModal?: () => void;
}

export function NodeEditorPanel({
  kind,
  node,
  nodes,
  edges,
  availableFlows,
  onChange,
  onClose,
  onOpenAuditModal,
}: NodeEditorPanelProps) {
  const fields = fieldsForKind(kind);
  const [triggerTab, setTriggerTab] = useState<"sensor" | "fsm" | "cron" | "webhook">("sensor");
  const availableVariables = getAvailableContextVariables(
    nodes ?? [],
    edges ?? [],
    node.id,
  );

  const [copiedEndpoint, setCopiedEndpoint] = useState(false);

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
      <InspectorShell title="Trigger" onClose={onClose}>
        <div className="mb-3 flex border-b border-emerald-100 text-xs">
          {(["sensor", "fsm", "cron", "webhook"] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              className={`flex-1 py-1 text-center font-medium capitalize cursor-pointer transition-colors ${
                currentKind === tab
                  ? "border-b-2 border-emerald-600 font-semibold text-emerald-900"
                  : "text-emerald-700/60 hover:text-emerald-800"
              }`}
              onClick={() => {
                setTriggerTab(tab);
                onChange(node.id, { ...node.data, kind: tab });
              }}
            >
              {tab}
            </button>
          ))}
        </div>
        <ConfigCard tone="sky">
          <Badge tone="sky">{tabBadge[currentKind]}</Badge>

          {currentKind === "sensor" && (
            <div className="flex flex-col gap-3">
              <h4 className="text-sm font-bold text-slate-900 leading-snug">
                {(node.data.title as string) || "pH (thời gian thực)"}
              </h4>

              <FieldGroup label="Cảm biến nguồn">
                <select
                  aria-label="Cảm biến nguồn"
                  className="ui-input w-full text-xs"
                  value={(node.data.sensorSource as string) || "pH Probe #1 - kênh A0"}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "sensor",
                      sensorSource: e.target.value,
                    })
                  }
                >
                  <option value="pH Probe #1 - kênh A0">pH Probe #1 - kênh A0</option>
                  <option value="EC Sensor #1 - kênh A1">EC Sensor #1 - kênh A1</option>
                  <option value="Nhiệt độ dung dịch - DS18B20">Nhiệt độ dung dịch - DS18B20</option>
                  <option value="Mực nước bồn - Kênh A2">Mực nước bồn - Kênh A2</option>
                </select>
              </FieldGroup>

              <FieldGroup label="Chu kỳ đọc">
                <InputWithSuffix
                  type="number"
                  ariaLabel="Chu kỳ đọc"
                  value={Number(node.data.intervalSec ?? 30)}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "sensor",
                      intervalSec: Number(e.target.value),
                    })
                  }
                  suffix="giây"
                  min={1}
                />
              </FieldGroup>

              <FieldGroup label="Lọc nhiễu tín hiệu">
                <select
                  aria-label="Lọc nhiễu tín hiệu"
                  className="ui-input w-full text-xs"
                  value={(node.data.filterNoise as string) || "Trung bình động - 5 mẫu"}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "sensor",
                      filterNoise: e.target.value,
                    })
                  }
                >
                  <option value="Trung bình động - 5 mẫu">Trung bình động - 5 mẫu</option>
                  <option value="Trung bình động - 10 mẫu">Trung bình động - 10 mẫu</option>
                  <option value="Trung vị (Median)">Trung vị (Median)</option>
                  <option value="Không lọc (Raw)">Không lọc (Raw)</option>
                </select>
              </FieldGroup>

              <ToggleRow
                label="Dùng giá trị gần nhất khi mất kết nối"
                checked={Boolean(node.data.useLastValueOnDisconnect ?? true)}
                onChange={(v) =>
                  onChange(node.id, {
                    ...node.data,
                    kind: "sensor",
                    useLastValueOnDisconnect: v,
                  })
                }
              />
            </div>
          )}

          {currentKind === "fsm" && (
            <div className="flex flex-col gap-3">
              <h4 className="text-sm font-bold text-slate-900 leading-snug">
                {(node.data.title as string) || "Giai đoạn canh tác (FSM)"}
              </h4>

              <FieldGroup label="Máy trạng thái">
                <select
                  aria-label="Máy trạng thái"
                  className="ui-input w-full text-xs"
                  value={(node.data.fsmMachine as string) || "Vòng đời cây trồng"}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "fsm",
                      fsmMachine: e.target.value,
                    })
                  }
                >
                  <option value="Vòng đời cây trồng">Vòng đời cây trồng</option>
                  <option value="Chu trình vệ sinh CIP">Chu trình vệ sinh CIP</option>
                  <option value="Quy trình súc rửa">Quy trình súc rửa</option>
                </select>
              </FieldGroup>

              <FieldGroup label="Kích hoạt khi">
                <select
                  aria-label="Kích hoạt khi"
                  className="ui-input w-full text-xs"
                  value={(node.data.triggerWhen as string) || "Vào giai đoạn mới"}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "fsm",
                      triggerWhen: e.target.value,
                    })
                  }
                >
                  <option value="Vào giai đoạn mới">Vào giai đoạn mới</option>
                  <option value="Rời khỏi giai đoạn">Rời khỏi giai đoạn</option>
                  <option value="Trong suốt giai đoạn">Trong suốt giai đoạn</option>
                </select>
              </FieldGroup>

              <div className="flex flex-col gap-1.5">
                <span className="text-[11px] text-emerald-800/70">Giai đoạn</span>
                <PillsSelector
                  options={[
                    { value: "Cây con", label: "Cây con" },
                    { value: "Sinh trưởng", label: "Sinh trưởng" },
                    { value: "Ra hoa", label: "Ra hoa" },
                    { value: "Thu hoạch", label: "Thu hoạch" },
                  ]}
                  selectedValues={[(node.data.stage as string) || "Ra hoa"]}
                  onToggle={(val) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "fsm",
                      stage: val,
                    })
                  }
                  tone="sky"
                />
              </div>

              <FieldGroup label="Thời lượng tối thiểu trong giai đoạn">
                <InputWithSuffix
                  type="number"
                  ariaLabel="Thời lượng tối thiểu trong giai đoạn"
                  value={Number(node.data.minDurationDays ?? 3)}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "fsm",
                      minDurationDays: Number(e.target.value),
                    })
                  }
                  suffix="ngày"
                  min={0}
                />
              </FieldGroup>
            </div>
          )}

          {currentKind === "cron" && (
            <div className="flex flex-col gap-3">
              <h4 className="text-sm font-bold text-slate-900 leading-snug">
                {(node.data.title as string) || "07:00 mỗi ngày"}
              </h4>

              <FieldGroup label="Lịch dựng sẵn">
                <select
                  aria-label="Lịch dựng sẵn"
                  className="ui-input w-full text-xs"
                  onChange={(e) => {
                    const val = e.target.value;
                    if (val === "daily_7am") {
                      onChange(node.id, {
                        ...node.data,
                        kind: "cron",
                        expression: "0 0 7 * * *",
                        trigger: {
                          type: "cron",
                          cronExpression: "0 0 7 * * *",
                          timezone: (node.data.timezone as string) || "Asia/Ho_Chi_Minh",
                        },
                      });
                    } else if (val === "hourly") {
                      onChange(node.id, {
                        ...node.data,
                        kind: "cron",
                        expression: "0 0 * * * *",
                        trigger: {
                          type: "cron",
                          cronExpression: "0 0 * * * *",
                          timezone: (node.data.timezone as string) || "Asia/Ho_Chi_Minh",
                        },
                      });
                    } else if (val === "weekly_mon") {
                      onChange(node.id, {
                        ...node.data,
                        kind: "cron",
                        expression: "0 0 7 * * 1",
                        trigger: {
                          type: "cron",
                          cronExpression: "0 0 7 * * 1",
                          timezone: (node.data.timezone as string) || "Asia/Ho_Chi_Minh",
                        },
                      });
                    }
                  }}
                  defaultValue=""
                >
                  <option value="" disabled>
                    -- Chọn lịch mẫu --
                  </option>
                  <option value="daily_7am">Mỗi ngày lúc 07:00</option>
                  <option value="hourly">Mỗi giờ (phút thứ 0)</option>
                  <option value="weekly_mon">Mỗi thứ Hai lúc 07:00</option>
                </select>
              </FieldGroup>

              <FieldGroup label="Biểu thức Cron">
                <input
                  type="text"
                  aria-label="Biểu thức Cron"
                  value={(node.data.trigger as any)?.cronExpression || cronExp}
                  onChange={(e) => {
                    const val = e.target.value;
                    onChange(node.id, {
                      ...node.data,
                      kind: "cron",
                      expression: val,
                      trigger: {
                        type: "cron",
                        cronExpression: val,
                        timezone: (node.data.timezone as string) || "Asia/Ho_Chi_Minh",
                      },
                    });
                  }}
                  className="ui-input text-xs font-mono"
                  placeholder="0 0 7 * * *"
                />
              </FieldGroup>

              <FieldGroup label="Múi giờ">
                <select
                  aria-label="Múi giờ"
                  className="ui-input w-full text-xs"
                  value={(node.data.timezone as string) || "Asia/Ho_Chi_Minh"}
                  onChange={(e) => {
                    const tz = e.target.value;
                    onChange(node.id, {
                      ...node.data,
                      kind: "cron",
                      timezone: tz,
                      trigger: {
                        type: "cron",
                        cronExpression: (node.data.trigger as any)?.cronExpression || cronExp,
                        timezone: tz,
                      },
                    });
                  }}
                >
                  <option value="Asia/Ho_Chi_Minh">Asia/Ho_Chi_Minh</option>
                  <option value="UTC">UTC</option>
                </select>
              </FieldGroup>

              <ToggleRow
                label="Chạy bù nếu lỡ lịch (trong 1 giờ)"
                checked={Boolean(node.data.catchUpIfMissed ?? true)}
                onChange={(v) =>
                  onChange(node.id, {
                    ...node.data,
                    kind: "cron",
                    catchUpIfMissed: v,
                  })
                }
              />

              <ToggleRow
                label="Bỏ qua nếu thiết bị offline"
                checked={Boolean(node.data.skipIfOffline ?? false)}
                onChange={(v) =>
                  onChange(node.id, {
                    ...node.data,
                    kind: "cron",
                    skipIfOffline: v,
                  })
                }
              />
            </div>
          )}

          {currentKind === "webhook" && (
            <div className="flex flex-col gap-3">
              <h4 className="text-sm font-bold text-slate-900 leading-snug">
                {(node.data.title as string) || "Nhận dữ liệu từ bên ngoài"}
              </h4>

              <FieldGroup label="Endpoint">
                <InputWithButton
                  ariaLabel="Endpoint"
                  value={(node.data.endpoint as string) || "/hooks/f-2201"}
                  buttonText={copiedEndpoint ? "Đã chép ✓" : "Sao chép"}
                  onButtonClick={() => {
                    const url = (node.data.endpoint as string) || "/hooks/f-2201";
                    navigator.clipboard?.writeText(url);
                    setCopiedEndpoint(true);
                    setTimeout(() => setCopiedEndpoint(false), 2000);
                  }}
                />
              </FieldGroup>

              <FieldGroup label="Chế độ xử lý">
                <select
                  aria-label="Chế độ xử lý"
                  className="ui-input w-full text-xs"
                  value={(node.data.mode as string) || "flow (nối tiếp)"}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "webhook",
                      mode: e.target.value,
                    })
                  }
                >
                  <option value="flow (nối tiếp)">flow (nối tiếp)</option>
                  <option value="direct (trực tiếp)">direct (trực tiếp)</option>
                </select>
              </FieldGroup>

              <FieldGroup label="Xác thực">
                <select
                  aria-label="Xác thực"
                  className="ui-input w-full text-xs"
                  value={(node.data.auth as string) || "Chữ ký HMAC - ****3f2a"}
                  onChange={(e) =>
                    onChange(node.id, {
                      ...node.data,
                      kind: "webhook",
                      auth: e.target.value,
                    })
                  }
                >
                  <option value="Chữ ký HMAC - ****3f2a">Chữ ký HMAC - ****3f2a</option>
                  <option value="Bearer Token">Bearer Token</option>
                  <option value="API Key">API Key</option>
                  <option value="Không xác thực">Không xác thực</option>
                </select>
              </FieldGroup>

              <div className="flex flex-col gap-1.5 pt-1">
                <span className="text-[11px] text-emerald-800/70">Các biến payload</span>
                <div className="flex flex-wrap gap-1.5">
                  {((node.data.payloadFields as string[]) || [
                    "ec_out:flow",
                    "ph_ext:fgh",
                    "temp_out:temp",
                  ]).map((tag, idx) => (
                    <DashedTag
                      key={idx}
                      label={tag}
                      onRemove={() => {
                        const current =
                          (node.data.payloadFields as string[]) || [
                            "ec_out:flow",
                            "ph_ext:fgh",
                            "temp_out:temp",
                          ];
                        onChange(node.id, {
                          ...node.data,
                          kind: "webhook",
                          payloadFields: current.filter((_, i) => i !== idx),
                        });
                      }}
                    />
                  ))}
                </div>
              </div>
            </div>
          )}
        </ConfigCard>
      </InspectorShell>
    );
  }

  if (node.type === "condition" || node.type === "condition_group") {
    const rawConditions = Array.isArray(node.data?.conditions)
      ? (node.data.conditions as ConditionOrGroup[])
      : [];

    const isGroupCondition =
      node.type === "condition_group" ||
      node.data?.isGroup === true ||
      node.data?.type === "condition_group" ||
      (rawConditions.length > 0 && "op" in (rawConditions[0] as any)) ||
      rawConditions.length > 1;

    // 2.2 CONDITION · NHÓM
    if (isGroupCondition) {
      const groupOp = (node.data?.groupOp as "and" | "or") || "and";
      const subConditions =
        rawConditions.length > 0
          ? rawConditions
          : [
              { sensor: "ec", operator: ">" as ComparisonOperator, value: 1.8 },
              { sensor: "temp", operator: "<" as ComparisonOperator, value: 26 },
              { sensor: "giờ", operator: "==" as ComparisonOperator, value: "22:00-05:00" },
            ];

      return (
        <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Điều kiện</h3>
            <button
              type="button"
              className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
              onClick={onClose}
            >
              Đóng
            </button>
          </div>

          <ConfigCard tone="amber">
            <div className="flex items-center gap-1.5">
              <Badge tone="amber">CONDITION</Badge>
              <Badge tone="amber">NHÓM</Badge>
            </div>

            <h4 className="text-sm font-bold text-slate-900 leading-snug">
              {(node.data.title as string) ||
                (groupOp === "and" ? "Tất cả đều đúng" : "Một trong số điều kiện đúng")}
            </h4>

            <FieldGroup label="Toán tử nhóm" as="div">
              <Segmented
                options={[
                  { value: "and", label: "AND — tất cả đúng" },
                  { value: "or", label: "OR — bất kỳ đúng" },
                ]}
                value={groupOp}
                onChange={(op) =>
                  onChange(node.id, {
                    ...node.data,
                    isGroup: true,
                    groupOp: op,
                  })
                }
              />
            </FieldGroup>

            <div className="flex flex-col gap-1.5">
              <span className="text-[11px] text-emerald-800/70">
                {subConditions.length} điều kiện con
              </span>
              <div className="flex flex-wrap items-center gap-1.5">
                {subConditions.map((c: any, idx: number) => {
                  const label =
                    typeof c === "string"
                      ? c
                      : c.sensor === "temp"
                        ? `${c.sensor} < ${c.value}°C`
                        : `${c.sensor} ${c.operator} ${c.value}`;
                  return (
                    <Chip
                      key={idx}
                      tone="amber"
                      onRemove={() => {
                        const next = subConditions.filter((_, i) => i !== idx);
                        onChange(node.id, {
                          ...node.data,
                          isGroup: true,
                          conditions: next,
                          summary: summarizeConditionTree(next as any),
                        });
                      }}
                    >
                      {label}
                    </Chip>
                  );
                })}

                <button
                  type="button"
                  onClick={() => {
                    const next = [
                      ...subConditions,
                      { sensor: fields[0] || "ph", operator: ">" as ComparisonOperator, value: 7.0 },
                    ];
                    onChange(node.id, {
                      ...node.data,
                      isGroup: true,
                      conditions: next,
                      summary: summarizeConditionTree(next as any),
                    });
                  }}
                  className="rounded-full border border-dashed border-amber-300 bg-amber-50/50 px-2.5 py-1 text-[10.5px] font-semibold text-amber-800 hover:bg-amber-100 cursor-pointer transition-colors"
                >
                  + Thêm điều kiện
                </button>
              </div>
            </div>

            <ToggleRow
              label="Đảo ngược kết quả (NOT)"
              checked={Boolean(node.data.invertNot ?? false)}
              onChange={(v) =>
                onChange(node.id, {
                  ...node.data,
                  isGroup: true,
                  invertNot: v,
                })
              }
            />
          </ConfigCard>
        </div>
      );
    }

    // 2.1 CONDITION (Đơn lẻ)
    const firstCond: any = rawConditions[0] || {};
    const sensor = firstCond.sensor || (node.data.sensor as string) || "ph";
    const operator = firstCond.operator || (node.data.operator as string) || ">";
    const value =
      firstCond.value !== undefined
        ? firstCond.value
        : node.data.value !== undefined
          ? node.data.value
          : 7.2;
    const unit = sensor === "ec" ? "mS/cm" : sensor === "temp" ? "°C" : "pH";

    const updateSingleCondition = (updates: Partial<{ sensor: string; operator: ComparisonOperator; value: number }>) => {
      const nextSensor = updates.sensor ?? sensor;
      const nextOp = (updates.operator ?? operator) as ComparisonOperator;
      const nextVal = updates.value ?? value;
      const newConditions = [{ sensor: nextSensor, operator: nextOp, value: nextVal }];
      onChange(node.id, {
        ...node.data,
        conditions: newConditions,
        summary: `${nextSensor} ${nextOp} ${nextVal}`,
      });
    };

    return (
      <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Điều kiện</h3>
          <button
            type="button"
            className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
            onClick={onClose}
          >
            Đóng
          </button>
        </div>

        <ConfigCard tone="amber">
          <Badge tone="amber">CONDITION</Badge>

          <h4 className="text-sm font-bold text-slate-900 leading-snug">
            {(node.data.title as string) || "pH vượt ngưỡng an toàn"}
          </h4>

          <FieldGroup label="Biến so sánh">
            <select
              aria-label="Biến so sánh"
              className="ui-input w-full text-xs"
              value={sensor}
              onChange={(e) => updateSingleCondition({ sensor: e.target.value })}
            >
              <option value="ph">pH (tức thời)</option>
              <option value="ec">EC (tức thời)</option>
              <option value="temp">Nhiệt độ (tức thời)</option>
              <option value="water_level">Mực nước (tức thời)</option>
            </select>
          </FieldGroup>

          <FieldGroup label="Toán tử">
            <select
              aria-label="Toán tử"
              className="ui-input w-full text-xs"
              value={operator}
              onChange={(e) =>
                updateSingleCondition({ operator: e.target.value as ComparisonOperator })
              }
            >
              <option value=">">&gt; lớn hơn</option>
              <option value="<">&lt; nhỏ hơn</option>
              <option value=">=">&gt;= lớn hơn hoặc bằng</option>
              <option value="<=">&lt;= nhỏ hơn hoặc bằng</option>
              <option value="==">== bằng</option>
              <option value="!=">!= khác</option>
            </select>
          </FieldGroup>

          <FieldGroup label="Giá trị ngưỡng">
            <InputWithSuffix
              type="number"
              ariaLabel="Giá trị"
              step={0.1}
              value={value}
              onChange={(e) => updateSingleCondition({ value: Number(e.target.value) })}
              suffix={unit}
            />
          </FieldGroup>

          <FieldGroup label="Áp dụng trong">
            <select
              aria-label="Áp dụng trong"
              className="ui-input w-full text-xs"
              value={(node.data.applyWindow as string) || "Luôn luôn"}
              onChange={(e) =>
                onChange(node.id, {
                  ...node.data,
                  applyWindow: e.target.value,
                })
              }
            >
              <option value="Luôn luôn">Luôn luôn</option>
              <option value="Ban ngày (06:00 - 18:00)">Ban ngày (06:00 - 18:00)</option>
              <option value="Ban đêm (18:00 - 06:00)">Ban đêm (18:00 - 06:00)</option>
            </select>
          </FieldGroup>

          <ToggleRow
            label="Chống nhiễu: yêu cầu đúng liên tục = 3 chu kỳ"
            checked={Boolean(node.data.debounceContinuous ?? true)}
            onChange={(v) =>
              onChange(node.id, {
                ...node.data,
                debounceContinuous: v,
              })
            }
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

    // 3.3 ACTION · CHAIN
    if ((node.data as any)?.type === "chain" || (firstAction as any)?.type === "chain") {
      return (
        <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Hành động — Kích hoạt Flow khác</h3>
            <button
              type="button"
              className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
              onClick={onClose}
            >
              Đóng
            </button>
          </div>

          <ConfigCard tone="emerald">
            <Badge tone="emerald">ACTION · CHAIN</Badge>

            <h4 className="text-sm font-bold text-slate-900 leading-snug">
              {(node.data.title as string) || "Chạy tiếp Flow khác"}
            </h4>

            <FieldGroup label="Flow tiếp theo">
              <select
                aria-label="Flow tiếp theo"
                className="ui-input w-full text-xs"
                value={(node.data.targetFlow as string) || "Kiểm tra lại sau 10 phút"}
                onChange={(e) =>
                  onChange(node.id, {
                    ...node.data,
                    type: "chain",
                    targetFlow: e.target.value,
                  })
                }
              >
                <option value="Kiểm tra lại sau 10 phút">Kiểm tra lại sau 10 phút</option>
                <option value="Cân bằng pH khẩn cấp">Cân bằng pH khẩn cấp</option>
                <option value="Tưới xả nước định kỳ">Tưới xả nước định kỳ</option>
                {availableFlows?.map((f) => (
                  <option key={f.id} value={f.name}>
                    {f.name}
                  </option>
                ))}
              </select>
            </FieldGroup>

            <FieldGroup label="Độ trễ trước khi chạy">
              <InputWithSuffix
                type="number"
                ariaLabel="Độ trễ trước khi chạy"
                value={Number(node.data.delayMin ?? 10)}
                onChange={(e) =>
                  onChange(node.id, {
                    ...node.data,
                    type: "chain",
                    delayMin: Number(e.target.value),
                  })
                }
                suffix="phút"
                min={0}
              />
            </FieldGroup>

            <ToggleRow
              label="Truyền biến ngữ cảnh sang flow tiếp theo"
              checked={Boolean(node.data.passContext ?? true)}
              onChange={(v) =>
                onChange(node.id, {
                  ...node.data,
                  type: "chain",
                  passContext: v,
                })
              }
            />

            <FieldGroup label="Giới hạn số lần lặp">
              <InputWithSuffix
                type="number"
                ariaLabel="Giới hạn số lần lặp"
                value={Number(node.data.iterationLimit ?? 5)}
                onChange={(e) =>
                  onChange(node.id, {
                    ...node.data,
                    type: "chain",
                    iterationLimit: Number(e.target.value),
                  })
                }
                suffix="lần tối đa"
                min={1}
              />
            </FieldGroup>
          </ConfigCard>
        </div>
      );
    }

    // 3.2 ACTION · DOSE/WATER
    if (
      kind === "action_command" ||
      (node.data as any)?.type === "control" ||
      (node.data as any)?.type === "dose" ||
      firstAction?.type === "dose" ||
      firstAction?.type === "water_on" ||
      firstAction?.type === "water_off"
    ) {
      const isDose = firstAction?.type === "dose" || !firstAction;
      const dosePump = isDose && firstAction?.type === "dose" ? firstAction.pump : (node.data.pump as any) || "PUMP_A";
      const doseMl = isDose && firstAction?.type === "dose" ? firstAction.doseMl : Number(node.data.volumeMl ?? 12);
      const dosePwm = isDose && firstAction?.type === "dose" ? firstAction.pwm : Number(node.data.pwm ?? 60);

      return (
        <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Điều khiển</h3>
            <button
              type="button"
              className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
              onClick={onClose}
            >
              Đóng
            </button>
          </div>

          <ConfigCard tone="emerald">
            <Badge tone="emerald">ACTION · DOSE/WATER</Badge>

            <h4 className="text-sm font-bold text-slate-900 leading-snug">
              {(node.data.title as string) || "Định lượng dinh dưỡng A"}
            </h4>

            <FieldGroup label="Bơm">
              <select
                aria-label="Bơm"
                className="ui-input mt-1 text-xs"
                value={
                  (node.data.pumpDisplay as string) ||
                  (dosePump === "PUMP_B"
                    ? "PUMP_B - Dinh dưỡng B"
                    : dosePump === "PH_UP"
                      ? "PH_UP - Tăng pH"
                      : dosePump === "PH_DOWN"
                        ? "PH_DOWN - Giảm pH"
                        : "PUMP_A - Dinh dưỡng A")
                }
                onChange={(e) => {
                  const val = e.target.value;
                  let mappedPump: "PUMP_A" | "PUMP_B" | "PH_UP" | "PH_DOWN" = "PUMP_A";
                  if (val.includes("PUMP_B")) mappedPump = "PUMP_B";
                  else if (val.includes("PH_UP")) mappedPump = "PH_UP";
                  else if (val.includes("PH_DOWN")) mappedPump = "PH_DOWN";

                  onChange(node.id, {
                    ...node.data,
                    pump: mappedPump,
                    pumpDisplay: val,
                    actions: [{ type: "dose", pump: mappedPump, doseMl, pwm: dosePwm }],
                    summary: `dose ${doseMl}ml (${mappedPump})`,
                  });
                }}
              >
                <option value="PUMP_A - Dinh dưỡng A">PUMP_A - Dinh dưỡng A</option>
                <option value="PUMP_B - Dinh dưỡng B">PUMP_B - Dinh dưỡng B</option>
                <option value="PH_UP - Tăng pH">PH_UP - Tăng pH</option>
                <option value="PH_DOWN - Giảm pH">PH_DOWN - Giảm pH</option>
                <option value="WATER_PUMP_IN - Bơm cấp nước">WATER_PUMP_IN - Bơm cấp nước</option>
                <option value="WATER_PUMP_OUT - Bơm xả">WATER_PUMP_OUT - Bơm xả</option>
              </select>
            </FieldGroup>

            <FieldGroup label="Thể tích">
              <InputWithSuffix
                type="number"
                ariaLabel="Liều (ml)"
                value={doseMl}
                onChange={(e) => {
                  const ml = Number(e.target.value);
                  const mappedPump = (node.data.pump as any) || dosePump || "PUMP_A";
                  onChange(node.id, {
                    ...node.data,
                    actions: [{ type: "dose", pump: mappedPump, doseMl: ml, pwm: dosePwm }],
                    summary: `dose ${ml}ml (${mappedPump})`,
                  });
                }}
                suffix="ml"
                min={1}
              />
            </FieldGroup>

            <FieldGroup label="Công suất PWM">
              <InputWithSuffix
                type="number"
                ariaLabel="Công suất PWM"
                value={dosePwm}
                onChange={(e) => {
                  const pwm = Number(e.target.value);
                  const mappedPump = (node.data.pump as any) || dosePump || "PUMP_A";
                  onChange(node.id, {
                    ...node.data,
                    actions: [{ type: "dose", pump: mappedPump, doseMl: doseMl, pwm }],
                    summary: `dose ${doseMl}ml (${mappedPump})`,
                  });
                }}
                suffix="%"
                min={1}
                max={100}
              />
            </FieldGroup>

            <ToggleRow
              label="Giới hạn an toàn: tối đa 3 lần / giờ"
              checked={Boolean(node.data.safetyLimit ?? true)}
              onChange={(v) =>
                onChange(node.id, {
                  ...node.data,
                  safetyLimit: v,
                })
              }
            />
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
        <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Recipe</h3>
            <button
              type="button"
              className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
              onClick={onClose}
            >
              Đóng
            </button>
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

    // 3.1 ACTION · ALERT (default)
    const alertAct = firstAction?.type === "alert" ? firstAction : undefined;
    const level = alertAct?.level ?? (node.data?.level as any) ?? "warning";
    const title = alertAct?.title ?? (node.data?.title as string) ?? "";
    const message = alertAct?.message ?? (node.data?.message as string) ?? "EC vượt ngưỡng điểm: {ec} mS/cm lúc {time}";

      return (
        <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Alert</h3>
            <button
              type="button"
              className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
              onClick={onClose}
            >
              Đóng
            </button>
          </div>

          <ConfigCard tone="emerald">
            <Badge tone="emerald">ACTION · ALERT</Badge>

            <h4 className="text-sm font-bold text-slate-900 leading-snug">
              {(node.data.heading as string) || "Gửi cảnh báo mức warning"}
            </h4>

            <FieldGroup label="Mức độ" as="div">
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

            <div className="flex flex-col gap-1.5">
              <span className="text-[11px] text-emerald-800/70">Kênh thông báo</span>
              <PillsSelector
                options={[
                  { value: "fcm", label: "FCM" },
                  { value: "email", label: "Email" },
                  { value: "webhook", label: "Webhook" },
                ]}
                selectedValues={(node.data.channels as string[]) || ["fcm"]}
                onToggle={(val) => {
                  const current = (node.data.channels as string[]) || ["fcm"];
                  const next = current.includes(val)
                    ? current.filter((c) => c !== val)
                    : [...current, val];
                  onChange(node.id, { ...node.data, channels: next });
                }}
                tone="emerald"
              />
            </div>

            <FieldGroup label="Nội dung thông báo">
              <textarea
                aria-label="Message"
                className="ui-input mt-1 w-full text-xs font-mono"
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
                    className="rounded-full border border-emerald-200 bg-emerald-50 px-2 py-0.5 text-[10px] font-medium text-emerald-800 hover:bg-emerald-100 cursor-pointer"
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

            <FieldGroup label="Chống trùng lặp">
              <InputWithSuffix
                type="number"
                ariaLabel="Chống trùng lặp"
                value={Number(node.data.cooldownMin ?? 30)}
                onChange={(e) =>
                  onChange(node.id, {
                    ...node.data,
                    cooldownMin: Number(e.target.value),
                  })
                }
                suffix="phút - không gửi lại"
                min={0}
              />
            </FieldGroup>
          </ConfigCard>
        </div>
      );
  }

  // ==========================================
  // 4. CONFIG NODE (2 Kiểu)
  // ==========================================
  if (node.type === "config") {
    const variant = (node.data?.variant as string) === "overwrite" ? "overwrite" : "read";

    // 4.1 CONFIG · ĐỌC (MỚI)
    if (variant === "read") {
      return (
        <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Config — Đọc</h3>
            <button
              type="button"
              className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
              onClick={onClose}
            >
              Đóng
            </button>
          </div>

          <ConfigCard tone="indigo">
            <Badge tone="indigo">CONFIG · ĐỌC (MỚI)</Badge>

            <h4 className="text-sm font-bold text-slate-900 leading-snug">
              {(node.data.title as string) || "Đọc ph_target hiện tại"}
            </h4>

            <FieldGroup label="Config key">
              <input
                type="text"
                aria-label="Config key"
                className="ui-input text-xs"
                placeholder="ph_target"
                value={(node.data?.configKey as string) ?? ""}
                onChange={(e) =>
                  onChange(node.id, { ...node.data, variant: "read", configKey: e.target.value })
                }
              />
            </FieldGroup>

            <FieldGroup label="Thiết bị / nhóm">
              <select
                aria-label="Thiết bị / nhóm"
                className="ui-input text-xs"
                value={(node.data?.deviceGroup as string) ?? "Zone A - Bơm dinh dưỡng"}
                onChange={(e) =>
                  onChange(node.id, { ...node.data, variant: "read", deviceGroup: e.target.value })
                }
              >
                <option value="Zone A - Bơm dinh dưỡng">Zone A - Bơm dinh dưỡng</option>
                <option value="Zone B - Hệ thống tưới">Zone B - Hệ thống tưới</option>
                <option value="Toàn hệ thống">Toàn hệ thống</option>
              </select>
            </FieldGroup>

            <FieldGroup label="Lưu vào biến">
              <input
                type="text"
                aria-label="Lưu vào biến"
                className="ui-input text-xs"
                placeholder="vd: ph_target_now"
                value={(node.data?.saveToVariable as string) ?? ""}
                onChange={(e) =>
                  onChange(node.id, {
                    ...node.data,
                    variant: "read",
                    saveToVariable: e.target.value,
                  })
                }
              />
            </FieldGroup>

            <SafeNote>Chỉ đọc — không thay đổi trạng thái thiết bị</SafeNote>
          </ConfigCard>
        </div>
      );
    }

    // 4.2 CONFIG · GHI ĐÈ (MỚI)
    return (
      <div className="w-96 shrink-0 overflow-y-auto border-l border-emerald-100 bg-white p-3.5 shadow-sm">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Config — Ghi đè</h3>
          <button
            type="button"
            className="text-xs font-medium text-emerald-700/70 hover:text-emerald-900 cursor-pointer"
            onClick={onClose}
          >
            Đóng
          </button>
        </div>

        <ConfigCard tone="indigo" emphasized>
          <Badge tone="indigo">CONFIG · GHI ĐÈ (MỚI)</Badge>

          <h4 className="text-sm font-bold text-slate-900 leading-snug">
            {(node.data.title as string) || "Ghi đè ec_target → 1.8"}
          </h4>

          <FieldGroup label="Config key">
            <input
              type="text"
              aria-label="Config key"
              className="ui-input text-xs"
              placeholder="ec_target"
              value={(node.data?.configKey as string) ?? ""}
              onChange={(e) =>
                onChange(node.id, { ...node.data, variant: "overwrite", configKey: e.target.value })
              }
            />
          </FieldGroup>

          <FieldGroup label="Giá trị ghi đè">
            <div className="relative flex items-center">
              <VariableCombobox
                id={`cfg-${node.id}-override`}
                ariaLabel="Giá trị ghi đè"
                value={String(node.data?.overrideValue ?? "")}
                onChange={(val) =>
                  onChange(node.id, { ...node.data, variant: "overwrite", overrideValue: val })
                }
                availableVariables={availableVariables}
                placeholder="vd: 1.8 hoặc chọn biến..."
              />
              <span className="pointer-events-none absolute right-2.5 text-xs text-slate-400 select-none font-medium">
                mS/cm
              </span>
            </div>
          </FieldGroup>

          <FieldGroup label="Thời điểm áp dụng">
            <select
              aria-label="Thời điểm áp dụng"
              className="ui-input text-xs"
              value={(node.data?.applyWhen as string) ?? "previous_condition_true"}
              onChange={(e) =>
                onChange(node.id, { ...node.data, variant: "overwrite", applyWhen: e.target.value })
              }
            >
              <option value="previous_condition_true">Khi điều kiện trước đúng</option>
              <option value="always">Luôn áp dụng</option>
            </select>
          </FieldGroup>

          <ToggleRow
            label="Đọc giá trị gốc trước khi ghi (rollback an toàn)"
            checked={Boolean(node.data?.readOriginalBeforeWrite ?? true)}
            onChange={(v) =>
              onChange(node.id, {
                ...node.data,
                variant: "overwrite",
                readOriginalBeforeWrite: v,
              })
            }
          />

          <FieldGroup label="Chế độ khôi phục">
            <select
              aria-label="Chế độ khôi phục"
              className="ui-input text-xs"
              value={(node.data?.restoreMode as string) ?? "on_flow_exit"}
              onChange={(e) =>
                onChange(node.id, { ...node.data, variant: "overwrite", restoreMode: e.target.value })
              }
            >
              <option value="manual">Thủ công (không tự khôi phục)</option>
              <option value="on_flow_exit">Khi Flow kết thúc</option>
              <option value="on_condition_false">Khi điều kiện không còn đúng</option>
            </select>
          </FieldGroup>

          {onOpenAuditModal && (
            <button
              type="button"
              onClick={onOpenAuditModal}
              className="w-full mt-3 py-1.5 px-3 rounded-xl border border-indigo-200 bg-indigo-50/80 text-indigo-900 text-xs font-semibold hover:bg-indigo-100 transition-colors flex items-center justify-center gap-1.5 cursor-pointer shadow-2xs"
            >
              Mở chi tiết an toàn & Audit Log →
            </button>
          )}
        </ConfigCard>
      </div>
    );
  }

  return null;
}


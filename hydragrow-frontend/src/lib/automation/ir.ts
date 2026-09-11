import { z } from 'zod';

// Mirrors ScriptSensorInput in hydragrow-backend/src/models/script.rs
export const SENSOR_FIELDS = ['ph', 'ec', 'temp', 'water_level'] as const;
// Mirrors ScriptFsmInput fields available to recipe_override scripts
export const FSM_FIELDS = ['ph', 'ec', 'stage_index', 'elapsed_sec'] as const;

export const ComparisonOperatorSchema = z.enum(['>', '<', '>=', '<=', '==', '!=']);
export type ComparisonOperator = z.infer<typeof ComparisonOperatorSchema>;

export const RangeModeSchema = z.enum(['instant', 'mean', 'min', 'max']);
export type RangeMode = z.infer<typeof RangeModeSchema>;

export const ConditionSchema = z
  .object({
    sensor: z.string().min(1),
    operator: ComparisonOperatorSchema,
    value: z.number(),
    /** 'instant' (mặc định, hành vi cũ) = so sánh giá trị tức thời.
     * 'mean'/'min'/'max' = gọi fetch_range_stat(sensor, mode, windowSec). */
    mode: RangeModeSchema.default('instant'),
    /** Bắt buộc khi mode != 'instant'. Đơn vị giây. */
    windowSec: z.number().int().positive().optional(),
    /** Khi có mặt, trình biên dịch (subsystem "compiler + runtime context")
     * so sánh với giá trị của biến ngữ cảnh này thay vì literal `value`.
     * `value` vẫn bắt buộc để giữ tương thích ngược — UI có thể để nguyên 0
     * khi dùng valueVariable, xem VariableCombobox. */
    valueVariable: z.string().min(1).optional(),
  })
  .refine((c) => c.mode === 'instant' || c.windowSec !== undefined, {
    message: 'windowSec bắt buộc khi mode là mean/min/max',
    path: ['windowSec'],
  });

export interface Condition {
  sensor: string;
  operator: ComparisonOperator;
  value: number;
  mode?: RangeMode;
  windowSec?: number;
  valueVariable?: string;
}

export interface ConditionGroup {
  op: 'and' | 'or';
  children: ConditionOrGroup[];
}
export type ConditionOrGroup = Condition | ConditionGroup;

export const ConditionOrGroupSchema: z.ZodType<ConditionOrGroup> = z.lazy(() =>
  z.union([ConditionSchema, ConditionGroupSchema]),
);
export const ConditionGroupSchema: z.ZodType<ConditionGroup> = z.lazy(() =>
  z.object({
    op: z.enum(['and', 'or']),
    children: z.array(ConditionOrGroupSchema).min(1),
  }),
);

export const AlertActionSchema = z.object({
  type: z.literal('alert'),
  level: z.enum(['info', 'warning', 'error']),
  title: z.string().optional(),
  message: z.string().min(1),
  /** Ghi đè tường minh việc có gửi FCM hay không, bất kể level. `undefined` =
   * ir_json cũ trước tính năng này — backend fallback theo level (xem
   * script_eval::should_notify_fcm). */
  notifyFcm: z.boolean().optional(),
});

export const StageOverrideActionSchema = z.object({
  type: z.literal('advance_stage'),
  targetStageOffset: z.number().int(), // relative to current stage_index, e.g. +1
  reason: z.string().min(1),
});

export const EndSeasonActionSchema = z.object({
  type: z.literal('end_season'),
  reason: z.string(),
});

export const DosingPumpSchema = z.enum(['PUMP_A', 'PUMP_B', 'PH_UP', 'PH_DOWN']);
export const WaterPumpSchema = z.enum(['WATER_PUMP_IN', 'WATER_PUMP_OUT', 'MIST_VALVE', 'OSAKA_PUMP']);

export const DoseActionSchema = z.object({
  type: z.literal('dose'),
  pump: DosingPumpSchema,
  doseMl: z.number().positive(),
  pwm: z.number().int().min(1).max(100),
});

export const WaterOnActionSchema = z.object({
  type: z.literal('water_on'),
  pump: WaterPumpSchema,
  durationSec: z.number().int().positive(),
});

export const WaterOffActionSchema = z.object({
  type: z.literal('water_off'),
  pump: WaterPumpSchema,
});

export const EmergencyStopActionSchema = z.object({
  type: z.literal('emergency_stop'),
});

export const ConfigOverrideActionSchema = z.object({
  type: z.literal('config_override'),
  key: z.string().min(1),
  value: z.number(),
  restoreOnExit: z.boolean().default(true),
  priority: z.number().int().default(0),
});

export const ActionSchema = z.discriminatedUnion('type', [
  AlertActionSchema,
  StageOverrideActionSchema,
  EndSeasonActionSchema,
  DoseActionSchema,
  WaterOnActionSchema,
  WaterOffActionSchema,
  EmergencyStopActionSchema,
  ConfigOverrideActionSchema,
]);
export type Action = z.infer<typeof ActionSchema>;

export const WebhookFieldMappingSchema = z.object({
  bodyPath: z.string().min(1),
  targetField: z.string().min(1),
});
export type WebhookFieldMapping = z.infer<typeof WebhookFieldMappingSchema>;

export const WebhookTriggerConfigSchema = z.object({
  type: z.literal('webhook'),
  mode: z.enum(['flow', 'direct']).default('flow'),
  fieldMappings: z.array(WebhookFieldMappingSchema).default([]),
});
export type WebhookTriggerConfig = z.infer<typeof WebhookTriggerConfigSchema>;

export const CronTriggerConfigSchema = z.object({
  type: z.literal('cron'),
  cronExpression: z.string().min(1), // "0 0 7 * * *" — 6 field, giây ở đầu, khớp crate `cron` backend
  timezone: z.string().default('Asia/Ho_Chi_Minh'),
});
export type CronTriggerConfig = z.infer<typeof CronTriggerConfigSchema>;

export const TriggerSchema = z.discriminatedUnion('type', [
  z.object({ type: z.enum(['sensor', 'fsm']) }),
  WebhookTriggerConfigSchema,
  CronTriggerConfigSchema,
]);

// React Flow canvas state — opaque to the compiler, used only to restore the UI.
export const AutomationNodeSchema = z.object({
  id: z.string(),
  type: z.enum(['trigger', 'sensor', 'condition', 'delay', 'action', 'config']),
  position: z.object({ x: z.number(), y: z.number() }),
  data: z.record(z.string(), z.unknown()),
});
export const AutomationEdgeSchema = z.object({
  id: z.string(),
  source: z.string(),
  target: z.string(),
});

export const AutomationKindSchema = z.enum(['alert', 'recipe_override', 'action_command']);

export const ContextReadSchema = z.object({
  configKey: z.string().min(1),
  saveToVariable: z.string().min(1),
});
export type ContextRead = z.infer<typeof ContextReadSchema>;

export const ConfigOverwriteSchema = z.object({
  configKey: z.string().min(1),
  /** Literal ("1.8", "true") hoặc tên 1 context variable — phân giải ở
   * backend, xem hydragrow-backend/src/services/config_override.rs::write_field. */
  value: z.string().min(1),
  readOriginalBeforeWrite: z.boolean().default(false),
  restoreMode: z.literal('on_condition_false').default('on_condition_false'),
  priority: z.number().int().default(0),
});
export type ConfigOverwrite = z.infer<typeof ConfigOverwriteSchema>;

/** Cấu hình cho toàn bộ Flow chain (next_flow_ids) của IR này — không phải
 * per-link, xem "Known scope boundary" trong plan triển khai frontend. */
export const ChainConfigSchema = z.object({
  /** Khi true, flow con nhận BẢN SAO context hiện tại (không phải reference)
   * — thực thi copy-on-call nằm ở backend (subsystem "Chain context-copy +
   * per-node iteration limit"), field này chỉ lưu ý định người dùng chọn trên UI. */
  passContextVariables: z.boolean().default(false),
  /** Số hop tối đa Chain được phép đi qua trước khi engine dừng — giới hạn cứng
   * MAX_CHAIN_DEPTH ở backend luôn áp dụng thêm, giá trị này không thể vượt qua nó. */
  iterationLimit: z.number().int().positive().default(5),
});
export type ChainConfig = z.infer<typeof ChainConfigSchema>;

export const AutomationIrSchema = z
  .object({
    kind: AutomationKindSchema,
    trigger: TriggerSchema,
    conditions: z.array(ConditionOrGroupSchema).min(1),
    actions: z.array(ActionSchema).min(1),
    nodes: z.array(AutomationNodeSchema),
    edges: z.array(AutomationEdgeSchema),
    /** IDs của các Flow sẽ được kích hoạt kế tiếp sau khi Flow này thực thi thành công.
     * Vắng hoặc `[]` = Flow độc lập (hành vi cũ, backward-compat). */
    next_flow_ids: z.array(z.string()).default([]),
    chainConfig: ChainConfigSchema.default({ passContextVariables: false, iterationLimit: 5 }),
    contextReads: z.array(ContextReadSchema).default([]),
    configOverwrite: ConfigOverwriteSchema.optional(),
  })
  .refine(
    (ir) => {
      if (ir.kind === 'alert') return ir.actions.every((a) => a.type === 'alert');
      if (ir.kind === 'recipe_override') {
        return ir.actions.every((a) => a.type === 'advance_stage' || a.type === 'end_season');
      }
      return ir.actions.every((a) => ['dose', 'water_on', 'water_off', 'emergency_stop'].includes(a.type));
    },
    { message: 'actions must match kind' },
  );

export type AutomationIr = z.infer<typeof AutomationIrSchema>;

/** Dạng IR cũ (loại `config_override`, action-based) chỉ còn tồn tại trong
 * dữ liệu Flow lưu từ trước bản redesign canvas. Không parse được bằng
 * AutomationIrSchema nữa — phải qua normalizeLegacyIr trước. */
export interface LegacyAutomationIr {
  kind: 'config_override';
  trigger?: unknown;
  conditions?: unknown;
  actions?: unknown;
  nodes?: unknown;
  edges?: unknown;
  next_flow_ids?: unknown;
  chainConfig?: unknown;
  contextReads?: unknown;
  configOverwrite?: unknown;
}

/**
 * F5: di chuyển IR `config_override` cũ sang dạng mới hợp lệ (kind 'alert' +
 * block `configOverwrite` riêng). Với IR không phải legacy trả về null — caller
 * dùng nguyên giá trị. Trigger cron/fsm/webhook/sensor hợp lệ được giữ nguyên
 * để lịch chạy không mất; trigger 'manual' kiểu cũ bị ép về sensor. Block
 * configOverwrite có sẵn được ưu tiên; nếu thiếu thì suy ra từ action
 * config_override đầu tiên (restoreOnExit=false ⇒ không phục hồi giá trị gốc).
 */
export function normalizeLegacyIr(ir: unknown): AutomationIr | null {
  if (typeof ir !== 'object' || ir === null) return null;
  const candidate = ir as Record<string, unknown>;
  if (candidate.kind !== 'config_override') return null;

  const rawActions = Array.isArray(candidate.actions) ? candidate.actions : [];
  const legacyAction = rawActions.find(
    (a) => (a as { type?: unknown })?.type === 'config_override',
  ) as { key?: string; value?: number; restoreOnExit?: boolean; priority?: number } | undefined;

  const existingOverwrite = candidate.configOverwrite as
    | { configKey?: string; value?: string; readOriginalBeforeWrite?: boolean; restoreMode?: string; priority?: number }
    | undefined;

  const configOverwrite = existingOverwrite?.configKey
    ? existingOverwrite
    : legacyAction?.key
      ? {
          configKey: legacyAction.key,
          value: String(legacyAction.value ?? ''),
          readOriginalBeforeWrite: legacyAction.restoreOnExit ?? true,
          restoreMode: 'on_condition_false' as const,
          priority: legacyAction.priority ?? 0,
        }
      : undefined;

  const nonLegacyActions = rawActions.filter(
    (a) => (a as { type?: unknown })?.type !== 'config_override',
  );
  const actions = (nonLegacyActions.length > 0
    ? nonLegacyActions
    : configOverwrite
      ? [
          {
            type: 'alert',
            level: 'info',
            message: `Đã ghi đè ${configOverwrite.configKey} → ${configOverwrite.value}`,
          },
        ]
      : [
          {
            type: 'alert',
            level: 'info',
            message: 'Config Override',
          },
        ]) as AutomationIr['actions'];

  const rawTrigger = candidate.trigger as { type?: unknown } | undefined;
  const triggerType = rawTrigger?.type;
  const isKnownTrigger =
    triggerType === 'sensor' || triggerType === 'fsm' || triggerType === 'cron' || triggerType === 'webhook';
  const trigger = (isKnownTrigger ? candidate.trigger : { type: 'sensor' }) as AutomationIr['trigger'];

  return {
    kind: 'alert',
    trigger,
    conditions: Array.isArray(candidate.conditions) ? (candidate.conditions as AutomationIr['conditions']) : [],
    actions,
    nodes: Array.isArray(candidate.nodes) ? (candidate.nodes as AutomationIr['nodes']) : [],
    edges: Array.isArray(candidate.edges) ? (candidate.edges as AutomationIr['edges']) : [],
    next_flow_ids: Array.isArray(candidate.next_flow_ids) ? (candidate.next_flow_ids as string[]) : [],
    chainConfig: (candidate.chainConfig as AutomationIr['chainConfig']) ?? { passContextVariables: false, iterationLimit: 5 },
    contextReads: Array.isArray(candidate.contextReads) ? (candidate.contextReads as AutomationIr['contextReads']) : [],
    configOverwrite: configOverwrite as AutomationIr['configOverwrite'],
  } as AutomationIr;
}

import deviceConfigKeys from './device-config-keys.json';

export interface DeviceConfigKeyDef {
  key: string;
  valueType: string;
  min: number;
  max: number;
  unit: string;
  step: number;
  label: string;
  sourceGroup: string;
  defaultVal: number;
}

export interface DeviceConfigBound {
  min: number;
  max: number;
  unit: string;
  step: number;
  label: string;
  sourceGroup: string;
  defaultVal: number;
}

export const DEVICE_CONFIG_BOUNDS: Record<string, DeviceConfigBound> = Object.fromEntries(
  (deviceConfigKeys as DeviceConfigKeyDef[]).map((d) => [
    d.key,
    { min: d.min, max: d.max, unit: d.unit, step: d.step, label: d.label, sourceGroup: d.sourceGroup, defaultVal: d.defaultVal },
  ]),
);

export function clampConfigValue(key: string, val: number): { value: number; clamped: boolean; bound?: DeviceConfigBound } {
  const bound = DEVICE_CONFIG_BOUNDS[key];
  if (!bound) return { value: val, clamped: false };
  if (val < bound.min) return { value: bound.min, clamped: true, bound };
  if (val > bound.max) return { value: bound.max, clamped: true, bound };
  return { value: val, clamped: false, bound };
}


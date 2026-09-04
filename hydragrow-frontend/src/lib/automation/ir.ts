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

export const ActionSchema = z.discriminatedUnion('type', [
  AlertActionSchema,
  StageOverrideActionSchema,
  EndSeasonActionSchema,
  DoseActionSchema,
  WaterOnActionSchema,
  WaterOffActionSchema,
  EmergencyStopActionSchema,
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
  type: z.enum(['trigger', 'sensor', 'condition', 'delay', 'action']),
  position: z.object({ x: z.number(), y: z.number() }),
  data: z.record(z.string(), z.unknown()),
});
export const AutomationEdgeSchema = z.object({
  id: z.string(),
  source: z.string(),
  target: z.string(),
});

export const AutomationKindSchema = z.enum(['alert', 'recipe_override', 'action_command']);

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

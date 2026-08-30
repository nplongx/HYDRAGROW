import { z } from 'zod';

// Mirrors ScriptSensorInput in hydragrow-backend/src/models/script.rs
export const SENSOR_FIELDS = ['ph', 'ec', 'temp', 'water_level'] as const;
// Mirrors ScriptFsmInput fields available to recipe_override scripts
export const FSM_FIELDS = ['ph', 'ec', 'stage_index', 'elapsed_sec'] as const;

export const ComparisonOperatorSchema = z.enum(['>', '<', '>=', '<=', '==', '!=']);
export type ComparisonOperator = z.infer<typeof ComparisonOperatorSchema>;

export const ConditionSchema = z.object({
  sensor: z.string().min(1),
  operator: ComparisonOperatorSchema,
  value: z.number(),
});
export type Condition = z.infer<typeof ConditionSchema>;

export const AlertActionSchema = z.object({
  type: z.literal('alert'),
  level: z.enum(['info', 'warning', 'error']),
  title: z.string().optional(),
  message: z.string().min(1),
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

export const TriggerSchema = z.object({
  type: z.enum(['sensor', 'fsm']),
});

// React Flow canvas state — opaque to the compiler, used only to restore the UI.
export const AutomationNodeSchema = z.object({
  id: z.string(),
  type: z.enum(['sensor', 'condition', 'delay', 'action']),
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
    conditions: z.array(ConditionSchema).min(1),
    actions: z.array(ActionSchema).min(1),
    nodes: z.array(AutomationNodeSchema),
    edges: z.array(AutomationEdgeSchema),
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

import type { AutomationIr } from '../lib/automation/ir';

export interface UserScript {
  id: string;
  device_id: string;
  kind: 'alert' | 'recipe_override' | 'action_command';
  name: string;
  source: string;
  enabled: boolean;
  ir_json: AutomationIr | null;
  created_at: string;
  updated_at: string;
}

export interface UpsertScriptRequest {
  id?: string;
  kind: 'alert' | 'recipe_override' | 'action_command';
  name: string;
  source: string;
  enabled?: boolean;
  ir_json?: AutomationIr;
  next_flow_ids?: string[];
}

export interface ConditionTraceEntry {
  description: string;
  passed: boolean;
  actual_value: number | null;
}

export interface TestScriptRequest {
  ir_json: AutomationIr;
  sample: Record<string, number | number[]>;
}

export interface TestScriptResponse {
  will_fire: boolean;
  trace: ConditionTraceEntry[];
  actions_preview: Record<string, unknown>[];
}

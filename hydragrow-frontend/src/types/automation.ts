import type { AutomationIr } from '../lib/automation/ir';

export interface UserScript {
  id: string;
  device_id: string;
  kind: 'alert' | 'recipe_override' | 'action_command' | 'config_override';
  name: string;
  source: string;
  enabled: boolean;
  ir_json: AutomationIr | null;
  created_at: string;
  updated_at: string;
}

export interface UpsertScriptRequest {
  id?: string;
  kind: 'alert' | 'recipe_override' | 'action_command' | 'config_override';
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

export interface ConfigOverrideActiveItem {
  configKey: string;
  deviceId: string;
  deviceName?: string;
  originalValue: string | number;
  currentValue: string | number;
  unit?: string;
  flowName: string;
  flowId?: string;
  status: 'active' | 'restored';
}

export interface ConfigAuditLogEntry {
  id: string;
  timestamp: string;
  deviceId: string;
  deviceName?: string;
  configKey: string;
  originalValue: string | number;
  overrideValue: string | number;
  unit?: string;
  reason: string;
  status: 'applied' | 'restored' | 'clamped_warning';
}


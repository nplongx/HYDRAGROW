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
  kind: 'alert' | 'recipe_override' | 'action_command';
  name: string;
  source: string;
  enabled?: boolean;
  ir_json?: AutomationIr;
}

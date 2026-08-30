import * as Blockly from "blockly/core";
import type { Action, Condition } from "../../../lib/automation/ir";

export function extractConditions(workspace: Blockly.Workspace): Condition[] {
  return workspace
    .getBlocksByType("hydragrow_sensor_condition", false)
    .map((block) => ({
      sensor: block.getFieldValue("SENSOR"),
      operator: block.getFieldValue("OPERATOR") as Condition["operator"],
      value: Number(block.getFieldValue("VALUE")),
    }));
}

export function extractActions(workspace: Blockly.Workspace): Action[] {
  const alerts: Action[] = workspace.getBlocksByType('hydragrow_alert_action', false).map((block) => ({
    type: 'alert' as const,
    level: block.getFieldValue('LEVEL') as Action extends { type: 'alert' } ? Action['level'] : never,
    message: block.getFieldValue('MESSAGE'),
  }));
  const advances: Action[] = workspace.getBlocksByType('hydragrow_advance_stage_action', false).map((block) => ({
    type: 'advance_stage' as const,
    targetStageOffset: Number(block.getFieldValue('OFFSET')),
    reason: block.getFieldValue('REASON'),
  }));
  const endSeasons: Action[] = workspace
    .getBlocksByType('hydragrow_end_season_action', false)
    .map((block) => ({ type: 'end_season' as const, reason: block.getFieldValue('REASON') }));
  const doses: Action[] = workspace.getBlocksByType('hydragrow_dose_action', false).map((block) => ({
    type: 'dose' as const,
    pump: block.getFieldValue('PUMP') as 'PUMP_A' | 'PUMP_B' | 'PH_UP' | 'PH_DOWN',
    doseMl: Number(block.getFieldValue('DOSE_ML')),
    pwm: Number(block.getFieldValue('PWM')),
  }));
  const waters: Action[] = workspace.getBlocksByType('hydragrow_water_action', false).map((block) => {
    const pump = block.getFieldValue('PUMP') as 'WATER_PUMP_IN' | 'WATER_PUMP_OUT' | 'MIST_VALVE' | 'OSAKA_PUMP';
    if (block.getFieldValue('STATE') === 'on') {
      return { type: 'water_on' as const, pump, durationSec: Number(block.getFieldValue('DURATION_SEC')) };
    }
    return { type: 'water_off' as const, pump };
  });
  const emergencyStops: Action[] = workspace
    .getBlocksByType('hydragrow_emergency_stop_action', false)
    .map(() => ({ type: 'emergency_stop' as const }));
  return [...alerts, ...advances, ...endSeasons, ...doses, ...waters, ...emergencyStops];
}

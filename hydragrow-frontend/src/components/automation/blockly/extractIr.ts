import * as Blockly from 'blockly/core';
import type { Action, Condition } from '../../../lib/automation/ir';

export function extractConditions(workspace: Blockly.Workspace): Condition[] {
  return workspace
    .getBlocksByType('hydragrow_sensor_condition', false)
    .map((block) => ({
      sensor: block.getFieldValue('SENSOR'),
      operator: block.getFieldValue('OPERATOR') as Condition['operator'],
      value: Number(block.getFieldValue('VALUE')),
    }));
}

export function extractActions(workspace: Blockly.Workspace): Action[] {
  return workspace.getBlocksByType('hydragrow_alert_action', false).map((block) => ({
    type: 'alert' as const,
    level: block.getFieldValue('LEVEL') as Action extends { type: 'alert' } ? Action['level'] : never,
    message: block.getFieldValue('MESSAGE'),
  }));
}

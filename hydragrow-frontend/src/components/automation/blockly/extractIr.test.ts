import { describe, expect, it, beforeEach } from 'vitest';
import * as Blockly from 'blockly/core';
import { registerHydragrowBlocks } from './blocks';
import { extractConditions, extractActions } from './extractIr';

describe('extractIr', () => {
  let workspace: Blockly.Workspace;

  beforeEach(() => {
    registerHydragrowBlocks();
    workspace = new Blockly.Workspace();
  });

  it('extracts a single condition block', () => {
    const block = workspace.newBlock('hydragrow_sensor_condition');
    block.setFieldValue('ph', 'SENSOR');
    block.setFieldValue('>', 'OPERATOR');
    block.setFieldValue('7.5', 'VALUE');
    const conditions = extractConditions(workspace);
    expect(conditions).toEqual([{ sensor: 'ph', operator: '>', value: 7.5 }]);
  });

  it('extracts a single alert action block', () => {
    const block = workspace.newBlock('hydragrow_alert_action');
    block.setFieldValue('warning', 'LEVEL');
    block.setFieldValue('pH cao', 'MESSAGE');
    const actions = extractActions(workspace);
    expect(actions).toEqual([{ type: 'alert', level: 'warning', message: 'pH cao' }]);
  });

  it('extracts a single advance_stage action block', () => {
    const block = workspace.newBlock('hydragrow_advance_stage_action');
    block.setFieldValue('2', 'OFFSET');
    block.setFieldValue('Đủ 24h', 'REASON');
    const actions = extractActions(workspace);
    expect(actions).toEqual([{ type: 'advance_stage', targetStageOffset: 2, reason: 'Đủ 24h' }]);
  });
});

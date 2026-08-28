import { describe, expect, it } from 'vitest';
import * as Blockly from 'blockly/core';
import { registerHydragrowBlocks } from './blocks';

describe('registerHydragrowBlocks', () => {
  it('registers the sensor_condition block', () => {
    registerHydragrowBlocks();
    expect(Blockly.Blocks['hydragrow_sensor_condition']).toBeDefined();
  });

  it('registers the alert_action block', () => {
    registerHydragrowBlocks();
    expect(Blockly.Blocks['hydragrow_alert_action']).toBeDefined();
  });

  it('registers the advance_stage_action block', () => {
    registerHydragrowBlocks();
    expect(Blockly.Blocks['hydragrow_advance_stage_action']).toBeDefined();
  });

  it('re-registering with a different field list updates the dropdown options', () => {
    registerHydragrowBlocks(['stage_index', 'elapsed_sec']);
    const block = new Blockly.Block(new Blockly.Workspace(), 'hydragrow_sensor_condition');
    const dropdown = block.getField('SENSOR') as Blockly.FieldDropdown;
    expect(dropdown.getOptions().map(([, value]) => value)).toEqual(['stage_index', 'elapsed_sec']);
  });
});

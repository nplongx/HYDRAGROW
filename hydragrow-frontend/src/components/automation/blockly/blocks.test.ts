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
});

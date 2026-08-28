import * as Blockly from 'blockly/core';
import { SENSOR_FIELDS } from '../../../lib/automation/ir';

let registeredFields: readonly string[] | null = null;

/**
 * Idempotent per field-list — safe to call from multiple mounted editor
 * instances. Re-registering with a *different* field list (e.g. switching
 * kind from alert → recipe_override) rebuilds the sensor dropdown options.
 */
export function registerHydragrowBlocks(fields: readonly string[] = SENSOR_FIELDS) {
  if (registeredFields && registeredFields.join(',') === fields.join(',')) return;
  registeredFields = fields;

  Blockly.Blocks['hydragrow_sensor_condition'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField(new Blockly.FieldDropdown(fields.map((f) => [f, f])), 'SENSOR')
        .appendField(
          new Blockly.FieldDropdown([
            ['>', '>'],
            ['<', '<'],
            ['>=', '>='],
            ['<=', '<='],
            ['==', '=='],
            ['!=', '!='],
          ]),
          'OPERATOR',
        )
        .appendField(new Blockly.FieldNumber(0), 'VALUE');
      this.setPreviousStatement(true, 'condition');
      this.setNextStatement(true, 'condition');
      this.setColour(210);
      this.setTooltip('So sánh một giá trị cảm biến với một ngưỡng.');
    },
  };

  Blockly.Blocks['hydragrow_alert_action'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField('Alert')
        .appendField(
          new Blockly.FieldDropdown([
            ['info', 'info'],
            ['warning', 'warning'],
            ['error', 'error'],
          ]),
          'LEVEL',
        );
      this.appendDummyInput().appendField(new Blockly.FieldTextInput('Message'), 'MESSAGE');
      this.setPreviousStatement(true, 'action');
      this.setColour(20);
      this.setTooltip('Gửi một alert khi điều kiện phía trên đúng.');
    },
  };

  Blockly.Blocks['hydragrow_advance_stage_action'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField('Advance stage, offset')
        .appendField(new Blockly.FieldNumber(1), 'OFFSET');
      this.appendDummyInput().appendField(new Blockly.FieldTextInput('Reason'), 'REASON');
      this.setPreviousStatement(true, 'action');
      this.setColour(160);
      this.setTooltip('Chuyển sang stage khác trong recipe khi điều kiện phía trên đúng.');
    },
  };
}

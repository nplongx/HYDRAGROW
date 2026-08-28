import * as Blockly from 'blockly/core';
import { SENSOR_FIELDS } from '../../../lib/automation/ir';

let registered = false;

/** Idempotent — safe to call from multiple mounted editor instances. */
export function registerHydragrowBlocks() {
  if (registered) return;
  registered = true;

  Blockly.Blocks['hydragrow_sensor_condition'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField(new Blockly.FieldDropdown(SENSOR_FIELDS.map((f) => [f, f])), 'SENSOR')
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
}

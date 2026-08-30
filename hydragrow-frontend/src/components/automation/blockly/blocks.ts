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

  Blockly.Blocks['hydragrow_end_season_action'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField('Kết thúc mùa vụ, lý do')
        .appendField(new Blockly.FieldTextInput('Hoàn thành'), 'REASON');
      this.setPreviousStatement(true, 'action');
      this.setColour(65);
      this.setTooltip('Đóng mùa vụ hiện tại (crop_seasons.status = completed). Không reset gain learner.');
    },
  };

  Blockly.Blocks['hydragrow_dose_action'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField('Dose')
        .appendField(
          new Blockly.FieldDropdown([
            ['PUMP_A', 'PUMP_A'],
            ['PUMP_B', 'PUMP_B'],
            ['PH_UP', 'PH_UP'],
            ['PH_DOWN', 'PH_DOWN'],
          ]),
          'PUMP',
        );
      this.appendDummyInput()
        .appendField('ml')
        .appendField(new Blockly.FieldNumber(1, 0), 'DOSE_ML')
        .appendField('PWM %')
        .appendField(new Blockly.FieldNumber(100, 1, 100), 'PWM');
      this.setPreviousStatement(true, 'action');
      this.setColour(0);
      this.setTooltip('Bơm một liều dung dịch — luôn đi qua safety gate ở backend trước khi publish.');
    },
  };

  Blockly.Blocks['hydragrow_water_action'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput()
        .appendField('Water')
        .appendField(
          new Blockly.FieldDropdown([
            ['WATER_PUMP_IN', 'WATER_PUMP_IN'],
            ['WATER_PUMP_OUT', 'WATER_PUMP_OUT'],
            ['MIST_VALVE', 'MIST_VALVE'],
            ['OSAKA_PUMP', 'OSAKA_PUMP'],
          ]),
          'PUMP',
        )
        .appendField(new Blockly.FieldDropdown([['on', 'on'], ['off', 'off']]), 'STATE');
      this.appendDummyInput()
        .appendField('giây (chỉ dùng khi bật)')
        .appendField(new Blockly.FieldNumber(10, 0), 'DURATION_SEC');
      this.setPreviousStatement(true, 'action');
      this.setColour(200);
      this.setTooltip('Bật/tắt bơm nước hoặc van tuần hoàn.');
    },
  };

  Blockly.Blocks['hydragrow_emergency_stop_action'] = {
    init(this: Blockly.Block) {
      this.appendDummyInput().appendField('EMERGENCY STOP — dừng mọi actor');
      this.setPreviousStatement(true, 'action');
      this.setColour(0);
      this.setTooltip('Publish lệnh dừng khẩn cấp cho toàn bộ thiết bị.');
    },
  };
}

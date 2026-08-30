import * as Blockly from 'blockly/core';
import type { Action, Condition } from '../../../lib/automation/ir';

/**
 * Nghịch đảo của `extractConditions`/`extractActions` — vẽ lại block stack mà một
 * người dùng đáng lẽ đã kéo thả, để mở một automation Blockly đã lưu không còn
 * cho ra canvas trống (trước đây `BlockLogicEditor` chỉ đọc MỘT CHIỀU từ workspace
 * ra ngoài, không bao giờ ghi ngược từ IR đã lưu vào workspace).
 *
 * `extractConditions`/`extractActions` đọc MỘT block đúng loại có trong workspace,
 * bất kể có nối stack với nhau hay không (`getBlocksByType(type, false)`), nên thứ
 * tự nối ở đây chỉ là UX cho đẹp mắt — không ảnh hưởng tính đúng của round-trip.
 */
export function hydrateWorkspace(
  workspace: Blockly.Workspace,
  conditions: Condition[],
  actions: Action[],
): void {
  let previousBlock: Blockly.Block | null = null;
  let offsetY = 0;

  const placeAndChain = (block: Blockly.Block) => {
    (block as unknown as { initSvg?: () => void }).initSvg?.();
    (block as unknown as { render?: () => void }).render?.();
    if (previousBlock?.nextConnection && block.previousConnection) {
      previousBlock.nextConnection.connect(block.previousConnection);
    } else {
      (block as unknown as { moveBy?: (x: number, y: number) => void }).moveBy?.(20, offsetY);
    }
    previousBlock = block;
    offsetY += 40;
  };

  for (const condition of conditions) {
    const block = workspace.newBlock('hydragrow_sensor_condition');
    block.setFieldValue(condition.sensor, 'SENSOR');
    block.setFieldValue(condition.operator, 'OPERATOR');
    block.setFieldValue(String(condition.value), 'VALUE');
    placeAndChain(block);
  }

  for (const action of actions) {
    if (action.type === 'alert') {
      const block = workspace.newBlock('hydragrow_alert_action');
      block.setFieldValue(action.level, 'LEVEL');
      block.setFieldValue(action.message, 'MESSAGE');
      placeAndChain(block);
    } else if (action.type === 'advance_stage') {
      const block = workspace.newBlock('hydragrow_advance_stage_action');
      block.setFieldValue(String(action.targetStageOffset), 'OFFSET');
      block.setFieldValue(action.reason, 'REASON');
      placeAndChain(block);
    } else if (action.type === 'dose') {
      const block = workspace.newBlock('hydragrow_dose_action');
      block.setFieldValue(action.pump, 'PUMP');
      block.setFieldValue(String(action.doseMl), 'DOSE_ML');
      block.setFieldValue(String(action.pwm), 'PWM');
      placeAndChain(block);
    } else if (action.type === 'water_on' || action.type === 'water_off') {
      const block = workspace.newBlock('hydragrow_water_action');
      block.setFieldValue(action.pump, 'PUMP');
      block.setFieldValue(action.type === 'water_on' ? 'on' : 'off', 'STATE');
      if (action.type === 'water_on') {
        block.setFieldValue(String(action.durationSec), 'DURATION_SEC');
      }
      placeAndChain(block);
    } else if (action.type === 'emergency_stop') {
      const block = workspace.newBlock('hydragrow_emergency_stop_action');
      placeAndChain(block);
    }
  }
}

import { describe, it, expect } from 'vitest';
import { findScheduleConflicts } from './scheduleConflicts';
import type { UserScript } from '../../types/automation';

const makeScript = (overrides: Partial<UserScript>): UserScript => ({
  id: overrides.id || 'id',
  device_id: 'dev-1',
  kind: 'action_command',
  name: overrides.name || 'Flow',
  source: '',
  enabled: overrides.enabled ?? true,
  ir_json: overrides.ir_json ?? null,
  created_at: '',
  updated_at: '',
  ...overrides,
});

describe('findScheduleConflicts', () => {
  it('phát hiện xung đột khi 2 flow cùng thiết bị và cùng giờ cron', () => {
    const flowA = makeScript({
      id: 'a',
      name: 'Tưới buổi sáng',
      ir_json: {
        kind: 'action_command',
        trigger: { type: 'cron', cronExpression: '0 0 6 * * *', timezone: 'Asia/Ho_Chi_Minh' },
        actions: [{ type: 'water_on', pump: 'WATER_PUMP_IN', durationSec: 30 }],
      } as any,
    });
    const flowB = makeScript({
      id: 'b',
      name: 'Xả bồn định kỳ',
      ir_json: {
        kind: 'action_command',
        trigger: { type: 'cron', cronExpression: '0 0 6 * * *', timezone: 'Asia/Ho_Chi_Minh' },
        actions: [{ type: 'water_off', pump: 'WATER_PUMP_IN' }],
      } as any,
    });

    const conflicts = findScheduleConflicts([flowA, flowB]);
    expect(conflicts).toHaveLength(1);
    expect(conflicts[0].sharedPumps).toEqual(['WATER_PUMP_IN']);
  });

  it('không báo xung đột khi trùng giờ nhưng khác thiết bị', () => {
    const flowA = makeScript({
      id: 'a',
      ir_json: {
        kind: 'action_command',
        trigger: { type: 'cron', cronExpression: '0 0 6 * * *', timezone: 'Asia/Ho_Chi_Minh' },
        actions: [{ type: 'dose', pump: 'PUMP_A', doseMl: 5, pwm: 80 }],
      } as any,
    });
    const flowB = makeScript({
      id: 'b',
      ir_json: {
        kind: 'action_command',
        trigger: { type: 'cron', cronExpression: '0 0 6 * * *', timezone: 'Asia/Ho_Chi_Minh' },
        actions: [{ type: 'dose', pump: 'PH_UP', doseMl: 2, pwm: 60 }],
      } as any,
    });

    expect(findScheduleConflicts([flowA, flowB])).toHaveLength(0);
  });

  it('không báo xung đột khi không có flow nào active hoặc dùng trigger cron', () => {
    const flowDisabled = makeScript({
      id: 'a',
      enabled: false,
      ir_json: {
        kind: 'action_command',
        trigger: { type: 'cron', cronExpression: '0 0 6 * * *', timezone: 'Asia/Ho_Chi_Minh' },
        actions: [{ type: 'dose', pump: 'PUMP_A', doseMl: 5, pwm: 80 }],
      } as any,
    });
    const flowSensor = makeScript({
      id: 'b',
      ir_json: {
        kind: 'action_command',
        trigger: { type: 'sensor' },
        actions: [{ type: 'dose', pump: 'PUMP_A', doseMl: 5, pwm: 80 }],
      } as any,
    });

    expect(findScheduleConflicts([flowDisabled, flowSensor])).toHaveLength(0);
  });
});

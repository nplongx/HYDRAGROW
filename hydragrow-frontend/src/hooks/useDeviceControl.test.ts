import { describe, it, expect, beforeEach } from 'vitest';
import { INTERLOCK_PAIRS, ensureInterlock } from './useDeviceControl';

describe('INTERLOCK_PAIRS', () => {
  it('định nghĩa đối xứng cho 2 cặp khoá chéo', () => {
    expect(INTERLOCK_PAIRS.PH_UP).toBe('PH_DOWN');
    expect(INTERLOCK_PAIRS.PH_DOWN).toBe('PH_UP');
    expect(INTERLOCK_PAIRS.WATER_PUMP_IN).toBe('WATER_PUMP_OUT');
    expect(INTERLOCK_PAIRS.WATER_PUMP_OUT).toBe('WATER_PUMP_IN');
  });
});

describe('ensureInterlock', () => {
  beforeEach(() => {});

  it('chặn bật PH_UP khi PH_DOWN đang chạy', async () => {
    const error = await ensureInterlock('PH_UP', 'on', { ph_down: true });
    expect(error).toContain('pH Down');
  });

  it('chặn bật PH_DOWN khi PH_UP đang chạy', async () => {
    const error = await ensureInterlock('PH_DOWN', 'on', { ph_up: true });
    expect(error).toContain('pH Up');
  });

  it('cho phép bật PH_UP khi PH_DOWN đang tắt', async () => {
    const error = await ensureInterlock('PH_UP', 'on', { ph_down: false });
    expect(error).toBeNull();
  });

  it('không chặn hành động off dù cặp kia đang chạy', async () => {
    const error = await ensureInterlock('PH_UP', 'off', { ph_down: true });
    expect(error).toBeNull();
  });

  it('giữ nguyên hành vi cũ cho cặp cấp/xả nước', async () => {
    const error = await ensureInterlock('WATER_PUMP_IN', 'on', { water_pump_out: true });
    expect(error).toContain('Bơm xả thoát');
  });

  it('chặn fail-closed khi trạng thái pump đối tác không được biết', async () => {
    const error = await ensureInterlock('PH_UP', 'on', {});
    expect(error).toContain('KHÔNG XÁC ĐỊNH TRẠNG THÁI AN TOÀN');
  });

  it('chặn fail-closed khi chưa có actuator snapshot', async () => {
    const error = await ensureInterlock('WATER_PUMP_IN', 'on');
    expect(error).toContain('KHÔNG XÁC ĐỊNH TRẠNG THÁI AN TOÀN');
  });
});

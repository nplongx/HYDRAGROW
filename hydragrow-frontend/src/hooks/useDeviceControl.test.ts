import { describe, it, expect, beforeEach } from 'vitest';
import { useDeviceStore } from '../store/useDeviceStore';
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
  beforeEach(() => {
    useDeviceStore.setState({
      sensorData: {
        ...useDeviceStore.getState().sensorData,
        pump_status: {},
      } as any,
    });
  });

  it('chặn bật PH_UP khi PH_DOWN đang chạy', async () => {
    useDeviceStore.setState({
      sensorData: { pump_status: { ph_down: true } } as any,
    });
    const error = await ensureInterlock('PH_UP', 'on');
    expect(error).toContain('pH Down');
  });

  it('chặn bật PH_DOWN khi PH_UP đang chạy', async () => {
    useDeviceStore.setState({
      sensorData: { pump_status: { ph_up: true } } as any,
    });
    const error = await ensureInterlock('PH_DOWN', 'on');
    expect(error).toContain('pH Up');
  });

  it('cho phép bật PH_UP khi PH_DOWN đang tắt', async () => {
    useDeviceStore.setState({
      sensorData: { pump_status: { ph_down: false } } as any,
    });
    const error = await ensureInterlock('PH_UP', 'on');
    expect(error).toBeNull();
  });

  it('không chặn hành động off dù cặp kia đang chạy', async () => {
    useDeviceStore.setState({
      sensorData: { pump_status: { ph_down: true } } as any,
    });
    const error = await ensureInterlock('PH_UP', 'off');
    expect(error).toBeNull();
  });

  it('giữ nguyên hành vi cũ cho cặp cấp/xả nước', async () => {
    useDeviceStore.setState({
      sensorData: { pump_status: { water_pump_out: true } } as any,
    });
    const error = await ensureInterlock('WATER_PUMP_IN', 'on');
    expect(error).toContain('Bơm xả thoát');
  });
});

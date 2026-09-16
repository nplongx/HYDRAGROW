import { describe, expect, it } from 'vitest';
import { deviceConfigQueryKey, mergeUnifiedDeviceConfig } from './useDeviceConfig';
import { deviceTelemetryQueryKey, readSnapshot } from './useDeviceTelemetry';
import { journalQueryKey } from './useSystemEvents';

describe('frontend domain data boundaries', () => {
  it('merges unified config without inventing missing fields', () => {
    const result = mergeUnifiedDeviceConfig({
      device_config: { device_id: 'device-a', control_mode: 'auto' },
      water_config: { water_level_target: 80 },
    });

    expect(result).toEqual({
      device_id: 'device-a',
      control_mode: 'auto',
      water_level_target: 80,
    });
    expect(result.ph_target).toBeUndefined();
    expect(() => mergeUnifiedDeviceConfig({
      device_config: { device_id: 'device-a' },
    }, 'device-b')).toThrow('Config response belongs to a different device');
  });

  it('rejects malformed telemetry instead of creating plausible defaults', () => {
    expect(() => readSnapshot({ data: undefined })).toThrow('Telemetry response is unavailable');
    expect(() => readSnapshot({ data: { device_id: 'device-a', axes: [], operational_state: 'monitoring', actuator_contradictory: false } as any })).not.toThrow();
    expect(() => readSnapshot({ data: { device_id: 'device-a' } as any })).toThrow('Telemetry response is invalid');
    expect(() => readSnapshot({ data: { device_id: 'device-a', axes: [] } as any }, 'device-b'))
      .toThrow('Telemetry response belongs to a different device');
  });
});

describe('canonical query-key isolation', () => {
  it('includes device identity for device-scoped resources', () => {
    expect(deviceConfigQueryKey('device-a')).not.toEqual(deviceConfigQueryKey('device-b'));
    expect(deviceTelemetryQueryKey('device-a')).not.toEqual(deviceTelemetryQueryKey('device-b'));
    expect(journalQueryKey({ deviceId: 'device-a' })).not.toEqual(journalQueryKey({ deviceId: 'device-b' }));
  });

  it('serializes equivalent Journal filters deterministically', () => {
    const first = journalQueryKey({ deviceId: 'device-a', category: 'alert', level: 'critical', unresolved: true, search: ' pump ', from: '2026-09-01', to: '2026-09-16' });
    const second = journalQueryKey({ deviceId: 'device-a', category: 'alert', level: 'critical', unresolved: true, search: ' pump ', from: '2026-09-01', to: '2026-09-16' });
    expect(first).toEqual(second);
  });
});

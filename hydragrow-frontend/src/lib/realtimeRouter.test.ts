import { describe, expect, it } from 'vitest';
import { isMalformedRealtimeFrame, routeRealtimeEvent } from './realtimeRouter';

describe('realtimeRouter', () => {
  it.each([
    ['sensor_update', 'telemetry'],
    ['telemetry_snapshot', 'telemetry'],
    ['device_status', 'status'],
    ['health_snapshot', 'status'],
    ['command_lifecycle', 'command_lifecycle'],
    ['alert', 'alert'],
    ['unknown', 'ignored'],
  ] as const)('routes %s to %s', (type, expected) => {
    expect(routeRealtimeEvent(type)).toBe(expected);
  });

  it('rejects malformed frame roots without mutating state', () => {
    expect(isMalformedRealtimeFrame(null)).toBe(true);
    expect(isMalformedRealtimeFrame('not-json-object')).toBe(true);
    expect(isMalformedRealtimeFrame([])).toBe(true);
    expect(isMalformedRealtimeFrame({ type: 'telemetry_snapshot' })).toBe(false);
  });
});

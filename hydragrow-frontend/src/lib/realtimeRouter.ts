export type RealtimeRoute =
  | 'telemetry'
  | 'status'
  | 'command_lifecycle'
  | 'alert'
  | 'ignored';

export function routeRealtimeEvent(type: unknown): RealtimeRoute {
  if (type === 'sensor_update' || type === 'telemetry_snapshot') return 'telemetry';
  if (
    type === 'device_status' ||
    type === 'fsm_state_update' ||
    type === 'controller_status' ||
    type === 'device_health' ||
    type === 'health_snapshot'
  ) return 'status';
  if (type === 'command_lifecycle') return 'command_lifecycle';
  if (type === 'alert') return 'alert';
  return 'ignored';
}

export function isMalformedRealtimeFrame(value: unknown): boolean {
  return !value || typeof value !== 'object' || Array.isArray(value);
}

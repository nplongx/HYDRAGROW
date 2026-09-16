import { apiGet } from '../lib/apiClient';
import type { AuthoritativeTelemetrySnapshot } from '../types/models';

export type LatestTelemetryResponse = { data?: AuthoritativeTelemetrySnapshot };

export function normalizeLatestTelemetry(response: LatestTelemetryResponse, expectedDeviceId: string): AuthoritativeTelemetrySnapshot {
  const snapshot = response?.data;
  if (!snapshot || typeof snapshot !== 'object' || !snapshot.device_id || !Array.isArray(snapshot.axes)) throw new Error('Telemetry response is invalid');
  if (snapshot.device_id !== expectedDeviceId) throw new Error('Telemetry response belongs to a different device');
  if (snapshot.operational_state === undefined || snapshot.actuator_contradictory === undefined) throw new Error('Telemetry response is missing canonical operational fields');
  return snapshot;
}

export const telemetryApi = {
  latest: async (deviceId: string, signal?: AbortSignal) => normalizeLatestTelemetry(await apiGet<LatestTelemetryResponse>(`/devices/${encodeURIComponent(deviceId)}/sensors/latest`, { signal }), deviceId),
};

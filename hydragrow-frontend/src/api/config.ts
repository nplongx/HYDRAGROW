import { apiGet, apiPut } from '../lib/apiClient';
import type { ConfigurationSyncStatus, UnifiedDeviceConfig } from '../types/models';

export type UnifiedConfigResponse = {
  device_config: Partial<UnifiedDeviceConfig>;
  water_config: Partial<UnifiedDeviceConfig>;
  safety_config: Partial<UnifiedDeviceConfig>;
  sensor_calibration: Partial<UnifiedDeviceConfig>;
  dosing_calibration: Partial<UnifiedDeviceConfig>;
};

export function normalizeUnifiedConfig(response: Partial<UnifiedConfigResponse>, expectedDeviceId: string): Partial<UnifiedDeviceConfig> {
  const merged = { ...response.device_config, ...response.water_config, ...response.safety_config, ...response.sensor_calibration, ...response.dosing_calibration };
  if (merged.device_id && merged.device_id !== expectedDeviceId) throw new Error('Config response belongs to a different device');
  return merged;
}

export type UnifiedConfigPayload = UnifiedConfigResponse;

export const configApi = {
  getRaw: async (deviceId: string, signal?: AbortSignal) =>
    apiGet<UnifiedConfigResponse>(`/devices/${encodeURIComponent(deviceId)}/config/unified`, { signal }),
  get: async (deviceId: string, signal?: AbortSignal) => normalizeUnifiedConfig(await apiGet<UnifiedConfigResponse>(`/devices/${encodeURIComponent(deviceId)}/config/unified`, { signal }), deviceId),
  update: (deviceId: string, payload: UnifiedConfigPayload) => apiPut<{ status?: string; config_version?: number }>(`/devices/${encodeURIComponent(deviceId)}/config/unified`, payload),
  syncStatus: (deviceId: string, signal?: AbortSignal) =>
    apiGet<ConfigurationSyncStatus>(`/devices/${encodeURIComponent(deviceId)}/config/sync`, { signal }),
};

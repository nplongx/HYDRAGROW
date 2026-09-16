import { apiDelete, apiGet, apiPost, apiPut } from '../lib/apiClient';
import type { OwnedDevice, StatusPayload } from '../types/models';

export type DeviceStatus = StatusPayload;

export interface ClaimDeviceResponse {
  device_id: string;
  label: string | null;
  qr_payload: string;
}

export const devicesApi = {
  list: (signal?: AbortSignal) => apiGet<OwnedDevice[]>('/devices', { signal }),
  status: (deviceId: string, signal?: AbortSignal) => apiGet<DeviceStatus>(`/devices/${deviceId}/status`, { signal }),
  claim: (deviceId: string, label: string | null) =>
    apiPost<ClaimDeviceResponse>('/devices/claim', { device_id: deviceId, label }),
  rename: (deviceId: string, label: string | null) => apiPut(`/devices/${deviceId}/label`, { label }),
  unclaim: (deviceId: string) => apiDelete(`/devices/${deviceId}/claim`),
};

import { apiGet, apiPost } from '../lib/apiClient';
import type { OtaStatus, WifiConfigStatus, WifiProvisionEntry } from '../types/models';

export const deviceSettingsApi = {
  otaStatus: (deviceId: string, signal?: AbortSignal) =>
    apiGet<OtaStatus>(`/devices/${deviceId}/ota/status`, { signal }),

  triggerOta: (deviceId: string) =>
    apiPost(`/devices/${deviceId}/ota/trigger`, {}, { 'X-User-Confirmed': 'true' }),

  triggerOtaWithWifi: (deviceId: string, configVersion: number, entries: WifiProvisionEntry[]) =>
    apiPost(`/devices/${deviceId}/ota/trigger`, {
      wifi: { config_version: configVersion, entries },
    }, { 'X-User-Confirmed': 'true' }),

  wifiConfig: (deviceId: string) =>
    apiGet<WifiConfigStatus>(`/devices/${deviceId}/wifi`),

  saveWifi: (deviceId: string, candidates: Array<{ ssid: string; password: string; priority: number }>) =>
    apiPost(`/devices/${deviceId}/wifi`, { candidates }, { 'X-User-Confirmed': 'true' }),

  startPhCalibration: (deviceId: string) =>
    apiPost(`/devices/${deviceId}/calibration/ph/start`, { mode: '2-point' }),

  capturePhCalibration: (deviceId: string, payload: { point: number; sample_target: number; window_seconds: number }, timeoutMs: number) =>
    apiPost(`/devices/${deviceId}/calibration/ph/capture`, payload, undefined, { timeoutMs }),

  finishPhCalibration: (deviceId: string) =>
    apiPost(`/devices/${deviceId}/calibration/ph/finish`, {}),

  reboot: (deviceId: string) =>
    apiPost(`/devices/${deviceId}/reboot`, {}, { 'X-User-Confirmed': 'true' }),

  factoryReset: (deviceId: string) =>
    apiPost(`/devices/${deviceId}/factory-reset`, {}, { 'X-User-Confirmed': 'true' }),
};

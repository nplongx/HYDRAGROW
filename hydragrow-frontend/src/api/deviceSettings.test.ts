import { beforeEach, describe, expect, it, vi } from 'vitest';
import { deviceSettingsApi } from './deviceSettings';
import * as apiClient from '../lib/apiClient';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
}));

describe('deviceSettingsApi', () => {
  beforeEach(() => vi.clearAllMocks());

  it('preserves confirmation headers for destructive/device commands', async () => {
    vi.mocked(apiClient.apiPost).mockResolvedValue({});

    await deviceSettingsApi.reboot('dev-1');
    await deviceSettingsApi.factoryReset('dev-1');
    await deviceSettingsApi.triggerOta('dev-1');
    await deviceSettingsApi.saveWifi('dev-1', [{ ssid: 'farm', password: 'secret', priority: 0 }]);

    expect(apiClient.apiPost).toHaveBeenNthCalledWith(1, '/devices/dev-1/reboot', {}, { 'X-User-Confirmed': 'true' });
    expect(apiClient.apiPost).toHaveBeenNthCalledWith(2, '/devices/dev-1/factory-reset', {}, { 'X-User-Confirmed': 'true' });
    expect(apiClient.apiPost).toHaveBeenNthCalledWith(3, '/devices/dev-1/ota/trigger', {}, { 'X-User-Confirmed': 'true' });
    expect(apiClient.apiPost).toHaveBeenNthCalledWith(4, '/devices/dev-1/wifi', { candidates: [{ ssid: 'farm', password: 'secret', priority: 0 }] }, { 'X-User-Confirmed': 'true' });
  });

  it('preserves OTA WiFi payload and calibration timeout', async () => {
    vi.mocked(apiClient.apiPost).mockResolvedValue({ voltage: 1.234 });
    const entries = [{ ssid: 'farm', priority: 1, secret_action: 'keep' as const }];

    await deviceSettingsApi.triggerOtaWithWifi('dev-1', 8, entries);
    await deviceSettingsApi.startPhCalibration('dev-1');
    await deviceSettingsApi.capturePhCalibration('dev-1', { point: 7, sample_target: 5, window_seconds: 20 }, 25000);
    await deviceSettingsApi.finishPhCalibration('dev-1');

    expect(apiClient.apiPost).toHaveBeenNthCalledWith(1, '/devices/dev-1/ota/trigger', {
      wifi: { config_version: 8, entries },
    }, { 'X-User-Confirmed': 'true' });
    expect(apiClient.apiPost).toHaveBeenNthCalledWith(3, '/devices/dev-1/calibration/ph/capture', {
      point: 7, sample_target: 5, window_seconds: 20,
    }, undefined, { timeoutMs: 25000 });
  });

  it('keeps OTA status and WiFi reads in the domain module', async () => {
    const signal = new AbortController().signal;
    vi.mocked(apiClient.apiGet).mockResolvedValue({ device_id: 'dev-1' });

    await deviceSettingsApi.otaStatus('dev-1', signal);
    await deviceSettingsApi.wifiConfig('dev-1');

    expect(apiClient.apiGet).toHaveBeenNthCalledWith(1, '/devices/dev-1/ota/status', { signal });
    expect(apiClient.apiGet).toHaveBeenNthCalledWith(2, '/devices/dev-1/wifi');
  });
});

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { configApi } from './config';

const { apiGetMock, apiPutMock } = vi.hoisted(() => ({
  apiGetMock: vi.fn(),
  apiPutMock: vi.fn(),
}));

vi.mock('../lib/apiClient', () => ({
  apiGet: apiGetMock,
  apiPut: apiPutMock,
}));

describe('configApi', () => {
  beforeEach(() => {
    apiGetMock.mockReset();
    apiPutMock.mockReset();
  });

  it('loads durable configuration sync status for the selected device', async () => {
    const status = {
      device_id: 'device-a',
      config_version: 12,
      overall_state: 'published',
      controller: { state: 'applied', attempts: 1, last_attempt_at: null, applied_at: '2026-09-16T01:00:00Z' },
      sensor: { state: 'published', attempts: 1, last_attempt_at: '2026-09-16T01:00:00Z', applied_at: null },
      last_error: null,
      updated_at: '2026-09-16T01:00:00Z',
    };
    apiGetMock.mockResolvedValue(status);

    await expect(configApi.syncStatus('device-a')).resolves.toEqual(status);
    expect(apiGetMock).toHaveBeenCalledWith('/devices/device-a/config/sync', { signal: undefined });
  });

  it('returns the server-assigned config version from a unified update', async () => {
    apiPutMock.mockResolvedValue({ status: 'pending', config_version: 13 });
    const payload = { device_config: {}, water_config: {}, safety_config: {}, sensor_calibration: {}, dosing_calibration: {} };

    await expect(configApi.update('device-a', payload)).resolves.toEqual({ status: 'pending', config_version: 13 });
    expect(apiPutMock).toHaveBeenCalledWith('/devices/device-a/config/unified', payload);
  });
});

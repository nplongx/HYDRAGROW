import { describe, expect, it, vi } from 'vitest';
import { ApiError, apiGet } from './apiClient';

const httpFetch = vi.hoisted(() => vi.fn());

vi.mock('../platform/http', () => ({ httpFetch }));
vi.mock('../platform/settings', () => ({
  loadAppSettings: vi.fn().mockResolvedValue({ backend_url: 'http://backend', api_key: 'key' }),
  getDefaultBackendUrl: vi.fn(() => 'http://backend'),
}));

describe('apiClient', () => {
  it('normalizes canonical API errors', async () => {
    httpFetch.mockResolvedValueOnce(new Response(JSON.stringify({
      error: {
        code: 'DEVICE_FORBIDDEN',
        message: 'Device access denied',
        details: { device_id: 'device-a' },
        request_id: 'req-123',
      },
    }), { status: 403 }));

    const error = await apiGet('/devices/device-a')
      .catch((caught) => caught);

    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({
      status: 403,
      code: 'DEVICE_FORBIDDEN',
      message: 'Device access denied',
      details: { device_id: 'device-a' },
      requestId: 'req-123',
    });
  });

  it('keeps authentication failure distinct from authorization denial', async () => {
    httpFetch.mockResolvedValueOnce(new Response(JSON.stringify({
      error: {
        code: 'AUTHENTICATION_REQUIRED',
        message: 'Authentication required',
        request_id: 'req-401',
      },
    }), { status: 401 }));

    const error = await apiGet('/devices').catch((caught) => caught);

    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({
      status: 401,
      code: 'AUTHENTICATION_REQUIRED',
      requestId: 'req-401',
    });
    expect((error as ApiError).status).not.toBe(403);
  });

  it('passes AbortSignal through the transport boundary', async () => {
    httpFetch.mockResolvedValueOnce(new Response(JSON.stringify({ data: [] }), { status: 200 }));
    const controller = new AbortController();

    await apiGet('/devices', { signal: controller.signal });

    expect(httpFetch).toHaveBeenCalledWith(
      'http://backend/api/devices',
      expect.objectContaining({ signal: controller.signal }),
    );
  });
});

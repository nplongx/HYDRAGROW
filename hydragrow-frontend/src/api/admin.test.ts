import { beforeEach, describe, expect, it, vi } from 'vitest';
import { adminApi } from './admin';
import * as apiClient from '../lib/apiClient';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
  apiPatch: vi.fn(),
}));

describe('adminApi', () => {
  beforeEach(() => vi.clearAllMocks());

  it('unwraps the canonical admin user list response', async () => {
    vi.mocked(apiClient.apiGet).mockResolvedValue({
      status: 'success',
      data: [{ id: 1, email: 'admin@farm.vn' }],
    });

    await expect(adminApi.listUsers()).resolves.toEqual([{ id: 1, email: 'admin@farm.vn' }]);
    expect(apiClient.apiGet).toHaveBeenCalledWith('/admin/users', { signal: undefined });
  });

  it('passes the abort signal through admin user listing', async () => {
    const signal = new AbortController().signal;
    vi.mocked(apiClient.apiGet).mockResolvedValue({ status: 'success', data: [] });

    await adminApi.listUsers(signal);

    expect(apiClient.apiGet).toHaveBeenCalledWith('/admin/users', { signal });
  });

  it('keeps admin write endpoints behind the canonical module', async () => {
    vi.mocked(apiClient.apiPost).mockResolvedValue({ status: 'ok' });
    vi.mocked(apiClient.apiPatch).mockResolvedValue({ status: 'ok' });

    await adminApi.provisionUser({
      firebase_uid: 'uid-1',
      email: 'user@farm.vn',
      display_name: null,
      scopes: ['read:telemetry'],
    });
    await adminApi.updateUser(7, { role: 'admin', scopes: ['*'] });

    expect(apiClient.apiPost).toHaveBeenCalledWith('/admin/users', {
      firebase_uid: 'uid-1',
      email: 'user@farm.vn',
      display_name: null,
      scopes: ['read:telemetry'],
    });
    expect(apiClient.apiPatch).toHaveBeenCalledWith('/admin/users/7', {
      role: 'admin',
      scopes: ['*'],
    });
  });
});

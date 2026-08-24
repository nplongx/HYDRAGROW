import { renderHook, waitFor } from '@testing-library/react';
import { vi, describe, it, expect } from 'vitest';
import { useOwnedDevices } from './useOwnedDevices';
import * as apiClient from '../lib/apiClient';

describe('useOwnedDevices', () => {
  it('trả về danh sách thiết bị khi fetch thành công', async () => {
    const mockDevices = [
      { id: 1, user_id: 1, device_id: 'dev-001', label: 'Nhà kính A', claimed_at: '2026-08-24T00:00:00Z' },
    ];
    vi.spyOn(apiClient, 'apiGet').mockResolvedValue(mockDevices);

    const { result } = renderHook(() => useOwnedDevices());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.devices).toHaveLength(1);
    expect(result.current.devices[0].device_id).toBe('dev-001');
  });

  it('set error khi fetch thất bại', async () => {
    vi.spyOn(apiClient, 'apiGet').mockRejectedValue(new Error('Network error'));
    const { result } = renderHook(() => useOwnedDevices());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeTruthy();
  });
});

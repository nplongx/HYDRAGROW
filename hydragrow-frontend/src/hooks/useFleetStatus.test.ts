import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import { vi, describe, it, expect } from 'vitest';
import { useFleetStatus } from './useFleetStatus';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn().mockResolvedValue({ data: [
    { device_id: 'device_001', label: 'Nhà kính 1', operational_state: { contact: 'CONTACTED', freshness: 'FRESH' } },
    { device_id: 'device_002', label: null, operational_state: { contact: 'UNKNOWN', freshness: 'UNKNOWN' } },
  ] }),
}));

const wrapper = ({ children }: { children: React.ReactNode }) =>
  React.createElement(QueryClientProvider, { client: new QueryClient() }, children);

describe('useFleetStatus', () => {
  it('loads devices list from /devices endpoint', async () => {
    const { result } = renderHook(() => useFleetStatus(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.devices).toHaveLength(2);
    expect(result.current.devices[0].device_id).toBe('device_001');
  });

  it('returns empty array when API fails', async () => {
    const { apiGet } = await import('../lib/apiClient');
    vi.mocked(apiGet).mockRejectedValueOnce(new Error('Network error'));
    const { result } = renderHook(() => useFleetStatus(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.devices).toHaveLength(0);
    expect(result.current.error).toBeTruthy();
  });
});

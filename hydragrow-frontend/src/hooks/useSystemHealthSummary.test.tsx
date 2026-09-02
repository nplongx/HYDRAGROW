import { describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useSystemHealthSummary } from './useSystemHealthSummary';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn().mockResolvedValue({
    status: 'success',
    data: {
      window_seconds: 3600,
      ec_dosing_count: 2,
      ph_dosing_count: 1,
      water_operation_count: 0,
      warning_count: 1,
      critical_count: 0,
      latest_ph_dosing_at: 1735000000000,
    },
  }),
}));

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('useSystemHealthSummary', () => {
  it('loads summary counts from /devices/{id}/health-summary', async () => {
    const { apiGet } = await import('../lib/apiClient');
    const { result } = renderHook(() => useSystemHealthSummary('device-1'), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.warning_count).toBe(1);
    expect(result.current.data?.ec_dosing_count).toBe(2);
    expect(apiGet).toHaveBeenCalledWith('/devices/device-1/health-summary');
  });

  it('stays disabled with no deviceId', () => {
    const { result } = renderHook(() => useSystemHealthSummary(''), { wrapper });
    expect(result.current.fetchStatus).toBe('idle');
  });
});

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { analyticsApi } from '../api/analytics';
import { useAnalyticsHealth, useUpdateWeeklyReportPreference } from './useAnalytics';

vi.mock('../api/analytics', () => ({
  analyticsApi: {
    health: vi.fn(),
    updateWeeklyReportPreference: vi.fn(),
  },
}));

function wrapper(client: QueryClient) {
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
}

describe('analytics hooks', () => {
  it('owns device health query key and polling contract', async () => {
    vi.mocked(analyticsApi.health).mockResolvedValue({
      free_heap_bytes: 1,
      wifi_rssi_dbm: -50,
      uptime_seconds: 2,
      backend_process_cpu_percent: 3,
      last_updated_at: null,
    });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });

    const { result } = renderHook(() => useAnalyticsHealth('device-a'), { wrapper: wrapper(client) });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.free_heap_bytes).toBe(1);
    expect(analyticsApi.health).toHaveBeenCalledWith('device-a', expect.any(AbortSignal));
    expect(client.getQueryData(['device-health', 'device-a'])).toEqual(result.current.data);
    expect(client.getQueryData(['device-health', 'device-b'])).toBeUndefined();
  });

  it('invalidates whoami after weekly report preference mutation', async () => {
    vi.mocked(analyticsApi.updateWeeklyReportPreference).mockResolvedValue({ status: 'success' });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(client, 'invalidateQueries');

    const { result } = renderHook(() => useUpdateWeeklyReportPreference(), { wrapper: wrapper(client) });
    await result.current.mutateAsync({ weekly_report: true, other: 'preserved' });

    expect(analyticsApi.updateWeeklyReportPreference).toHaveBeenCalledWith({ weekly_report: true, other: 'preserved' });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ['whoami'] });
  });
});

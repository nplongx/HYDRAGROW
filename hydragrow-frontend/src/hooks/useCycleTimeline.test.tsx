import { describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useCycleTimeline } from './useCycleTimeline';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn().mockResolvedValue({
    status: 'success',
    data: [{ id: 1, device_id: 'device-1', level: 'info', category: 'dosing', title: 'Bắt đầu châm', message: '', timestamp: 1000 }],
  }),
}));

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('useCycleTimeline', () => {
  it('loads timeline events for a cycle_id', async () => {
    const { apiGet } = await import('../lib/apiClient');
    const { result } = renderHook(() => useCycleTimeline('device-1', 'cyc-9'), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(1);
    expect(apiGet).toHaveBeenCalledWith('/devices/device-1/events/cycle/cyc-9');
  });

  it('stays disabled when cycleId is null', () => {
    const { result } = renderHook(() => useCycleTimeline('device-1', null), { wrapper });
    expect(result.current.fetchStatus).toBe('idle');
  });
});

import { describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useDashboardFleet } from './useDashboardFleet';

const apiGet = vi.hoisted(() => vi.fn());
vi.mock('../lib/apiClient', () => ({ apiGet }));

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('useDashboardFleet', () => {
  it('surfaces fleet query failures instead of treating them as an empty healthy fleet', async () => {
    apiGet.mockRejectedValueOnce(new Error('fleet unavailable'));
    const { result } = renderHook(() => useDashboardFleet(), { wrapper });

    await waitFor(() => expect(result.current.error).toBeTruthy());
    expect(result.current.stations).toEqual([]);
    expect(result.current.error).toEqual(expect.objectContaining({ message: 'fleet unavailable' }));
  });

  it('rejects malformed fleet data instead of passing fabricated state to the Dashboard', async () => {
    apiGet.mockResolvedValueOnce({ status: 'success', data: null });
    const { result } = renderHook(() => useDashboardFleet(), { wrapper });

    await waitFor(() => expect(result.current.error).toBeTruthy());
    expect(result.current.error).toEqual(expect.objectContaining({ message: 'Fleet summary response is invalid' }));
  });
});

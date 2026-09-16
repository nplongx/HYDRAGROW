import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useCropSeason } from './useCropSeason';
import { apiPost } from '../lib/apiClient';

vi.mock('../lib/apiClient', () => ({
  apiDelete: vi.fn(),
  apiGet: vi.fn().mockResolvedValue({ data: [] }),
  apiPost: vi.fn(),
  apiPut: vi.fn(),
}));

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({
    status: 'Selected',
    selectedDeviceId: 'device-a',
    selectedDevice: null,
    availableDevices: [],
    error: null,
    selectDevice: vi.fn(),
    switchDevice: vi.fn(),
    clearSelection: vi.fn(),
    refreshAvailableDevices: vi.fn(),
  }),
}));

function makeClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

describe('useCropSeason query/mutation boundaries', () => {
  beforeEach(() => {
    vi.mocked(apiPost).mockReset();
    vi.mocked(apiPost).mockResolvedValue({ status: 'success' });
  });

  it('keeps season query keys isolated by device and invalidates only the mutated device', async () => {
    const client = makeClient();
    const invalidateQueries = vi.spyOn(client, 'invalidateQueries');

    const { result } = renderHook(() => useCropSeason(), {
      wrapper: ({ children }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    });

    await result.current.createSeason('Tomato', 'tomato', 'spring');

    await waitFor(() => expect(invalidateQueries).toHaveBeenCalled());
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ['seasons', 'device-a'] });
    expect(invalidateQueries).not.toHaveBeenCalledWith({ queryKey: ['seasons', 'device-b'] });
    expect(apiPost).toHaveBeenCalledWith('/devices/device-a/seasons', {
      name: 'Tomato',
      plant_type: 'tomato',
      description: 'spring',
    });
  });

  it('does not invalidate cache when mutation fails', async () => {
    const client = makeClient();
    client.setQueryData(['seasons', 'device-a', 'active'], null);
    client.setQueryData(['seasons', 'device-a', 'history'], []);
    vi.mocked(apiPost).mockRejectedValueOnce(new Error('validation failed'));

    const { result } = renderHook(() => useCropSeason(), {
      wrapper: ({ children }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    });

    await expect(result.current.createSeason('Bad', 'tomato', '')).rejects.toThrow('validation failed');
    expect(client.getQueryState(['seasons', 'device-a', 'active'])?.isInvalidated).toBe(false);
    expect(client.getQueryState(['seasons', 'device-a', 'history'])?.isInvalidated).toBe(false);
  });
});

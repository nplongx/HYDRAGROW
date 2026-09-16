import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { adminApi } from '../api/admin';
import { useAdminUsers } from './useAdminUsers';

vi.mock('../api/admin', () => ({
  adminApi: {
    listUsers: vi.fn(),
    provisionUser: vi.fn(),
    updateUser: vi.fn(),
  },
}));

function wrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}

describe('useAdminUsers', () => {
  it('loads users through the canonical admin API', async () => {
    vi.mocked(adminApi.listUsers).mockResolvedValue([]);
    const { result } = renderHook(() => useAdminUsers(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.users).toEqual([]);
    expect(adminApi.listUsers).toHaveBeenCalled();
  });
});

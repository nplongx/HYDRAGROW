import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { recipesApi } from '../api/recipes';
import { useApplyRecipe, useBulkApplyRecipe, useCreateRecipe, useDeleteRecipe, useRecipes, useUpdateRecipe } from './useRecipes';

vi.mock('../api/recipes', () => ({
  recipesApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    remove: vi.fn(),
    apply: vi.fn(),
    bulkApply: vi.fn(),
    status: vi.fn(),
  },
}));

const wrapper = (client: QueryClient) =>
  ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );

const payload = {
  name: 'Tomato',
  crop: 'tomato',
  description: 'spring',
  stages: [],
};

describe('recipe query/mutation boundaries', () => {
  beforeEach(() => {
    vi.mocked(recipesApi.list).mockResolvedValue([]);
    vi.mocked(recipesApi.create).mockResolvedValue({ status: 'success' });
    vi.mocked(recipesApi.update).mockResolvedValue({ status: 'success' });
    vi.mocked(recipesApi.remove).mockResolvedValue({ status: 'success' });
    vi.mocked(recipesApi.apply).mockResolvedValue({ status: 'success', data: {} as never });
    vi.mocked(recipesApi.bulkApply).mockResolvedValue({
      status: 'success',
      data: { succeeded: ['device-a'], failed: [] },
    });
  });

  it('owns the canonical recipe list key and domain API call', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    renderHook(() => useRecipes(), { wrapper: wrapper(client) });

    await vi.waitFor(() => expect(recipesApi.list).toHaveBeenCalled());
    expect(client.getQueryCache().find({ queryKey: ['recipes'] })).toBeDefined();
  });

  it('invalidates recipe templates after create, update, and delete', async () => {
    const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    const invalidateQueries = vi.spyOn(client, 'invalidateQueries');

    const create = renderHook(() => useCreateRecipe(), { wrapper: wrapper(client) });
    await create.result.current.mutateAsync(payload);

    const update = renderHook(() => useUpdateRecipe(), { wrapper: wrapper(client) });
    await update.result.current.mutateAsync({ recipeId: 'recipe-1', payload });

    const remove = renderHook(() => useDeleteRecipe(), { wrapper: wrapper(client) });
    await remove.result.current.mutateAsync('recipe-1');

    expect(invalidateQueries).toHaveBeenCalledTimes(3);
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ['recipes'] });
  });

  it('apply invalidates only the selected device recipe status key', async () => {
    const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    const invalidateQueries = vi.spyOn(client, 'invalidateQueries');
    const apply = renderHook(() => useApplyRecipe('device-a'), { wrapper: wrapper(client) });

    await apply.result.current.mutateAsync('recipe-1');

    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['device', 'device-a', 'recipe-status'],
    });
    expect(invalidateQueries).not.toHaveBeenCalledWith({
      queryKey: ['device', 'device-b', 'recipe-status'],
    });
    expect(recipesApi.apply).toHaveBeenCalledWith('device-a', 'recipe-1');
  });

  it('bulk apply invalidates each targeted device recipe status key', async () => {
    const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    const invalidateQueries = vi.spyOn(client, 'invalidateQueries');
    const bulkApply = renderHook(() => useBulkApplyRecipe(), { wrapper: wrapper(client) });

    await bulkApply.result.current.mutateAsync({
      device_ids: ['device-a', 'device-b'],
      recipe_id: 'recipe-1',
    });

    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ['device', 'device-a', 'recipe-status'] });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ['device', 'device-b', 'recipe-status'] });
  });
});

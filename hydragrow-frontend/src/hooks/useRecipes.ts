import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { recipesApi, type RecipePayload } from '../api/recipes';
import { queryKeys } from '../api/queryKeys';

export function useRecipes() {
  return useQuery({
    queryKey: queryKeys.recipes(),
    queryFn: ({ signal }) => recipesApi.list(signal),
  });
}

export function useCreateRecipe() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: RecipePayload) => recipesApi.create(payload),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.recipes() }),
  });
}

export function useUpdateRecipe() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ recipeId, payload }: { recipeId: string; payload: RecipePayload }) =>
      recipesApi.update(recipeId, payload),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.recipes() }),
  });
}

export function useDeleteRecipe() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (recipeId: string) => recipesApi.remove(recipeId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.recipes() }),
  });
}

export function useApplyRecipe(deviceId: string | null) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (recipeId: string) => {
      if (!deviceId) throw new Error('Chưa chọn thiết bị.');
      return recipesApi.apply(deviceId, recipeId);
    },
    onSuccess: () => {
      if (!deviceId) return;
      queryClient.invalidateQueries({ queryKey: queryKeys.recipeStatus(deviceId) });
    },
  });
}

export function useBulkApplyRecipe() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: { device_ids: string[]; recipe_id: string }) => recipesApi.bulkApply(payload),
    onSuccess: (_result, payload) => {
      for (const deviceId of payload.device_ids) {
        void queryClient.invalidateQueries({ queryKey: queryKeys.recipeStatus(deviceId) });
      }
    },
  });
}

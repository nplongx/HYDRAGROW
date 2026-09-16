import { apiDelete, apiGet, apiPost, apiPut } from '../lib/apiClient';
import type { BulkApplyResult, CropRecipe, RecipeTemplate } from '../types/models';

export type RecipePayload = {
  name: string;
  crop: string;
  description?: string;
  stages: RecipeTemplate['stages'];
};

export type RecipeStatusResponse = {
  data?: {
    device_id: string;
    active_recipe?: CropRecipe | null;
    updated_at: string;
  };
  active_recipe?: CropRecipe | null;
};

export type BulkApplyRecipePayload = {
  device_ids: string[];
  recipe_id: string;
};

export type BulkApplyRecipeResponse = {
  status: string;
  data: BulkApplyResult;
};

export const recipesApi = {
  list: (signal?: AbortSignal) =>
    apiGet<{ data?: RecipeTemplate[] }>('/recipes', { signal }).then((response) => response.data ?? []),
  create: (payload: RecipePayload) => apiPost('/recipes', payload),
  update: (recipeId: string, payload: RecipePayload) => apiPut(`/recipes/${encodeURIComponent(recipeId)}`, payload),
  remove: (recipeId: string) => apiDelete(`/recipes/${encodeURIComponent(recipeId)}`),
  status: (deviceId: string, signal?: AbortSignal) =>
    apiGet<RecipeStatusResponse>(`/devices/${encodeURIComponent(deviceId)}/recipe/status`, { signal }),
  apply: (deviceId: string, recipeId: string) =>
    apiPost<{ status: string; data: CropRecipe }>(`/devices/${encodeURIComponent(deviceId)}/recipe/apply`, {
      recipe_id: recipeId,
    }),
  bulkApply: (payload: BulkApplyRecipePayload) =>
    apiPost<BulkApplyRecipeResponse, BulkApplyRecipePayload>('/recipes/bulk-apply', payload),
};

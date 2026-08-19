import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { RefreshCcw } from 'lucide-react';
import { httpFetch } from '../../platform/http';
import { useDeviceStore } from '../../store/useDeviceStore';

export const ActiveRecipeStatus: React.FC = () => {
  const settings = useDeviceStore((s) => s.settings);
  const deviceId = useDeviceStore((s) => s.deviceId);

  const recipeStatus = useQuery({
    queryKey: ['recipe-status', settings?.backend_url, deviceId],
    enabled: Boolean(settings?.backend_url && deviceId),
    queryFn: async () => {
      const res = await httpFetch(`${settings!.backend_url}/api/devices/${deviceId}/recipe/status`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': settings?.api_key || '',
        },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
  });

  const activeRecipe = recipeStatus.data?.active_recipe || recipeStatus.data?.recipe || recipeStatus.data;
  const statusText = activeRecipe?.status || activeRecipe?.state || recipeStatus.data?.status || 'Chưa có active recipe';
  const recipeName = activeRecipe?.name || activeRecipe?.recipe_name || activeRecipe?.template_name || 'Không rõ template';
  const currentStage = activeRecipe?.current_stage || activeRecipe?.stage || recipeStatus.data?.current_stage;

  return (
    <section className="ui-card space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-bold text-emerald-950">Active recipe</h2>
          <p className="text-xs text-emerald-800/75">Đọc từ GET /api/devices/{'{device_id}'}/recipe/status.</p>
        </div>
        <button className="ui-btn-md bg-white border border-emerald-200" onClick={() => recipeStatus.refetch()} disabled={recipeStatus.isFetching}>
          <RefreshCcw size={16} className={recipeStatus.isFetching ? 'animate-spin' : ''} />
        </button>
      </div>

      {recipeStatus.isError ? (
        <div className="rounded-xl border border-red-100 bg-red-50 p-3 text-sm text-red-700">
          Không thể tải recipe status: {(recipeStatus.error as Error).message}
        </div>
      ) : (
        <div className="grid gap-3 md:grid-cols-3">
          <div className="farm-muted-panel"><span className="ui-form-label">Recipe</span><p className="font-bold text-emerald-950">{recipeStatus.isLoading ? 'Đang tải...' : recipeName}</p></div>
          <div className="farm-muted-panel"><span className="ui-form-label">Status</span><p className="font-bold text-emerald-950">{recipeStatus.isLoading ? 'Đang tải...' : statusText}</p></div>
          <div className="farm-muted-panel"><span className="ui-form-label">Stage hiện tại</span><p className="font-bold text-emerald-950">{recipeStatus.isLoading ? 'Đang tải...' : currentStage?.name || currentStage || 'N/A'}</p></div>
        </div>
      )}
    </section>
  );
};

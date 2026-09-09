import { useQuery } from '@tanstack/react-query';
import { httpFetch } from '../platform/http';
import { useDeviceStore } from '../store/useDeviceStore';
import { CropRecipe, CropStage } from '../types/models';

export const useActiveRecipeStatus = () => {
    const settings = useDeviceStore((s) => s.settings);
    const deviceId = useDeviceStore((s) => s.deviceId);

    const query = useQuery({
        queryKey: ['recipe-status', settings?.backend_url, deviceId],
        enabled: Boolean(settings?.backend_url && deviceId),
        queryFn: async () => {
            const res = await httpFetch(`${settings!.backend_url}/api/devices/${deviceId}/recipe/status`, {
                method: 'GET',
                headers: { 'Content-Type': 'application/json', 'X-API-Key': settings?.api_key || '' },
            });
            if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
            return res.json();
        },
    });

    const activeRecipe: CropRecipe | undefined = query.data?.data?.active_recipe || query.data?.active_recipe;
    const currentStage: CropStage | undefined =
        activeRecipe?.stages && activeRecipe.current_stage_index !== undefined
            ? activeRecipe.stages[activeRecipe.current_stage_index]
            : undefined;

    return { ...query, activeRecipe, currentStage };
};

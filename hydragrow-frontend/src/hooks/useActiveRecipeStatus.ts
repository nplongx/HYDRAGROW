import { useQuery } from "@tanstack/react-query";
import { useStationContext } from "../contexts/StationContext";
import type { CropRecipe, CropStage } from "../types/models";
import { recipesApi } from "../api/recipes";
import { queryKeys } from "../api/queryKeys";

export const useActiveRecipeStatus = () => {
  const { selectedDeviceId: deviceId } = useStationContext();

  const query = useQuery({
    queryKey: deviceId ? queryKeys.recipeStatus(deviceId) : ['device', null, 'recipe-status'],
    enabled: Boolean(deviceId),
    queryFn: ({ signal }) => recipesApi.status(deviceId!, signal),
  });

  const activeRecipe: CropRecipe | undefined =
    query.data?.data?.active_recipe ?? query.data?.active_recipe ?? undefined;
  const currentStage: CropStage | undefined =
    activeRecipe?.stages && activeRecipe.current_stage_index !== undefined
      ? activeRecipe.stages[activeRecipe.current_stage_index]
      : undefined;

  return { ...query, activeRecipe, currentStage };
};

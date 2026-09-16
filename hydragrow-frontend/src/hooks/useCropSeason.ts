import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { CropSeason } from '../types/models';
import { useStationContext } from '../contexts/StationContext';
import { queryKeys } from '../api/queryKeys';
import { seasonsApi } from '../api/seasons';
import toast from 'react-hot-toast';

export const useCropSeason = () => {
  const queryClient = useQueryClient();
  const { selectedDeviceId: deviceId } = useStationContext();
  const activeSeasonQuery = useQuery<CropSeason | null>({
    queryKey: deviceId ? queryKeys.seasonActive(deviceId) : ['seasons', null, 'active'],
    queryFn: ({ signal }) => seasonsApi.active(deviceId!, signal),
    enabled: Boolean(deviceId),
  });

  const seasonHistoryQuery = useQuery<CropSeason[]>({
    queryKey: deviceId ? queryKeys.seasonHistory(deviceId) : ['seasons', null, 'history'],
    queryFn: ({ signal }) => seasonsApi.history(deviceId!, signal),
    enabled: Boolean(deviceId),
  });

  const createMutation = useMutation({
    mutationFn: async (payload: { name: string; plant_type?: string; description?: string; recipe_id?: string }) => {
      return seasonsApi.create(deviceId!, payload);
    },
    onSuccess: () => {
      toast.success('Đã khởi tạo mùa vụ & nạp quy trình thành công!');
      queryClient.invalidateQueries({ queryKey: queryKeys.seasons(deviceId!) });
      queryClient.invalidateQueries({ queryKey: queryKeys.recipeStatus(deviceId!) });
    },
    onError: (err: any) => toast.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: async (payload: { name: string; plant_type: string; description: string }) => {
      return seasonsApi.updateActive(deviceId!, payload);
    },
    onSuccess: () => {
      toast.success('Cập nhật thành công!');
      queryClient.invalidateQueries({ queryKey: queryKeys.seasons(deviceId!) });
    },
    onError: (err: any) => toast.error(err.message),
  });

  const endMutation = useMutation({
    mutationFn: async () => {
      return seasonsApi.end(deviceId!);
    },
    onSuccess: () => {
      toast.success('Đã kết thúc mùa vụ canh tác!');
      queryClient.invalidateQueries({ queryKey: queryKeys.seasons(deviceId!) });
      queryClient.invalidateQueries({ queryKey: queryKeys.recipeStatus(deviceId!) });
    },
    onError: (err: any) => toast.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: async (seasonId: string) => {
      return seasonsApi.remove(deviceId!, seasonId);
    },
    onSuccess: () => {
      toast.success('Đã xoá mùa vụ.');
      queryClient.invalidateQueries({ queryKey: queryKeys.seasons(deviceId!) });
    },
    onError: (err: any) => toast.error(err.message),
  });

  return {
    activeSeason: activeSeasonQuery.data || null,
    history: seasonHistoryQuery.data || [],
    isLoading: activeSeasonQuery.isLoading || seasonHistoryQuery.isLoading || createMutation.isPending || updateMutation.isPending,
    createSeason: (name: string, plantType: string, description: string, recipeId?: string) =>
      createMutation.mutateAsync({ name, plant_type: plantType, description, recipe_id: recipeId }),
    updateSeason: (name: string, plantType: string, description: string) =>
      updateMutation.mutateAsync({ name, plant_type: plantType, description }),
    endSeason: () => endMutation.mutateAsync(),
    deleteSeason: (seasonId: string) => deleteMutation.mutateAsync(seasonId),
  };
};

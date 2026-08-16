// src/hooks/useCropSeason.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { CropSeason } from '../types/models';
import { useDeviceStore } from '../store/useDeviceStore';
import { httpFetch } from '../platform/http';
import toast from 'react-hot-toast';

export const useCropSeason = () => {
  const queryClient = useQueryClient();
  const deviceId = useDeviceStore((s) => s.deviceId);
  const settings = useDeviceStore((s) => s.settings);

  const baseUrl = `${settings?.backend_url}/api/devices/${deviceId}/seasons`;
  const headers = {
    'Content-Type': 'application/json',
    'X-API-Key': settings?.api_key || '',
  };

  // 1. Query lấy Active Season
  const activeSeasonQuery = useQuery<CropSeason | null>({
    queryKey: ['seasons', deviceId, 'active'],
    queryFn: async () => {
      const res = await httpFetch(`${baseUrl}/active`, { headers });
      if (!res.ok) return null;
      const json = await res.json();
      return json?.data || null;
    },
    enabled: Boolean(deviceId && settings?.backend_url),
  });

  // 2. Query lịch sử các mùa vụ
  const seasonHistoryQuery = useQuery<CropSeason[]>({
    queryKey: ['seasons', deviceId, 'history'],
    queryFn: async () => {
      const res = await httpFetch(baseUrl, { headers });
      if (!res.ok) return [];
      const json = await res.json();
      return json?.data || [];
    },
    enabled: Boolean(deviceId && settings?.backend_url),
  });

  // 3. Mutation Tạo Mùa Vụ Mới
  const createMutation = useMutation({
    mutationFn: async (payload: { name: string; plant_type: string; description: string }) => {
      const res = await httpFetch(baseUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error('Không thể tạo mùa vụ mới');
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã tạo mùa vụ mới!');
      queryClient.invalidateQueries({ queryKey: ['seasons', deviceId] });
      queryClient.invalidateQueries({ queryKey: ['seasons-list', deviceId] });
    },
    onError: (err: any) => toast.error(err.message),
  });

  // 4. Mutation Cập nhật Mùa Vụ Đang Chạy
  const updateMutation = useMutation({
    mutationFn: async (payload: { name: string; plant_type: string; description: string }) => {
      const res = await httpFetch(`${baseUrl}/active`, {
        method: 'PUT',
        headers,
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error('Không thể cập nhật thông tin mùa vụ');
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã cập nhật thông tin mùa vụ!');
      queryClient.invalidateQueries({ queryKey: ['seasons', deviceId] });
      queryClient.invalidateQueries({ queryKey: ['seasons-list', deviceId] });
    },
    onError: (err: any) => toast.error(err.message),
  });

  // 5. Mutation Kết thúc Mùa Vụ
  const endMutation = useMutation({
    mutationFn: async () => {
      const res = await httpFetch(`${baseUrl}/active/end`, { method: 'PUT', headers });
      if (!res.ok) throw new Error('Không thể kết thúc mùa vụ');
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã kết thúc mùa vụ!');
      queryClient.invalidateQueries({ queryKey: ['seasons', deviceId] });
      queryClient.invalidateQueries({ queryKey: ['seasons-list', deviceId] });
    },
    onError: (err: any) => toast.error(err.message),
  });

  return {
    activeSeason: activeSeasonQuery.data || null,
    history: seasonHistoryQuery.data || [],
    isLoading: activeSeasonQuery.isLoading || seasonHistoryQuery.isLoading || updateMutation.isPending,
    createSeason: (name: string, plantType: string, description: string) =>
      createMutation.mutateAsync({ name, plant_type: plantType, description }),
    updateSeason: (name: string, plantType: string, description: string) =>
      updateMutation.mutateAsync({ name, plant_type: plantType, description }),
    endSeason: () => endMutation.mutateAsync(),
  };
};

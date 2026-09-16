import { useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPost } from '../lib/apiClient';
import { queryKeys } from '../api/queryKeys';

export type BackupArtifact = {
  format: string;
  schema_version: number;
  created_at: string;
  producer: { application: string; version: string };
  scope: 'station' | 'device' | 'controller';
  source: {
    station_id: string | null;
    device_ids: string[];
    controller_ids: string[];
  };
  configuration: Record<string, unknown>;
  integrity: { algorithm: string; digest: string };
};

export type RestorePreview = {
  status: 'validated';
  target_device_id: string;
  source_device_id: string;
  additions: string[];
  changes: string[];
  unchanged: string[];
  rejected: string[];
  warnings: string[];
  apply_permitted: boolean;
};

export function useConfigBackup(deviceId: string | null) {
  const queryClient = useQueryClient();

  const exportMutation = useMutation({
    mutationFn: async () => {
      if (!deviceId) throw new Error('Chưa chọn thiết bị');
      return apiGet<BackupArtifact>(`/devices/${deviceId}/admin/backup`);
    },
  });

  const previewMutation = useMutation({
    mutationFn: async (artifact: BackupArtifact) => {
      if (!deviceId) throw new Error('Chưa chọn thiết bị');
      return apiPost<RestorePreview>(`/devices/${deviceId}/admin/restore/validate`, artifact);
    },
  });

  const restoreMutation = useMutation({
    mutationFn: async (artifact: BackupArtifact) => {
      if (!deviceId) throw new Error('Chưa chọn thiết bị');
      return apiPost<{ status: string; device_id: string }>(
        `/devices/${deviceId}/admin/restore`,
        artifact,
      );
    },
    onSuccess: () => {
      if (!deviceId) return;
      void queryClient.invalidateQueries({ queryKey: ['device-config', deviceId] });
      void queryClient.invalidateQueries({ queryKey: queryKeys.recipeStatus(deviceId) });
      void queryClient.invalidateQueries({ queryKey: ['device-health', deviceId] });
    },
  });

  return { exportMutation, previewMutation, restoreMutation };
}

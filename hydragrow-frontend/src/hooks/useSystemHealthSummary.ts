import { useQuery } from '@tanstack/react-query';
import { apiGet } from '../lib/apiClient';

export interface SystemHealthSummary {
  window_seconds: number;
  ec_dosing_count: number;
  ph_dosing_count: number;
  water_operation_count: number;
  warning_count: number;
  critical_count: number;
  latest_ph_dosing_at: number | null;
}

export function useSystemHealthSummary(deviceId: string) {
  return useQuery({
    queryKey: ['system-health-summary', deviceId],
    queryFn: () =>
      apiGet<{ status: string; data: SystemHealthSummary }>(`/devices/${deviceId}/health-summary`).then(
        (r) => r.data,
      ),
    enabled: !!deviceId,
    refetchInterval: 30_000,
  });
}

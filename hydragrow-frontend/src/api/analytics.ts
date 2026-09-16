import { apiGet, apiPut } from '../lib/apiClient';
export interface DeviceHealthMetrics {
  free_heap_bytes: number | null;
  wifi_rssi_dbm: number | null;
  uptime_seconds: number | null;
  backend_process_cpu_percent: number | null;
  last_updated_at: string | null;
}

export const analyticsApi = {
  health: (deviceId: string, signal?: AbortSignal) =>
    apiGet<DeviceHealthMetrics>(`/devices/${deviceId}/analytics/health`, { signal }),
  updateWeeklyReportPreference: (preferences: Record<string, unknown>) =>
    apiPut('/admin/me/preferences', { preferences }),
};

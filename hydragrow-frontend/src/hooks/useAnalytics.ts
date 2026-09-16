import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { analyticsApi, type DeviceHealthMetrics } from '../api/analytics';
import { queryKeys } from '../api/queryKeys';

export function useAnalyticsHealth(deviceId: string | null) {
  return useQuery<DeviceHealthMetrics>({
    queryKey: deviceId ? queryKeys.analyticsHealth(deviceId) : ['device-health', null],
    queryFn: ({ signal }) => analyticsApi.health(deviceId!, signal),
    enabled: Boolean(deviceId),
    refetchInterval: 15_000,
  });
}

export function useUpdateWeeklyReportPreference() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (preferences: Record<string, unknown>) =>
      analyticsApi.updateWeeklyReportPreference(preferences),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['whoami'] });
    },
  });
}

import { useQuery } from '@tanstack/react-query';
import { apiGet } from '../lib/apiClient';
import { queryKeys } from '../api/queryKeys';
import type { OperationalState } from '../types/models';

export interface DashboardFleetStation {
  device_id: string;
  label?: string | null;
  is_online?: boolean | null;
  last_seen?: string | null;
  operational_state: OperationalState;
  crop?: string | null;
  ec_latest?: number | null;
  ph_latest?: number | null;
  warning_count: number;
  warning_count_known: boolean;
  telemetry_freshness: 'UNKNOWN' | 'FRESH' | 'STALE';
  telemetry_observed_at: string | null;
  telemetry_received_at: string | null;
  ec_quality: 'UNKNOWN' | 'VALID' | 'STALE' | 'ERROR' | 'INVALID' | null;
  ec_observed_at?: string | null;
  ph_quality: 'UNKNOWN' | 'VALID' | 'STALE' | 'ERROR' | 'INVALID' | null;
  ph_observed_at?: string | null;
}

interface FleetSummaryResponse {
  data: DashboardFleetStation[];
}

export interface FleetComparison {
  data: Array<Pick<DashboardFleetStation, 'device_id' | 'label' | 'crop' | 'ec_latest' | 'ph_latest' | 'telemetry_freshness' | 'telemetry_observed_at' | 'ec_quality' | 'ec_observed_at' | 'ph_quality' | 'ph_observed_at'>>;
  comparison: { same_crop: boolean; normalized: boolean };
}

export function useDashboardFleet() {
  const query = useQuery({
    queryKey: queryKeys.fleetSummary(),
    queryFn: async () => {
      const response = await apiGet<FleetSummaryResponse>('/fleet/summary');
      if (!response || !Array.isArray(response.data)) {
        throw new Error('Fleet summary response is invalid');
      }
      return response;
    },
    staleTime: 5_000,
    refetchOnWindowFocus: false,
    retry: false,
  });

  return {
    stations: query.data?.data ?? [],
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    error: query.error,
    refresh: query.refetch,
  };
}

export function useFleetComparison(deviceIds: string[]) {
  const normalizedIds = [...deviceIds].sort();

  return useQuery({
    queryKey: queryKeys.fleetCompare(normalizedIds),
    queryFn: () => apiGet<FleetComparison>(`/fleet/compare?device_ids=${encodeURIComponent(normalizedIds.join(','))}`),
    enabled: normalizedIds.length >= 2,
    staleTime: 5_000,
    refetchOnWindowFocus: false,
    retry: false,
  });
}

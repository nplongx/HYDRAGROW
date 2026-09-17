import { useQuery } from '@tanstack/react-query';
import { useStationContext } from '../contexts/StationContext';
import type { AuthoritativeTelemetrySnapshot } from '../types/models';
import { telemetryApi, normalizeLatestTelemetry } from '../api/telemetry';
import { queryKeys } from '../api/queryKeys';

export const deviceTelemetryQueryKey = (deviceId: string) =>
  queryKeys.telemetry(deviceId);

export function readSnapshot(
  response: { data?: AuthoritativeTelemetrySnapshot },
  expectedDeviceId?: string,
): AuthoritativeTelemetrySnapshot {
  const snapshot = response?.data;
  if (!snapshot || typeof snapshot !== 'object') {
    throw new Error('Telemetry response is unavailable');
  }
  return normalizeLatestTelemetry(response, expectedDeviceId ?? snapshot.device_id);
}

export function useDeviceTelemetry(deviceIdOverride?: string | null) {
  const { selectedDeviceId } = useStationContext();
  const deviceId = deviceIdOverride ?? selectedDeviceId;

  return useQuery({
    queryKey: deviceId ? deviceTelemetryQueryKey(deviceId) : ['device-telemetry', null],
    queryFn: ({ signal }) => telemetryApi.latest(deviceId!, signal),
    enabled: Boolean(deviceId),
    // A 503 here means the backend has no authoritative current state yet.
    // Retrying immediately only amplifies load and can trip the API limiter.
    retry: false,
    staleTime: 5_000,
    refetchOnWindowFocus: false,
  });
}

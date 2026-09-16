import { useQuery } from "@tanstack/react-query";
import { useOptionalStationContext } from "../contexts/StationContext";
import type { UnifiedDeviceConfig } from "../types/models";
import { configApi, type UnifiedConfigResponse } from "../api/config";
import { queryKeys } from "../api/queryKeys";

export const deviceConfigQueryKey = (deviceId: string) =>
  queryKeys.config(deviceId);

export function mergeUnifiedDeviceConfig(
  response: Partial<UnifiedConfigResponse>,
  expectedDeviceId?: string,
): Partial<UnifiedDeviceConfig> {
  const merged = {
    ...response.device_config,
    ...response.water_config,
    ...response.safety_config,
    ...response.sensor_calibration,
    ...response.dosing_calibration,
  };
  if (
    expectedDeviceId &&
    merged.device_id &&
    merged.device_id !== expectedDeviceId
  ) {
    throw new Error("Config response belongs to a different device");
  }
  return merged;
}

export function useDeviceConfig(deviceIdOverride?: string | null) {
  const stationContext = useOptionalStationContext();
  const selectedDeviceId = stationContext?.selectedDeviceId ?? null;
  const deviceId = deviceIdOverride ?? selectedDeviceId;

  return useQuery({
    queryKey: deviceId
      ? deviceConfigQueryKey(deviceId)
      : ["device-config", null],
    queryFn: ({ signal }) => configApi.get(deviceId!, signal),
    enabled: Boolean(deviceId),
  });
}

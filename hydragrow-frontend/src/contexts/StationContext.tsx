import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { apiGet } from "../lib/apiClient";
import { configApi } from "../api/config";
import { queryKeys } from "../api/queryKeys";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { ConfigurationSyncState } from "../types/models";
import { getItem, removeItem, setItem } from "../platform/storage";
import { loadAppSettings } from "../platform/settings";
import type { OwnedDevice } from "../types/models";

export const STATION_SELECTION_STORAGE_KEY = "hydragrow_selected_device_id";

export type StationSelectionStatus =
  | "LoadingSelection"
  | "NoSelection"
  | "Selected"
  | "InvalidSelection"
  | "Unavailable"
  | "PermissionDenied";

export interface StationContextValue {
  status: StationSelectionStatus;
  selectedDeviceId: string | null;
  selectedDevice: OwnedDevice | null;
  availableDevices: OwnedDevice[];
  error: Error | null;
  selectDevice: (deviceId: string) => void;
  switchDevice: (deviceId: string) => void;
  clearSelection: () => void;
  refreshAvailableDevices: () => Promise<OwnedDevice[] | null>;
  configurationSync?: {
    status: ConfigurationSyncState | null;
    version: number | null;
    controller: ConfigurationSyncState | null;
    sensor: ConfigurationSyncState | null;
    error: string | null;
    isLoading: boolean;
    refresh: () => Promise<unknown>;
  };
}

const StationContext = createContext<StationContextValue | null>(null);

type ApiError = Error & { status?: number };

function isPermissionError(error: unknown): boolean {
  return (
    (error as ApiError | null)?.status === 401 ||
    (error as ApiError | null)?.status === 403
  );
}

export function StationProvider({ children }: { children: ReactNode }) {
  const [availableDevices, setAvailableDevices] = useState<OwnedDevice[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null);
  const [status, setStatus] =
    useState<StationSelectionStatus>("LoadingSelection");
  const [error, setError] = useState<Error | null>(null);
  const initialLoadDoneRef = useRef(false);
  const requestSequenceRef = useRef(0);
  const queryClient = useQueryClient();
  const configurationSyncQuery = useQuery({
    queryKey: selectedDeviceId
      ? queryKeys.configSync(selectedDeviceId)
      : ["device-config-sync", null],
    queryFn: ({ signal }) => configApi.syncStatus(selectedDeviceId!, signal),
    enabled: Boolean(selectedDeviceId),
    staleTime: 0,
    refetchInterval: 2000,
  });


  const persistSelection = useCallback((deviceId: string | null) => {
    const operation =
      deviceId === null
        ? removeItem(STATION_SELECTION_STORAGE_KEY)
        : setItem(STATION_SELECTION_STORAGE_KEY, deviceId);

    void operation.catch(() => {
      // Selection state remains authoritative even when persistence is unavailable.
    });
  }, []);

  const applyAvailableDevices = useCallback((devices: OwnedDevice[]) => {
    setAvailableDevices(devices);
    setError(null);
  }, []);

  const refreshAvailableDevices = useCallback(async (): Promise<
    OwnedDevice[] | null
  > => {
    const requestId = ++requestSequenceRef.current;
    try {
      const devices = await apiGet<OwnedDevice[]>("/devices");
      if (requestId !== requestSequenceRef.current) return null;

      const nextDevices = Array.isArray(devices) ? devices : [];
      applyAvailableDevices(nextDevices);
      initialLoadDoneRef.current = true;
      return nextDevices;
    } catch (caught) {
      if (requestId !== requestSequenceRef.current) return null;

      const nextError =
        caught instanceof Error
          ? caught
          : new Error("Không thể tải danh sách trạm.");
      setError(nextError);

      if (!initialLoadDoneRef.current) {
        setStatus(
          isPermissionError(nextError) ? "PermissionDenied" : "Unavailable",
        );
      }
      // Preserve the last authoritative list and current valid selection on refresh failure.
      return null;
    }
  }, [applyAvailableDevices]);

  useEffect(() => {
    let cancelled = false;

    const initialize = async () => {
      setStatus("LoadingSelection");
      const devices = await refreshAvailableDevices();
      if (cancelled) return;

      let persisted = await getItem<string>(STATION_SELECTION_STORAGE_KEY);
      if (!devices) {
        const configured = (await loadAppSettings())?.device_id?.trim() || null;
        if (configured) {
          persistSelection(configured);
          setSelectedDeviceId(configured);
          setStatus("Selected");
        }
        return;
      }

      if (!persisted) {
        const legacySettings = await loadAppSettings();
        persisted = legacySettings?.device_id?.trim() || null;
        if (persisted) persistSelection(persisted);
      }

      if (cancelled) return;
      setSelectedDeviceId((currentId) => {
        const candidate = persisted || currentId;
        if (!candidate) {
          setStatus("NoSelection");
          return null;
        }
        if (devices.some((device) => device.device_id === candidate)) {
          setStatus("Selected");
          return candidate;
        }
        setStatus("InvalidSelection");
        return candidate;
      });
    };

    void initialize();
    return () => {
      cancelled = true;
    };
  }, [persistSelection, refreshAvailableDevices]);

  useEffect(() => {
    if (!initialLoadDoneRef.current) return;
    if (!selectedDeviceId) {
      setStatus("NoSelection");
      return;
    }
    setStatus(
      availableDevices.some((device) => device.device_id === selectedDeviceId)
        ? "Selected"
        : "InvalidSelection",
    );
  }, [availableDevices, selectedDeviceId]);

  const selectDevice = useCallback(
    (deviceId: string) => {
      const exists = availableDevices.some(
        (device) => device.device_id === deviceId,
      );
      if (!exists) {
        setError(
          new Error(
            `Thiết bị không thuộc danh sách trạm có thể truy cập: ${deviceId}`,
          ),
        );
        return;
      }

      setError(null);
      setSelectedDeviceId(deviceId);
      setStatus("Selected");
      persistSelection(deviceId);
    },
    [availableDevices, persistSelection],
  );

  const clearSelection = useCallback(() => {
    setSelectedDeviceId(null);
    setStatus("NoSelection");
    setError(null);
    persistSelection(null);
  }, [persistSelection]);

  const selectedDevice = useMemo(
    () =>
      availableDevices.find(
        (device) => device.device_id === selectedDeviceId,
      ) ?? null,
    [availableDevices, selectedDeviceId],
  );

  const configurationSync = useMemo(() => ({
    status: configurationSyncQuery.data?.overall_state ?? null,
    version: configurationSyncQuery.data?.config_version ?? null,
    controller: configurationSyncQuery.data?.controller?.state ?? null,
    sensor: configurationSyncQuery.data?.sensor?.state ?? null,
    error: configurationSyncQuery.data?.last_error ?? null,
    isLoading: configurationSyncQuery.isLoading,
    refresh: () => queryClient.invalidateQueries({
      queryKey: selectedDeviceId ? queryKeys.configSync(selectedDeviceId) : ["device-config-sync", null],
    }),
  }), [configurationSyncQuery.data, configurationSyncQuery.isLoading, queryClient, selectedDeviceId]);

  const value = useMemo<StationContextValue>(
    () => ({
      status,
      selectedDeviceId,
      selectedDevice,
      availableDevices,
      error,
      selectDevice,
      switchDevice: selectDevice,
      clearSelection,
      refreshAvailableDevices,
      configurationSync,
    }),
    [
      status,
      selectedDeviceId,
      selectedDevice,
      availableDevices,
      error,
      selectDevice,
      clearSelection,
      refreshAvailableDevices,
      configurationSync,
    ],
  );

  return (
    <StationContext.Provider value={value}>{children}</StationContext.Provider>
  );
}

export function useStationContext(): StationContextValue {
  const context = useContext(StationContext);
  if (!context)
    throw new Error("useStationContext must be used within StationProvider");
  return context;
}

export function useOptionalStationContext(): StationContextValue | null {
  return useContext(StationContext);
}

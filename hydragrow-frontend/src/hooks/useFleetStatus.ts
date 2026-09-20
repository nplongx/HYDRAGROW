import type { OperationalState } from '../types/models';
import { useDashboardFleet } from './useDashboardFleet';

export interface FleetDevice {
  device_id: string;
  label: string | null;
  is_online?: boolean;
  firmware_version?: string;
  last_seen?: string;
  operational_state?: OperationalState;
}

interface FleetStatusState {
  devices: FleetDevice[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useFleetStatus(): FleetStatusState {
  const query = useDashboardFleet();
  const devices: FleetDevice[] = query.stations.map((station) => ({
    device_id: station.device_id,
    label: station.label ?? null,
    is_online: station.is_online ?? undefined,
    last_seen: station.last_seen ?? undefined,
    operational_state: station.operational_state,
  }));

  return {
    devices,
    loading: query.isLoading,
    error: query.error instanceof Error ? query.error.message : query.error ? String(query.error) : null,
    refresh: () => void query.refresh(),
  };
}

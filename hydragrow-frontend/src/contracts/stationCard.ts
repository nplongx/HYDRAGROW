import type { OperationalState } from '../types/models';

export type StationCardConnection = 'ONLINE' | 'OFFLINE' | 'STALE' | 'UNKNOWN';
export type StationCardTelemetryQuality = 'CURRENT' | 'STALE' | 'UNKNOWN';

export interface StationCardState {
  operational: OperationalState;
  connection: StationCardConnection;
  telemetry: {
    freshness: OperationalState['freshness'];
    quality: StationCardTelemetryQuality;
  };
  warning: {
    count: number;
    known: boolean;
  };
}

/**
 * Normalize station-card display state from the authoritative operational state.
 * `is_online` is intentionally not used as an independent source of truth.
 */
export function normalizeStationCardState(input: {
  operational_state?: OperationalState | null;
  warning_count?: number | null;
  warning_count_known?: boolean | null;
}): StationCardState {
  const operational = input.operational_state;
  const freshness = operational?.freshness ?? 'UNKNOWN';
  const contact = operational?.contact ?? 'UNKNOWN';

  let connection: StationCardConnection = 'UNKNOWN';
  if (contact === 'CONTACTED' && freshness === 'FRESH') connection = 'ONLINE';
  else if (contact === 'NOT_CONTACTED') connection = 'OFFLINE';
  else if (freshness === 'STALE') connection = 'STALE';

  return {
    operational: operational ?? {
      contact: 'UNKNOWN',
      freshness: 'UNKNOWN',
      readiness: 'UNKNOWN',
      actuator: 'UNKNOWN',
      classified_at: null,
      observed_at: null,
    },
    connection,
    telemetry: {
      freshness,
      quality: freshness === 'FRESH' ? 'CURRENT' : freshness === 'STALE' ? 'STALE' : 'UNKNOWN',
    },
    warning: {
      count: input.warning_count ?? 0,
      known: input.warning_count_known !== false,
    },
  };
}

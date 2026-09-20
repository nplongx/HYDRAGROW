import { describe, expect, it } from 'vitest';
import { normalizeStationCardState } from './stationCard';

const operational = {
  contact: 'CONTACTED',
  freshness: 'FRESH',
  readiness: 'READY',
  actuator: 'KNOWN',
  classified_at: null,
  observed_at: null,
} as const;

describe('normalizeStationCardState', () => {
  it('uses authoritative contact and freshness for online state', () => {
    expect(normalizeStationCardState({ operational_state: operational }).connection).toBe('ONLINE');
    expect(
      normalizeStationCardState({
        operational_state: { ...operational, freshness: 'STALE' },
      }).connection,
    ).toBe('STALE');
  });

  it('keeps unknown operational state unknown instead of inferring offline', () => {
    const state = normalizeStationCardState({ operational_state: undefined });
    expect(state.connection).toBe('UNKNOWN');
    expect(state.telemetry.quality).toBe('UNKNOWN');
  });

  it('preserves unknown warning counts', () => {
    const state = normalizeStationCardState({
      operational_state: operational,
      warning_count: 0,
      warning_count_known: false,
    });
    expect(state.warning).toEqual({ count: 0, known: false });
  });

  it('does not treat a stale station as current telemetry', () => {
    const state = normalizeStationCardState({
      operational_state: { ...operational, freshness: 'STALE' },
    });
    expect(state.telemetry).toEqual({ freshness: 'STALE', quality: 'STALE' });
  });

  it('does not use the legacy is_online field as state authority', () => {
    const state = normalizeStationCardState({ operational_state: { ...operational, contact: 'UNKNOWN' } });
    expect(state.connection).toBe('UNKNOWN');
  });
});

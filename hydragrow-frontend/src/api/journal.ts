import { apiGet, apiPut } from '../lib/apiClient';

export interface JournalEvent {
  id: number;
  device_id: string;
  level: JournalLevel;
  category: JournalCategory;
  title: string;
  message: string;
  reason?: string;
  metadata?: Record<string, unknown>;
  timestamp: number;
  resolved_at?: string | null;
  occurred_at?: string;
  received_at?: string;
  event_type?: string;
  source?: string;
  actor?: { kind?: string; id?: string };
  reason_code?: string;
  correlation?: Record<string, unknown>;
}

export type JournalLevel = 'debug' | 'info' | 'warning' | 'error' | 'critical';
export type JournalCategory = 'system' | 'dosing' | 'water' | 'calibration' | 'sensor' | 'alert' | 'user_action';

type JournalWireEvent = Omit<JournalEvent, 'level' | 'category' | 'id'> & { id: number | string; level: string; category: string };

export function adaptJournalEvent(event: JournalWireEvent): JournalEvent {
  const level = event.level.toLowerCase();
  const category = event.category.toLowerCase();
  return {
    ...event,
    level: (['debug', 'info', 'warning', 'error', 'critical'].includes(level) ? level : 'info') as JournalLevel,
    category: (['system', 'dosing', 'water', 'calibration', 'sensor', 'alert', 'user_action'].includes(category) ? category : 'system') as JournalCategory,
    id: Number(event.id),
  };
}

export type JournalListResponse = {
  data?: JournalEvent[];
  next_cursor?: string | null;
};

export const journalApi = {
  list: async (deviceId: string, query: string, signal?: AbortSignal) => {
    const response = await apiGet<{ data?: JournalWireEvent[]; next_cursor?: string | null }>(
      `/devices/${encodeURIComponent(deviceId)}/events?${query}`, { signal });
    return { data: (response.data ?? []).map(adaptJournalEvent), next_cursor: response.next_cursor ?? null };
  },
  acknowledge: (deviceId: string, eventId: string, resolved: boolean) =>
    apiPut(`/devices/${encodeURIComponent(deviceId)}/events/${encodeURIComponent(eventId)}/acknowledge`, { resolved }),
};

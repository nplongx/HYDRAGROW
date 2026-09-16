import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { journalApi } from '../api/journal';
import { useStationContext } from '../contexts/StationContext';
import type { JournalEvent } from '../api/journal';

export const systemEventsQueryKey = (deviceId: string) =>
  ['system-events', deviceId, 'recent'] as const;

export function useSystemEvents(deviceIdOverride?: string | null, limit = 50) {
  const { selectedDeviceId } = useStationContext();
  const deviceId = deviceIdOverride ?? selectedDeviceId;

  return useQuery({
    queryKey: deviceId ? systemEventsQueryKey(deviceId) : ['system-events', null, 'recent'],
    queryFn: async () => {
      const response = await journalApi.list(
        deviceId!,
        new URLSearchParams({ limit: String(limit) }).toString(),
      );
      return (response.data ?? []).map((event) => ({
        id: event.id,
        device_id: event.device_id,
        level: event.level,
        category: event.category,
        message: event.message,
        timestamp_ms: event.timestamp > 1e12 ? event.timestamp : event.timestamp * 1000,
        metadata: event.metadata,
      }));
    },
    enabled: Boolean(deviceId),
  });
}

export type JournalFilters = {
  deviceId: string | null;
  category?: string;
  level?: string;
  unresolved?: boolean;
  search?: string;
  from?: string;
  to?: string;
};

export type JournalPage = {
  data: JournalEvent[];
  next_cursor?: string | null;
};

export const journalQueryKey = (filters: JournalFilters) =>
  [
    'journal',
    filters.deviceId,
    filters.category ?? 'all',
    filters.level ?? 'all',
    filters.unresolved ? 'unresolved' : 'all',
    filters.search ?? '',
    filters.from ?? '',
    filters.to ?? '',
  ] as const;

function buildJournalQuery(filters: JournalFilters, cursor?: string | null) {
  const params = new URLSearchParams({ limit: '200' });
  if (filters.category) params.set('category', filters.category);
  if (filters.level) params.set('level', filters.level);
  if (filters.unresolved) params.set('unresolved', 'true');
  if (filters.search?.trim()) params.set('search', filters.search.trim());
  if (filters.from) params.set('from', filters.from);
  if (filters.to) params.set('to', filters.to);
  if (cursor) params.set('cursor', cursor);
  return params.toString();
}

export function useJournalEvents(filters: JournalFilters) {
  return useInfiniteQuery({
    queryKey: journalQueryKey(filters),
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }) => {
      if (!filters.deviceId) return { data: [] } satisfies JournalPage;
      const response = await journalApi.list(filters.deviceId, buildJournalQuery(filters, pageParam));
      return { data: response.data ?? [], next_cursor: response.next_cursor ?? null };
    },
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    enabled: Boolean(filters.deviceId),
  });
}

export function useAcknowledgeJournalEvent(filters: JournalFilters) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ eventId, resolved }: { eventId: string; resolved: boolean }) => {
      if (!filters.deviceId) throw new Error('Chưa chọn thiết bị.');
      return journalApi.acknowledge(filters.deviceId, eventId, resolved);
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: journalQueryKey(filters) }),
  });
}

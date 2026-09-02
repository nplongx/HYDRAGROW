import { useQuery } from '@tanstack/react-query';
import { apiGet } from '../lib/apiClient';
import type { SystemEvent } from '../components/logs/EventLogCard';

export function useCycleTimeline(deviceId: string, cycleId: string | null) {
  return useQuery({
    queryKey: ['cycle-timeline', deviceId, cycleId],
    queryFn: () =>
      apiGet<{ status: string; data: SystemEvent[] }>(`/devices/${deviceId}/events/cycle/${cycleId}`).then(
        (r) => r.data,
      ),
    enabled: !!deviceId && !!cycleId,
  });
}

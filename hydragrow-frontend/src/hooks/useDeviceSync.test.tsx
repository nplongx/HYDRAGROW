import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useDeviceSync } from './useDeviceSync';

vi.mock('../platform/settings', () => ({
  loadAppSettings: () => Promise.resolve({ backend_url: 'http://localhost:8080', api_key: 'k', device_id: 'device-1' }),
  saveWebSettings: vi.fn(),
}));

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({
    status: 'Selected', selectedDeviceId: 'device-1', selectedDevice: null,
    availableDevices: [], error: null, selectDevice: vi.fn(), switchDevice: vi.fn(),
    clearSelection: vi.fn(), refreshAvailableDevices: vi.fn(),
  }),
}));

class FakeWebSocket {
  static instances: FakeWebSocket[] = [];
  static OPEN = 1;
  url: string;
  readyState = 0;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  send = vi.fn();
  close = vi.fn();
  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }
}

function Harness() {
  useDeviceSync();
  return null;
}

beforeEach(() => {
  FakeWebSocket.instances = [];
  vi.stubGlobal('WebSocket', FakeWebSocket);
});

describe('useDeviceSync WebSocket alert handling', () => {
  it('refetches authoritative device queries after reconnect', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const refetchQueries = vi.spyOn(client, 'refetchQueries').mockResolvedValue(undefined);
    render(<QueryClientProvider client={client}><Harness /></QueryClientProvider>);
    await waitFor(() => expect(FakeWebSocket.instances.length).toBeGreaterThan(0));
    FakeWebSocket.instances[0].onopen?.();
    await waitFor(() => expect(refetchQueries).toHaveBeenCalled());
    expect(refetchQueries).toHaveBeenCalledWith({ queryKey: ['device-telemetry', 'device-1'], type: 'active' });
    expect(refetchQueries).toHaveBeenCalledWith({ queryKey: ['device-config', 'device-1'], type: 'active' });
    expect(refetchQueries).toHaveBeenCalledWith({ queryKey: ['device-health', 'device-1'], type: 'active' });
  });

  it('does not overwrite telemetry cache with an older realtime snapshot', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const current = { device_id: 'device-1', observed_at: '2026-09-16T10:00:00Z', received_at: '2026-09-16T10:00:01Z', availability: 'ONLINE', axes: [], operational_state: { contact: 'CONTACTED', freshness: 'FRESH', readiness: 'READY', actuator: 'KNOWN', classified_at: null, observed_at: '2026-09-16T10:00:00Z' }, actuator_contradictory: false } as any;
    client.setQueryData(['device-telemetry', 'device-1'], current);
    render(<QueryClientProvider client={client}><Harness /></QueryClientProvider>);
    await waitFor(() => expect(FakeWebSocket.instances.length).toBeGreaterThan(0));
    FakeWebSocket.instances[0].onmessage?.({ data: JSON.stringify({ type: 'telemetry_snapshot', payload: { ...current, observed_at: '2026-09-15T10:00:00Z' } }) });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(client.getQueryData(['device-telemetry', 'device-1'])).toEqual(current);
  });

  it('đánh dấu stale cache system-events của thiết bị khi nhận cảnh báo mới', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(['system-events', 'device-1', 'recent'], []);
    client.setQueryData(['journal', 'device-1', 'all', 'all', 'all', '', '', ''], []);

    render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    await waitFor(() => expect(FakeWebSocket.instances.length).toBeGreaterThan(0));
    const socket = FakeWebSocket.instances[0];
    socket.onopen?.();
    client.setQueryData(['system-events', 'device-1', 'recent'], []);
    client.setQueryData(['journal', 'device-1', 'all', 'all', 'all', '', '', ''], []);

    expect(client.getQueryState(['system-events', 'device-1', 'recent'])?.isInvalidated).toBe(false);
    expect(client.getQueryState(['journal', 'device-1', 'all', 'all', 'all', '', '', ''])?.isInvalidated).toBe(false);

    socket.onmessage?.({
      data: JSON.stringify({
        type: 'alert',
        payload: {
          level: 'critical',
          category: 'alert',
          title: 'Tank Low',
          message: 'Tank A is low',
          device_id: 'device-1',
          timestamp_ms: Date.now(),
        },
      }),
    });

    await waitFor(() =>
      expect(client.getQueryState(['system-events', 'device-1', 'recent'])?.isInvalidated).toBe(true)
    );
    expect(client.getQueryState(['journal', 'device-1', 'all', 'all', 'all', '', '', ''])?.isInvalidated).toBe(true);
  });

  it('không đánh dấu stale cache của thiết bị khác', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(['system-events', 'some-other-device', 'recent'], []);

    render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    await waitFor(() => expect(FakeWebSocket.instances.length).toBeGreaterThan(0));
    const socket = FakeWebSocket.instances[0];
    socket.onopen?.();

    socket.onmessage?.({
      data: JSON.stringify({
        type: 'alert',
          payload: { device_id: 'device-2', level: 'critical', title: 'Tank Low', message: 'Tank A is low' },
      }),
    });

    // give the microtask queue a chance to run before asserting the negative
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(client.getQueryState(['system-events', 'some-other-device', 'recent'])?.isInvalidated).toBe(
      false
    );
  });
});

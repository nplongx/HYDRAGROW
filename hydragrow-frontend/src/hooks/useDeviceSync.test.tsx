import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useDeviceSync } from './useDeviceSync';
import { useDeviceStore } from '../store/useDeviceStore';
import { httpFetch } from '../platform/http';
import type { AppSettings } from '../types/models';

vi.mock('../platform/http', () => ({ httpFetch: vi.fn() }));

vi.mock('../platform/settings', () => ({
  isTauriRuntime: () => false,
  hasRequiredRemoteConfig: () => true,
  loadAppSettings: () => Promise.resolve(null),
  saveWebSettings: vi.fn(),
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
  vi.mocked(httpFetch).mockResolvedValue({ ok: false } as Response);
  useDeviceStore.setState({
    deviceId: 'device-1',
    settings: { backend_url: 'http://localhost:8080', api_key: 'k', device_id: 'device-1' } as AppSettings,
  });
});

describe('useDeviceSync WebSocket alert handling', () => {
  it('đánh dấu stale cache system-events của thiết bị khi nhận cảnh báo mới', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(['system-events', 'device-1', 'all'], []);

    render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    await waitFor(() => expect(FakeWebSocket.instances.length).toBeGreaterThan(0));
    const socket = FakeWebSocket.instances[0];
    socket.onopen?.();

    expect(client.getQueryState(['system-events', 'device-1', 'all'])?.isInvalidated).toBe(false);

    socket.onmessage?.({
      data: JSON.stringify({
        type: 'alert',
        payload: {
          level: 'critical',
          category: 'alert',
          title: 'Tank Low',
          message: 'Tank A is low',
          device_id: 'device-1',
          timestamp: Date.now(),
        },
      }),
    });

    await waitFor(() =>
      expect(client.getQueryState(['system-events', 'device-1', 'all'])?.isInvalidated).toBe(true)
    );
  });

  it('không đánh dấu stale cache của thiết bị khác', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(['system-events', 'some-other-device', 'all'], []);

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
        payload: { level: 'critical', title: 'Tank Low', message: 'Tank A is low' },
      }),
    });

    // give the microtask queue a chance to run before asserting the negative
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(client.getQueryState(['system-events', 'some-other-device', 'all'])?.isInvalidated).toBe(
      false
    );
  });
});

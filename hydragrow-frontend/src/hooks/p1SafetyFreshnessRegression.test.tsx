import { describe, expect, it, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ensureInterlock, useDeviceControl } from './useDeviceControl';
import { normalizeLatestTelemetry } from '../api/telemetry';
import { useDeviceSync } from './useDeviceSync';
import { queryKeys } from '../api/queryKeys';
import { deviceTelemetryQueryKey } from './useDeviceTelemetry';
import type { AuthoritativeTelemetrySnapshot } from '../types/models';

const station = { selectedDeviceId: 'device-a' as string | null };
    const telemetry: AuthoritativeTelemetrySnapshot = {
  device_id: 'device-a',
  observed_at: '2026-09-20T03:00:00Z',
  received_at: '2026-09-20T03:00:01Z',
  availability: 'ONLINE',
  axes: [],
  operational_state: {
    contact: 'CONTACTED',
    freshness: 'FRESH',
    readiness: 'READY',
    actuator: 'KNOWN',
    classified_at: null,
    observed_at: '2026-09-20T03:00:00Z',
  },
  actuator_contradictory: false,
  actuator: {
    pump_status: {
      water_pump_in: false,
      water_pump_out: false,
      ph_up: false,
      ph_down: false,
      pump_a: false,
      pump_b: false,
      osaka_pump: false,
      mist_valve: false,
      mix_valve: false,
    },
    observed_at: '2026-09-20T03:00:00Z',
    received_at: '2026-09-20T03:00:01Z',
    source: 'controller_sensor',
  },
    };

const { listCommandsMock, sendMock, issuePrivilegedTokenMock } = vi.hoisted(() => ({
  listCommandsMock: vi.fn().mockResolvedValue([]),
  sendMock: vi.fn(),
  issuePrivilegedTokenMock: vi.fn(),
}));

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({
    selectedDeviceId: station.selectedDeviceId,
    selectedDevice: null,
    availableDevices: [],
    status: station.selectedDeviceId ? 'Selected' : 'NoSelection',
    error: null,
    selectDevice: vi.fn(),
    switchDevice: vi.fn(),
    clearSelection: vi.fn(),
    refreshAvailableDevices: vi.fn(),
  }),
}));

vi.mock('./useDeviceTelemetry', () => ({
  deviceTelemetryQueryKey: (deviceId: string) => ['device-telemetry', deviceId],
  useDeviceTelemetry: () => ({ data: telemetry }),
  readSnapshot: (response: { data?: unknown }, expectedDeviceId: string) => {
    const snapshot = response.data as { device_id?: string; observed_at?: string };
    if (snapshot.device_id !== expectedDeviceId) throw new Error('Telemetry response belongs to a different device');
    return response.data;
  },
}));

vi.mock('../api/control', () => ({
  buildControlCommandRequest: (action: string, pumpId: string) => ({
    target: 'all',
    action,
    params: { pump_id: pumpId, duration_sec: null, pwm: null },
    command_metadata: { action, pump_id: pumpId, duration_sec: null, pwm: null, dangerous: false },
  }),
  controlApi: {
    listCommands: listCommandsMock,
    send: sendMock,
    issuePrivilegedToken: issuePrivilegedTokenMock,
  },
}));

vi.mock('../platform/settings', () => ({
  isTauriRuntime: () => false,
  loadAppSettings: () => Promise.resolve({ backend_url: 'http://localhost:8080', api_key: 'k', device_id: 'device-a' }),
}));

vi.mock('../lib/authToken', () => ({ getIdToken: () => 'token' }));
vi.mock('react-hot-toast', () => ({ default: { error: vi.fn(), success: vi.fn() } }));

class FakeWebSocket {
  static instances: FakeWebSocket[] = [];
  static OPEN = 1;
  readyState = 0;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  send = vi.fn();
  close = vi.fn();
  constructor(public url: string) {
    FakeWebSocket.instances.push(this);
  }
}

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  station.selectedDeviceId = 'device-a';
  listCommandsMock.mockReset().mockResolvedValue([]);
  sendMock.mockReset();
  issuePrivilegedTokenMock.mockReset();
  FakeWebSocket.instances = [];
  vi.stubGlobal('WebSocket', FakeWebSocket);
});

describe('P1 safety/freshness regressions', () => {
  it('blocks actuator commands when the interlock actuator state is unknown', async () => {
    await expect(ensureInterlock('PH_UP', 'on', {})).resolves.toContain('KHÔNG XÁC ĐỊNH TRẠNG THÁI AN TOÀN');
    await expect(ensureInterlock('WATER_PUMP_IN', 'on')).resolves.toContain('KHÔNG XÁC ĐỊNH TRẠNG THÁI AN TOÀN');
  });

  it('rejects partial telemetry before it can be treated as authoritative', () => {
    const partial = { data: { ...telemetry, actuator_contradictory: undefined } } as never;
    expect(() => normalizeLatestTelemetry(partial, 'device-a')).toThrow(
      'Telemetry response is missing canonical operational fields',
    );
    expect(() => normalizeLatestTelemetry({ data: { ...telemetry, device_id: 'device-b' } }, 'device-a')).toThrow(
      'Telemetry response belongs to a different device',
    );
  });

  it('drops Station A websocket callbacks after switching to Station B', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidateQueries = vi.spyOn(client, 'invalidateQueries').mockResolvedValue(undefined);
    const { rerender, unmount } = renderHook(() => useDeviceSync(), {
      wrapper: ({ children }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>,
    });
    await waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));

    const stationA = FakeWebSocket.instances[0];
    act(() => stationA.onopen?.());
    invalidateQueries.mockClear();
    station.selectedDeviceId = 'device-b';
    act(() => rerender());
    await waitFor(() => expect(FakeWebSocket.instances).toHaveLength(2));

    act(() => {
      stationA.onmessage?.({
        data: JSON.stringify({
          type: 'telemetry_snapshot',
          payload: { ...telemetry, device_id: 'device-a' },
        }),
      });
    });
    expect(client.getQueryData(deviceTelemetryQueryKey('device-a'))).toBeUndefined();
    expect(invalidateQueries).not.toHaveBeenCalledWith({ queryKey: deviceTelemetryQueryKey('device-a') });
    unmount();
  });

  it('treats an older telemetry snapshot as stale and keeps the newer state', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const current = { ...telemetry, observed_at: '2026-09-20T03:00:00Z' };
    client.setQueryData(deviceTelemetryQueryKey('device-a'), current);
    const invalidateQueries = vi.spyOn(client, 'invalidateQueries').mockResolvedValue(undefined);

    renderHook(() => useDeviceSync(), {
      wrapper: ({ children }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>,
    });
    await waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));

    await act(async () => {
      FakeWebSocket.instances[0].onmessage?.({
        data: JSON.stringify({
          type: 'telemetry_snapshot',
          payload: { ...current, observed_at: '2026-09-20T02:59:00Z' },
        }),
      });
      await Promise.resolve();
    });

    expect(client.getQueryData(deviceTelemetryQueryKey('device-a'))).toEqual(current);
    await waitFor(() =>
      expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: queryKeys.telemetry('device-a') }),
    );
  });

  it('keeps command success separate from physical confirmation', async () => {
    listCommandsMock.mockResolvedValue([
      {
        command_id: 'cmd-1',
        device_id: 'device-a',
        action: 'on',
        pump_id: 'WATER_PUMP_IN',
        requested_state: true,
        requested_pwm: null,
        lifecycle: 'REQUESTED',
      },
    ]);
    sendMock.mockResolvedValue({
      status: 'success',
      command_id: 'cmd-1',
      lifecycle: 'CONFIRMED',
      device_id: 'device-a',
      target: 'all',
      action: 'on',
      pump: 'WATER_PUMP_IN',
      duration_sec: null,
      pwm: null,
      published_at: Date.now(),
    });

    const { result } = renderHook(() => useDeviceControl('device-a'), { wrapper });
    await waitFor(() => expect(result.current.commandStatus.WATER_PUMP_IN).toBe('REQUESTED'));
    await act(async () => {
      await result.current.togglePump('WATER_PUMP_IN', 'on');
    });

    expect(sendMock).toHaveBeenCalledOnce();
    await waitFor(() => expect(result.current.commandStatus.WATER_PUMP_IN).toBe('SENT'));

    act(() => {
      window.dispatchEvent(
        new CustomEvent('hydragrow:command-lifecycle', {
          detail: { device_id: 'device-a', command_id: 'cmd-1', lifecycle: 'CONFIRMED' },
        }),
      );
    });
    await waitFor(() => expect(result.current.commandStatus.WATER_PUMP_IN).toBe('CONFIRMED'));
  });
});

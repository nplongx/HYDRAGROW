import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useStationContext } from '../contexts/StationContext';
import { loadAppSettings } from '../platform/settings';
import { getIdToken } from '../lib/authToken';
import toast from 'react-hot-toast';
import { deviceTelemetryQueryKey, readSnapshot } from './useDeviceTelemetry';
import type { AppSettings } from '../types/models';
import { queryKeys } from '../api/queryKeys';
import { isMalformedRealtimeFrame, routeRealtimeEvent } from '../lib/realtimeRouter';

export type DeviceSyncConnectionState = 'connecting' | 'recovering' | 'connected' | 'degraded';

export interface DeviceSyncState {
  connection: DeviceSyncConnectionState;
  lastEventAt: string | null;
  lastCorrelationId: string | null;
  lastCommandId: string | null;
}

export function useDeviceSync() {
  const { selectedDeviceId: deviceId } = useStationContext();
  const queryClient = useQueryClient();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [syncState, setSyncState] = useState<DeviceSyncState>({ connection: 'degraded', lastEventAt: null, lastCorrelationId: null, lastCommandId: null });
  const recoveryInFlightRef = useRef(false);

  useEffect(() => {
    let active = true;

    const loadSettings = async () => {
      try {
        const next = await loadAppSettings();
        if (active) setSettings(next ?? null);
      } catch {
        if (active) setSettings(null);
      }
    };

    void loadSettings();
    const onUpdate = () => { void loadSettings(); };
    window.addEventListener('hydragrow:settings-updated', onUpdate);
    window.addEventListener('focus', onUpdate);
    return () => {
      active = false;
      window.removeEventListener('hydragrow:settings-updated', onUpdate);
      window.removeEventListener('focus', onUpdate);
    };
  }, []);

  // WebSocket remains a transport/orchestration adapter during migration.
  useEffect(() => {
    if (!deviceId || !settings?.backend_url) return;
    let active = true;
    let ws: WebSocket | undefined;
    let pingInterval: ReturnType<typeof setInterval> | undefined;
    let reconnectTimeout: ReturnType<typeof setTimeout> | undefined;

    const recoverAuthoritativeState = async () => {
      if (recoveryInFlightRef.current) return;
      recoveryInFlightRef.current = true;
      setSyncState((prev) => ({ ...prev, connection: 'recovering' }));
      try {
        const deviceQueries = [
          queryKeys.telemetry(deviceId), queryKeys.config(deviceId), queryKeys.analyticsHealth(deviceId),
          queryKeys.systemEvents(deviceId), queryKeys.journal(deviceId),
        ] as const;
        await Promise.all(deviceQueries.map((queryKey) => queryClient.invalidateQueries({ queryKey })));
        await Promise.all(deviceQueries.map((queryKey) => queryClient.refetchQueries({ queryKey, type: 'active' })));
        setSyncState((prev) => ({ ...prev, connection: 'connected' }));
      } catch {
        setSyncState((prev) => ({ ...prev, connection: 'degraded' }));
      } finally {
        recoveryInFlightRef.current = false;
      }
    };

    const connectWs = () => {
      const accessToken = getIdToken() || '';
      const path = `/api/devices/${deviceId}/ws?api_key=${encodeURIComponent(
        settings.api_key || ''
      )}${accessToken ? `&token=${encodeURIComponent(accessToken)}` : ''}`;
      const cleanBaseUrl = settings.backend_url.replace(/\/$/, '');
      const wsUrl = `${cleanBaseUrl.replace(/^http/, 'ws')}${path}`;

      ws = new WebSocket(wsUrl);
      setSyncState((prev) => ({ ...prev, connection: 'connecting' }));

      ws.onopen = () => {
        ws?.send(JSON.stringify({
          type: 'auth',
          api_key: settings.api_key,
          token: getIdToken(),
        }));
        pingInterval = setInterval(() => {
          if (ws?.readyState === WebSocket.OPEN) ws.send('ping');
        }, 25000);
        void recoverAuthoritativeState();
      };

      ws.onmessage = (event) => {
          try {
            if (!active) return;
            const data = JSON.parse(event.data);
            if (isMalformedRealtimeFrame(data)) {
              void recoverAuthoritativeState();
              return;
            }
            const payload = data?.payload;
          if (payload?.device_id && payload.device_id !== deviceId) return;
          setSyncState((prev) => ({
            ...prev, lastEventAt: new Date().toISOString(),
            lastCorrelationId: typeof data.correlation_id === 'string' ? data.correlation_id : typeof payload?.correlation_id === 'string' ? payload.correlation_id : prev.lastCorrelationId,
            lastCommandId: typeof data.command_id === 'string' ? data.command_id : typeof payload?.command_id === 'string' ? payload.command_id : prev.lastCommandId,
          }));

          const route = routeRealtimeEvent(data.type);
          if (route === 'telemetry') {
            if (data.type === 'telemetry_snapshot') {
              try {
                const snapshot = readSnapshot({ data: payload }, deviceId);
                if (!snapshot.observed_at) { void recoverAuthoritativeState(); return; }
                const current = queryClient.getQueryData<typeof snapshot>(deviceTelemetryQueryKey(deviceId));
                if (current?.observed_at && Date.parse(snapshot.observed_at) < Date.parse(current.observed_at)) {
                  void recoverAuthoritativeState();
                  return;
                }
                queryClient.setQueryData(deviceTelemetryQueryKey(deviceId), snapshot);
              } catch {
                void recoverAuthoritativeState();
              }
            } else {
              queryClient.invalidateQueries({ queryKey: deviceTelemetryQueryKey(deviceId) });
            }
            return;
          }

          // Do not route contact/health/FSM events through telemetry. No dedicated
          // current-state query exists for every legacy event yet, so expose a
          // narrowly scoped session event for the remaining consumers.
          if (route === 'status') {
            queryClient.invalidateQueries({ queryKey: queryKeys.analyticsHealth(deviceId) });
            queryClient.invalidateQueries({ queryKey: queryKeys.telemetry(deviceId) });
            window.dispatchEvent(
              new CustomEvent(`hydragrow:${data.type}`, { detail: data.payload }),
            );
            return;
          }

          if (route === 'command_lifecycle') {
            queryClient.invalidateQueries({ queryKey: queryKeys.controlCommands(deviceId) });
            window.dispatchEvent(
              new CustomEvent('hydragrow:command-lifecycle', { detail: data.payload }),
            );
            return;
          }

          if (route === 'alert') {
            const alert = payload;
            if (!alert) return;

            queryClient.invalidateQueries({ queryKey: queryKeys.journal(deviceId) });
            queryClient.invalidateQueries({ queryKey: queryKeys.systemEvents(deviceId) });

            if (alert.level === 'critical' || alert.level === 'warning') {
              toast.error(`${alert.title}\n${alert.message}`, {
                id: 'sys-alert',
                duration: 4000,
              });
            } else if (alert.level === 'success') {
              toast.success(`${alert.title}\n${alert.message}`, {
                id: 'sys-success',
                duration: 3000,
              });
            }
          }
        } catch {
          void recoverAuthoritativeState();
        }
      };

      ws.onclose = () => {
        if (!active) return;
        setSyncState((prev) => ({ ...prev, connection: 'degraded' }));
        if (pingInterval) clearInterval(pingInterval);
        reconnectTimeout = setTimeout(connectWs, 5000);
      };
    };

    connectWs();

    return () => {
      active = false;
      if (pingInterval) clearInterval(pingInterval);
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
      ws?.close();
    };
  }, [deviceId, settings?.backend_url, settings?.api_key, queryClient]);

  return syncState;
}

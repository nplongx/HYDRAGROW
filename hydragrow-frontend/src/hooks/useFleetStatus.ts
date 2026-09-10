import { useState, useEffect, useCallback } from 'react';
import { apiGet } from '../lib/apiClient';
import { useDeviceStore } from '../store/useDeviceStore';

export interface FleetDevice {
  device_id: string;
  label: string | null;
  is_online?: boolean;
  firmware_version?: string;
  last_seen?: string;
}

interface FleetStatusState {
  devices: FleetDevice[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useFleetStatus(): FleetStatusState {
  const [devices, setDevices] = useState<FleetDevice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Lấy danh sách device đã claimed
      const owned = await apiGet<FleetDevice[]>('/devices');

      // Lấy trạng thái online/firmware của từng device từ backend
      const enriched = await Promise.allSettled(
        owned.map(async (d) => {
          try {
            const status = await apiGet<{ is_online: boolean; firmware_version: string; last_seen: string }>(
              `/devices/${d.device_id}/status`
            );
            return { ...d, ...status };
          } catch {
            return { ...d, is_online: false };
          }
        })
      );

      setDevices(
        enriched.map((r) => (r.status === 'fulfilled' ? r.value : { device_id: 'unknown', label: null }))
      );
    } catch (e: any) {
      const isMockAuth = typeof window !== 'undefined' && (
        localStorage.getItem('mock_auth') === 'true' ||
        new URLSearchParams(window.location.search).get('mock_auth') === 'true'
      );
      if (isMockAuth) {
        const currentDeviceId = useDeviceStore.getState().deviceId || 'esp32_01';
        setDevices([
          {
            device_id: currentDeviceId,
            label: 'Trạm Thủy Canh 1',
            is_online: false,
            firmware_version: 'v1.4.2',
            last_seen: new Date().toISOString(),
          },
        ]);
        setError(null);
      } else {
        setError(e.message);
        setDevices([]);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { devices, loading, error, refresh };
}

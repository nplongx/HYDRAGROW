import { useState, useEffect, useCallback } from 'react';
import { apiGet } from '../lib/apiClient';
import { useDeviceStore } from '../store/useDeviceStore';
import type { OwnedDevice } from '../types/models';

interface UseOwnedDevicesResult {
  devices: OwnedDevice[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useOwnedDevices(): UseOwnedDevicesResult {
  const [devices, setDevices] = useState<OwnedDevice[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiGet<OwnedDevice[]>('/devices');
      setDevices(data);
    } catch (e: any) {
      const isMockAuth = typeof window !== 'undefined' && (
        localStorage.getItem('mock_auth') === 'true' ||
        new URLSearchParams(window.location.search).get('mock_auth') === 'true'
      );
      if (isMockAuth) {
        const currentDeviceId = useDeviceStore.getState().deviceId || 'esp32_01';
        setDevices([
          {
            id: 1,
            user_id: 1,
            device_id: currentDeviceId,
            label: 'Trạm Thủy Canh 1 (esp32_01)',
            claimed_at: new Date().toISOString(),
          },
        ]);
        setError(null);
      } else {
        setError(e.message ?? 'Lỗi tải danh sách thiết bị');
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { devices, loading, error, refresh };
}

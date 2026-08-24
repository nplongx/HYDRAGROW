import { useState, useEffect, useCallback } from 'react';
import { apiGet } from '../lib/apiClient';
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
      setError(e.message ?? 'Lỗi tải danh sách thiết bị');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { devices, loading, error, refresh };
}

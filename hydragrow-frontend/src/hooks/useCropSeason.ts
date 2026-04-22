// src/hooks/useCropSeason.ts
import { useState, useEffect, useCallback } from 'react';
import { fetch } from '@tauri-apps/plugin-http';
import { CropSeason } from '../types/models';
import toast from 'react-hot-toast';
import { useDeviceContext } from '../context/DeviceContext';

export const useCropSeason = () => {
  const { deviceId, settings } = useDeviceContext();
  const [activeSeason, setActiveSeason] = useState<CropSeason | null>(null);
  const [history, setHistory] = useState<CropSeason[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const getHeaders = useCallback(() => ({
    'Content-Type': 'application/json',
    'X-API-Key': settings?.api_key || ''
  }), [settings]);

  // Hàm parse an toàn để fix lỗi Tauri Stream
  const safeJsonParse = async (res: Response) => {
    const text = await res.text();
    if (!text || text.trim() === '') return null;
    try {
      return JSON.parse(text);
    } catch (e) {
      console.warn("Không thể parse JSON:", text);
      return null;
    }
  };

  const loadSeasons = useCallback(async () => {
    if (!deviceId || !settings?.backend_url) return;
    setIsLoading(true);
    try {
      const baseUrl = `${settings.backend_url}/api/devices/${deviceId}/seasons`;

      // 1. Load active season
      const activeRes = await fetch(`${baseUrl}/active`, { method: 'GET', headers: getHeaders() });
      if (activeRes.ok) {
        const activeData = await safeJsonParse(activeRes);
        setActiveSeason(activeData?.data || null);
      } else {
        console.warn(`Lỗi API Active Season: ${activeRes.status}`);
      }

      // 2. Load history
      const historyRes = await fetch(baseUrl, { method: 'GET', headers: getHeaders() });
      if (historyRes.ok) {
        const historyData = await safeJsonParse(historyRes);
        setHistory(historyData?.data || []);
      } else {
        console.warn(`Lỗi API History: ${historyRes.status}`);
      }
    } catch (error) {
      console.error("Lỗi tải dữ liệu mùa vụ:", error);
      toast.error("Không thể tải thông tin mùa vụ");
    } finally {
      setIsLoading(false);
    }
  }, [deviceId, settings, getHeaders]);

  useEffect(() => {
    loadSeasons();
  }, [loadSeasons]);

  const createSeason = async (name: string, plantType: string, description: string = '') => {
    if (!deviceId || !settings?.backend_url) return false;
    try {
      const res = await fetch(`${settings.backend_url}/api/devices/${deviceId}/seasons`, {
        method: 'POST',
        headers: getHeaders(),
        body: JSON.stringify({ name, plant_type: plantType, description })
      });
      if (res.ok) {
        toast.success("Đã bắt đầu mùa vụ mới!");
        await loadSeasons();
        return true;
      } else {
        const err = await safeJsonParse(res);
        toast.error(`Lỗi: ${err?.message || 'Không thể tạo mùa vụ'}`);
      }
    } catch (error) {
      toast.error("Lỗi mạng khi tạo mùa vụ");
    }
    return false;
  };

  // Hàm Update mùa vụscheduled_dose_a_mlAPI chưa sẵn sàng ĐANG CHẠY
  const updateSeason = async (name: string, plantType: string, description: string) => {
    if (!deviceId || !settings?.backend_url) return false;
    try {
      const res = await fetch(`${settings.backend_url}/api/devices/${deviceId}/seasons/active`, {
        method: 'PUT',
        headers: getHeaders(),
        body: JSON.stringify({ name, plant_type: plantType, description })
      });
      if (res.ok) {
        toast.success("Đã cập nhật thông tin mùa vụ!");
        await loadSeasons(); // Tải lại dữ liệu mới
        return true;
      } else {
        const err = await safeJsonParse(res);
        toast.error(`Lỗi: ${err?.message || 'Không thể cập nhật mùa vụ'}`);
      }
    } catch (error) {
      toast.error("Lỗi mạng khi cập nhật mùa vụ");
    }
    return false;
  };

  const endSeason = async () => {
    if (!deviceId || !settings?.backend_url) return false;
    try {
      const res = await fetch(`${settings.backend_url}/api/devices/${deviceId}/seasons/active/end`, {
        method: 'PUT',
        headers: getHeaders()
      });
      if (res.ok) {
        toast.success("Đã kết thúc mùa vụ hiện tại!");
        await loadSeasons();
        return true;
      } else {
        const err = await safeJsonParse(res);
        toast.error(`Lỗi: ${err?.message || 'Không thể kết thúc mùa vụ'}`);
      }
    } catch (error) {
      toast.error("Lỗi mạng khi kết thúc mùa vụ");
    }
    return false;
  };

  return { activeSeason, history, isLoading, createSeason, endSeason, refresh: loadSeasons, updateSeason };
};

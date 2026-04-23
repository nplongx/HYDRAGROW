// src/hooks/useDeviceControl.ts
import { useState, useCallback } from 'react';
import { fetch } from '@tauri-apps/plugin-http';
import { useDeviceContext } from '../context/DeviceContext';
import toast from 'react-hot-toast';

export const useDeviceControl = (deviceId: string) => {
  const { settings } = useDeviceContext();
  const [isProcessing, setIsProcessing] = useState(false);

  // Hàm gửi command chung (Đảm bảo cấu trúc chuẩn với Backend Rust)
  const sendCommand = useCallback(
    async (pump: string, action: string, duration_sec?: number, pwm?: number) => {
      if (!deviceId || !settings?.backend_url) {
        toast.error('Chưa cấu hình thiết bị hoặc máy chủ!');
        return false;
      }

      setIsProcessing(true);
      try {
        // Body chuẩn khớp với MqttCommandPayload của ESP32
        const payload = {
          pump: pump,
          action: action,
          duration_sec: duration_sec || null,
          pwm: pwm || null,
        };

        const res = await fetch(`${settings.backend_url}/api/devices/${deviceId}/control`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings.api_key || '',
          },
          body: JSON.stringify(payload),
        });

        if (res.ok) {
          toast.success(`Đã gửi lệnh ${action.toUpperCase()} đến ${pump}`);
          return true;
        } else {
          const errorText = await res.text();
          console.error(`HTTP ${res.status}:`, errorText);
          toast.error(`Từ chối lệnh: HTTP ${res.status}`);
          return false;
        }
      } catch (error: any) {
        console.error(`Lỗi thực thi lệnh (${pump}):`, error);
        toast.error('Lỗi mạng khi gửi lệnh!');
        return false;
      } finally {
        setIsProcessing(false);
      }
    },
    [deviceId, settings],
  );

  // 1. Lệnh Bật/Tắt bình thường
  const togglePump = (pumpId: string, action: 'on' | 'off') => {
    return sendCommand(pumpId, action);
  };

  // 2. Lệnh Cưỡng chế an toàn (Kèm thời gian đếm ngược)
  const forceOn = (pumpId: string, durationSec: number) => {
    return sendCommand(pumpId, 'force_on', durationSec);
  };

  // 3. Lệnh Cài đặt Công suất (PWM)
  const setPwm = (pumpId: string, pwmValue: number) => {
    return sendCommand(pumpId, 'set_pwm', undefined, pwmValue);
  };

  const resetFault = () => {
    // Truyền "ALL" làm target để Backend cho qua, ESP32 sẽ nhận và reset toàn bộ state
    return sendCommand('ALL', 'reset_fault');
  };

  return { isProcessing, togglePump, forceOn, setPwm, resetFault };
};

// src/hooks/useDeviceControl.ts
import { useState, useCallback } from 'react';
import { useDeviceContext } from '../context/DeviceContext';
import toast from 'react-hot-toast';
import { httpFetch } from '../platform/http';

const isDangerousCommand = (pumpId: string, action: string, pwm?: number) => {
  const dosingPumps = ['A', 'PUMP_A', 'B', 'PUMP_B', 'PH_UP', 'PH_DOWN'];
  return action === 'force_on'
    || action === 'reset_fault'
    || action === 'set_pwm'
    || typeof pwm === 'number'
    || dosingPumps.includes(pumpId.toUpperCase());
};


export const useDeviceControl = (deviceId: string) => {
  const { settings, refreshDeviceSnapshot } = useDeviceContext();
  const [isProcessing, setIsProcessing] = useState(false);

  // Hàm gửi command chung (Đảm bảo cấu trúc chuẩn với Backend Rust)
  const sendCommand = useCallback(async (
    pumpId: string,
    action: string,
    duration_sec?: number,
    pwm?: number
  ) => {
    if (!deviceId || !settings?.backend_url) {
      toast.error("Chưa cấu hình thiết bị hoặc máy chủ!");
      return false;
    }

    const dangerous = isDangerousCommand(pumpId, action, pwm);
    if (dangerous) {
      const confirmed = window.confirm(
        `Lệnh nguy hiểm: ${action} cho ${pumpId}. Xác nhận bạn muốn gửi lệnh điều khiển này?`
      );
      if (!confirmed) return false;
    }

    setIsProcessing(true);
    try {
      // Body chuẩn khớp với MqttCommandPayload của ESP32
      const payload = {
        target: 'all',
        action: action,
        params: {
          pump_id: pumpId,
          duration_sec: duration_sec || null,
          pwm: pwm ?? null
        },
        command_metadata: {
          action,
          pump_id: pumpId,
          duration_sec: duration_sec ?? null,
          pwm: pwm ?? null,
          dangerous
        }
      };

      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/control`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': settings.api_key || '',
          ...(dangerous ? { 'X-User-Confirmed': 'true' } : {})
        },
        body: JSON.stringify(payload)
      });

      if (res.ok) {
        const result = await res.json().catch(() => null);
        const publishedAction = result?.action || action;
        const target = result?.target || 'all';
        toast.success(`Backend đã publish MQTT: ${publishedAction} -> ${pumpId} (${target})`);

        await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/control/sync`, {
          method: 'POST',
          headers: {
            'X-API-Key': settings.api_key || ''
          }
        }).catch(err => console.error("Lỗi khi yêu cầu sync trạng thái:", err));

        await refreshDeviceSnapshot();
        setTimeout(() => {
          refreshDeviceSnapshot().catch(err => console.error("Lỗi refresh trạng thái sau lệnh:", err));
        }, 1200);

        return true;
      } else {
        const errorText = await res.text();
        console.error(`HTTP ${res.status}:`, errorText);
        toast.error(`Từ chối lệnh: HTTP ${res.status}`);
        return false;
      }
    } catch (error: any) {
      console.error(`Lỗi thực thi lệnh (${pumpId}):`, error);
      toast.error("Lỗi mạng khi gửi lệnh!");
      return false;
    } finally {
      setIsProcessing(false);
    }
  }, [deviceId, settings, refreshDeviceSnapshot]);

  // 1. Lệnh Bật/Tắt bình thường
  const togglePump = (pumpId: string, action: 'on' | 'off') => {
    return sendCommand(pumpId, action);
  };

  // 2. Lệnh Cưỡng chế an toàn (Kèm thời gian đếm ngược)
  const forceOn = (pumpId: string, durationSec: number, pwmValue?: number) => {
    return sendCommand(pumpId, 'force_on', durationSec, pwmValue);
  };

  // 3. Lệnh Cài đặt Công suất (PWM)
  const setPwm = (pumpId: string, pwmValue: number, durationSec?: number) => {
    return sendCommand(pumpId, 'set_pwm', durationSec, pwmValue);
  };

  const resetFault = () => {
    // Truyền "ALL" làm target để Backend cho qua, ESP32 sẽ nhận và reset toàn bộ state
    return sendCommand('ALL', 'reset_fault');
  };

  return { isProcessing, togglePump, forceOn, setPwm, resetFault };
};

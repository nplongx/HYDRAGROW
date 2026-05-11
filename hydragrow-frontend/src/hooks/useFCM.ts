// src/hooks/useFCM.ts
import { useEffect, useState } from 'react';
import { requestForWebToken, onWebMessageListener } from '../lib/firebase';
import { useDeviceContext } from '../context/DeviceContext';
import { httpFetch } from '../platform/http';
import toast from 'react-hot-toast';

export function useFCM(deviceId: string) {
  const { settings } = useDeviceContext();

  const [fcmToken, setFcmToken] = useState<string | null>(null);
  const [permission, setPermission] = useState(Notification.permission);

  // =========================================================
  // CHỈ GỌI KHI USER CLICK
  // =========================================================
  const enableNotifications = async () => {
    try {
      if (!settings?.backend_url || !settings?.api_key) return;

      const isWeb = !('__TAURI__' in window);

      if (!isWeb) {
        console.log("Tauri Native");
        return;
      }

      const result = await Notification.requestPermission();

      setPermission(result);

      if (result !== 'granted') {
        console.log('User từ chối notification');
        return;
      }

      const token = await requestForWebToken();

      if (!token) return;

      setFcmToken(token);

      console.log("FCM Token:", token);

      const res = await httpFetch(
        `${settings.backend_url}/api/notifications/register`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings.api_key
          },
          body: JSON.stringify({
            device_id: deviceId,
            token
          })
        }
      );

      if (res.ok) {
        console.log("Đăng ký FCM token thành công");
      } else {
        console.error(await res.text());
      }

    } catch (err) {
      console.error(err);
    }
  };

  // =========================================================
  // FOREGROUND MESSAGE
  // =========================================================
  useEffect(() => {
    const isWeb = !('__TAURI__' in window);

    if (!isWeb) return;

    const listenForMessages = async () => {
      try {
        const payload: any = await onWebMessageListener();

        console.log('Foreground message:', payload);

        if (payload?.notification) {
          toast(
            `${payload.notification.title}\n${payload.notification.body}`,
            { icon: '🔔' }
          );
        }

        listenForMessages();

      } catch (err) {
        console.error(err);
      }
    };

    listenForMessages();

  }, []);

  return {
    fcmToken,
    permission,
    enableNotifications
  };
}

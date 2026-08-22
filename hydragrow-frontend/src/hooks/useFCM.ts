// src/hooks/useFCM.ts
import { useEffect, useState } from 'react';
import { requestForWebToken, subscribeWebMessages } from '../lib/firebase';
import { useDeviceStore } from '../store/useDeviceStore';
import { httpFetch } from '../platform/http';
import { debugLog, redactSecret } from '../lib/redact';

export function useFCM() {
  // Lấy settings trực tiếp từ Zustand Store
  const settings = useDeviceStore((s) => s.settings);
  const [fcmToken, setFcmToken] = useState<string | null>(null);
  const [permission, setPermission] = useState(Notification.permission);

  const enableNotifications = async () => {
    try {
      if (!settings?.backend_url || !settings?.api_key) return;
      const isWeb = !('__TAURI__' in window);
      if (!isWeb) {
        debugLog("Tauri Native");
        return;
      }
      const result = await Notification.requestPermission();
      setPermission(result);
      if (result !== 'granted') {
        debugLog('User từ chối notification');
        return;
      }
      const token = await requestForWebToken();
      if (!token) return;
      setFcmToken(token);
      debugLog("FCM Token:", redactSecret(token));
      const res = await httpFetch(
        `${settings.backend_url}/api/notifications/register`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings.api_key
          },
          body: JSON.stringify({
            fcm_token: token
          })
        }
      );
      if (res.ok) {
        debugLog("Đăng ký FCM token thành công");
      } else {
        console.error(await res.text());
      }
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    const isWeb = !('__TAURI__' in window);
    if (!isWeb) return;

    if (Notification.permission === 'granted' && settings?.backend_url && settings?.api_key) {
      enableNotifications();
    }

    const unsubscribe = subscribeWebMessages((payload: any) => {
      debugLog('Foreground message:', payload);
      // if (payload?.notification) {
      //   const title = payload.notification.title || '';
      //   const body = payload.notification.body || '';
      //   const id = payload.messageId || `${title}:${body}`;
      //   toast(`${title}\n${body}`, { icon: '🔔', id });
      // }
    });
    return unsubscribe;
  }, [settings?.backend_url, settings?.api_key]);

  return {
    fcmToken,
    permission,
    enableNotifications
  };
}

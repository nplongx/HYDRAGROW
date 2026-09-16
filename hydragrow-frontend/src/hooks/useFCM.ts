// src/hooks/useFCM.ts
import { useEffect, useState } from 'react';
import { requestForWebToken, subscribeWebMessages } from '../lib/firebase';
import { useStationContext } from '../contexts/StationContext';
import { notificationsApi } from '../api/notifications';
import { debugLog, redactSecret } from '../lib/redact';

export function useFCM() {
  const { selectedDeviceId: deviceId } = useStationContext();
  const [fcmToken, setFcmToken] = useState<string | null>(null);
  const [permission, setPermission] = useState(Notification.permission);

  const enableNotifications = async () => {
    try {
      if (!deviceId) return;
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
      await notificationsApi.registerFcmToken(token, deviceId);
      debugLog("Đăng ký FCM token thành công");
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    const isWeb = !('__TAURI__' in window);
    if (!isWeb) return;

    if (Notification.permission === 'granted' && deviceId) {
      enableNotifications();
    }

    const unsubscribe = subscribeWebMessages((payload: any) => {
      debugLog('Foreground message:', payload);
    });
    return unsubscribe;
  }, [deviceId]);

  return {
    fcmToken,
    permission,
    enableNotifications
  };
}

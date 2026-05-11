// src/hooks/useFCM.ts
import { useEffect, useState } from 'react';
import { requestForWebToken, onWebMessageListener } from '../lib/firebase';
import { useDeviceContext } from '../context/DeviceContext';
import { httpFetch } from '../platform/http';
import toast from 'react-hot-toast'; // Hoặc thư viện toast bạn đang dùng

export function useFCM(deviceId: string) {
  const { settings } = useDeviceContext();
  const [fcmToken, setFcmToken] = useState<string | null>(null);

  useEffect(() => {
    const requestPermission = async () => {
      // Đảm bảo settings đã sẵn sàng trước khi gửi API
      if (!settings?.backend_url || !settings?.api_key) return;

      if (typeof Notification !== 'undefined') {
        let permission = Notification.permission;

        if (permission !== 'granted') {
          try {
            permission = await Notification.requestPermission();
          } catch (error) {
            console.error('Lỗi xin quyền thông báo:', error);
            return;
          }
        }

        if (permission === 'granted') {
          console.log('Quyền thông báo đã được cấp!');

          const isWeb = !('__TAURI__' in window);

          if (isWeb) {
            const token = await requestForWebToken();
            if (token) {
              setFcmToken(token);
              console.log("FCM Token Web:", token);

              // Sử dụng httpFetch đúng chuẩn (không dùng .post)
              try {
                const res = await httpFetch(`${settings.backend_url}/api/notifications/register-token`, {
                  method: 'POST',
                  headers: {
                    'Content-Type': 'application/json',
                    'X-API-Key': settings.api_key
                  },
                  body: JSON.stringify({
                    device_id: deviceId, // Truyền deviceId để backend biết token này của trạm nào
                    token: token
                  })
                });

                if (res.ok) {
                  console.log("Đã gửi FCM Token lên Backend thành công!");
                } else {
                  console.error("Gửi FCM Token thất bại:", await res.text());
                }
              } catch (err) {
                console.error("Lỗi Network khi gửi FCM Token:", err);
              }

            }
          } else {
            console.log("Đang chạy trên môi trường Tauri Native (Android/PC).");
          }
        }
      }
    };

    // Chỉ gọi hàm khi đã có settings và deviceId
    if (settings?.backend_url && deviceId) {
      requestPermission();
    }
  }, [deviceId, settings]); // Thêm dependencies để chạy lại nếu properties thay đổi

  // Hook nhận thông báo Foreground
  useEffect(() => {
    const isWeb = !('__TAURI__' in window);

    if (isWeb) {
      const listenForMessages = async () => {
        try {
          const payload: any = await onWebMessageListener();
          console.log('Nhận được thông báo Foreground: ', payload);

          // Bật popup Notification nhỏ trong App bằng toast
          if (payload?.notification) {
            toast(
              `${payload.notification.title}\n${payload.notification.body}`,
              { icon: '🔔' }
            );
          }

          listenForMessages(); // Lắng nghe tiếp
        } catch (err) {
          console.log('Lỗi khi lắng nghe thông báo: ', err);
        }
      };

      listenForMessages();
    }
  }, []);

  return { fcmToken };
}

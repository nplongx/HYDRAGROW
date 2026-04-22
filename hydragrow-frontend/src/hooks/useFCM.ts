// src/hooks/useFCM.ts
import { useEffect } from 'react';

export function useFCM() {
  useEffect(() => {
    // Chỉ cần xin quyền thông báo khi mở App để Android cho phép hiện Popup
    const requestPermission = async () => {
      if (typeof Notification !== 'undefined' && Notification.permission !== 'granted') {
        try {
          await Notification.requestPermission();
          console.log('Quyền thông báo đã được cấp!');
        } catch (error) {
          console.error('Lỗi xin quyền thông báo:', error);
        }
      }
    };

    requestPermission();
  }, []);
}

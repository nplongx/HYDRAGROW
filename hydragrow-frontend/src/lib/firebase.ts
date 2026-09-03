// src/lib/firebase.ts

import { initializeApp } from "firebase/app";
// SỬA: Import toàn bộ từ firebase/messaging thay vì rải rác
import { getMessaging, getToken, onMessage, MessagePayload } from "firebase/messaging";
import { debugLog, redactSecret } from "./redact";

// Configure Firebase with Vite environment variables from a local .env file.
// See README.md for the required VITE_FIREBASE_* and VITE_FIREBASE_VAPID_KEY values.
const firebaseConfig = {
  apiKey: import.meta.env.VITE_FIREBASE_API_KEY,
  authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN,
  projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID,
  storageBucket: import.meta.env.VITE_FIREBASE_STORAGE_BUCKET,
  messagingSenderId: import.meta.env.VITE_FIREBASE_MESSAGING_SENDER_ID,
  appId: import.meta.env.VITE_FIREBASE_APP_ID,
  measurementId: import.meta.env.VITE_FIREBASE_MEASUREMENT_ID
};


const firebaseVapidKey = import.meta.env.VITE_FIREBASE_VAPID_KEY;
export const app = initializeApp(firebaseConfig);

// Khởi tạo Messaging instance (tránh crash trong môi trường test/jsdom thiếu WindowMessaging)
export const messaging = (() => {
  try {
    return getMessaging(app);
  } catch {
    return null as unknown as ReturnType<typeof getMessaging>;
  }
})();

export const requestForWebToken = async () => {
  try {
    // Đăng ký service worker trước, rồi truyền vào getToken
    const registration = await navigator.serviceWorker.register('/firebase-messaging-sw.js');

    const currentToken = await getToken(messaging, {
      vapidKey: firebaseVapidKey,
      serviceWorkerRegistration: registration,
    });

    if (currentToken) {
      debugLog('Web FCM Token:', redactSecret(currentToken));
      return currentToken;
    }
    debugLog('Không thể lấy FCM token.');
    return null;
  } catch (err) {
    console.error('Lỗi khi lấy token:', err);
    return null;
  }
};

// Hàm lắng nghe thông báo khi Web App đang mở (Foreground)
export const onWebMessageListener = () =>
  new Promise<MessagePayload>((resolve) => {
    // SỬA: Dùng onMessage (chữ M hoa) và định nghĩa type cho payload
    onMessage(messaging, (payload: MessagePayload) => {
      resolve(payload);
    });
  });

export const subscribeWebMessages = (handler: (payload: MessagePayload) => void) => {
  return onMessage(messaging, handler);
};

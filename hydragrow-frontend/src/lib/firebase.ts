// src/lib/firebase.ts

import { initializeApp } from "firebase/app";
// SỬA: Import toàn bộ từ firebase/messaging thay vì rải rác
import { getMessaging, getToken, onMessage, MessagePayload } from "firebase/messaging";

// Configure Firebase with Vite environment variables from a local .env file.
// See README.md for the required VITE_FIREBASE_* and VITE_FIREBASE_VAPID_KEY values.
const firebaseConfig = {
  apiKey: import.meta.env.VITE_FIREBASE_API_KEY || "YOUR_FIREBASE_API_KEY",
  authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN || "YOUR_FIREBASE_AUTH_DOMAIN",
  projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID || "YOUR_FIREBASE_PROJECT_ID",
  storageBucket: import.meta.env.VITE_FIREBASE_STORAGE_BUCKET || "YOUR_FIREBASE_STORAGE_BUCKET",
  messagingSenderId: import.meta.env.VITE_FIREBASE_MESSAGING_SENDER_ID || "YOUR_FIREBASE_MESSAGING_SENDER_ID",
  appId: import.meta.env.VITE_FIREBASE_APP_ID || "YOUR_FIREBASE_APP_ID",
  measurementId: import.meta.env.VITE_FIREBASE_MEASUREMENT_ID || "YOUR_FIREBASE_MEASUREMENT_ID"
};

const firebaseVapidKey = import.meta.env.VITE_FIREBASE_VAPID_KEY || "YOUR_FIREBASE_VAPID_KEY";

const app = initializeApp(firebaseConfig);

// Khởi tạo Messaging instance
export const messaging = getMessaging(app);

export const requestForWebToken = async () => {
  try {
    // Đăng ký service worker trước, rồi truyền vào getToken
    const registration = await navigator.serviceWorker.register('/firebase-messaging-sw.js');

    const currentToken = await getToken(messaging, {
      vapidKey: firebaseVapidKey,
      serviceWorkerRegistration: registration,
    });

    if (currentToken) {
      console.log('Web FCM Token:', currentToken);
      return currentToken;
    }
    console.log('Không thể lấy FCM token.');
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

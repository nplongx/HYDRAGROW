// src/lib/firebase.ts

import { initializeApp } from "firebase/app";
// SỬA: Import toàn bộ từ firebase/messaging thay vì rải rác
import { getMessaging, getToken, onMessage, MessagePayload } from "firebase/messaging";
import { debugLog, redactSecret } from "./redact";

// Configure Firebase with Vite environment variables from a local .env file.
// See README.md for the required VITE_FIREBASE_* and VITE_FIREBASE_VAPID_KEY values.
const firebaseConfig = {
  apiKey: "AIzaSyAjxXN5YIUztbY_pSpor1xsleEvHNuZqnc",
  authDomain: "hydragrow-iot.firebaseapp.com",
  projectId: "hydragrow-iot",
  storageBucket: "hydragrow-iot.firebasestorage.app",
  messagingSenderId: "810716913891",
  appId: "1:810716913891:web:a2fea867c0d63df1bfa5d6",
  measurementId: "G-14M8B93S7V"
};


const firebaseVapidKey = "BDHacUd3ZPRTo5QfnaErWYyXIgxW2sjOR22A9HrIyLzuPrJ62cylLTgaooS3PhscRnZ6jggodBFmd3hJ3izr33I";
export const app = initializeApp(firebaseConfig);

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

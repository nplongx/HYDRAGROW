// src/lib/firebase.ts
import { initializeApp } from 'firebase/app';
import { getMessaging, getToken, onMessage } from 'firebase/messaging';

// Thay thế bằng Firebase Config thực tế từ Firebase Console của bạn
const firebaseConfig = {
  apiKey: "AIzaSyAjxXN5YIUztbY_pSpor1xsleEvHNuZqnc",
  authDomain: "hydragrow-iot.firebaseapp.com",
  projectId: "hydragrow-iot",
  storageBucket: "hydragrow-iot.firebasestorage.app",
  messagingSenderId: "810716913891",
  appId: "1:810716913891:web:a2fea867c0d63df1bfa5d6",
  measurementId: "G-14M8B93S7V"
};


const app = initializeApp(firebaseConfig);

// Khởi tạo Messaging instance
export const messaging = getMessaging(app);

export const requestForWebToken = async () => {
  try {
    // Đăng ký service worker trước, rồi truyền vào getToken
    const registration = await navigator.serviceWorker.register('/firebase-messaging-sw.js');

    const currentToken = await getToken(messaging, {
      vapidKey: 'BDHacUd3ZPRTo5QfnaErWYyXIgxW2sjOR22A9HrIyLzuPrJ62cylLTgaooS3PhscRnZ6jggodBFmd3hJ3izr33I',
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
  new Promise((resolve) => {
    onMessage(messaging, (payload) => {
      resolve(payload);
    });
  });

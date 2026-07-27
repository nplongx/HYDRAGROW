/* global importScripts, firebase, self */

importScripts('https://www.gstatic.com/firebasejs/10.12.2/firebase-app-compat.js');
importScripts('https://www.gstatic.com/firebasejs/10.12.2/firebase-messaging-compat.js');

const firebaseConfig = {
  apiKey: 'YOUR_FIREBASE_API_KEY',
  authDomain: 'YOUR_FIREBASE_AUTH_DOMAIN',
  projectId: 'YOUR_FIREBASE_PROJECT_ID',
  storageBucket: 'YOUR_FIREBASE_STORAGE_BUCKET',
  messagingSenderId: 'YOUR_FIREBASE_MESSAGING_SENDER_ID',
  appId: 'YOUR_FIREBASE_APP_ID',
  measurementId: 'YOUR_FIREBASE_MEASUREMENT_ID',
};

firebase.initializeApp(firebaseConfig);

const messaging = firebase.messaging();

messaging.onBackgroundMessage((payload) => {
  if (self.location.hostname === 'localhost') {
    console.log('Đã nhận thông báo ngầm:', payload);
  }

  const notificationTitle = payload?.notification?.title || 'HydraGrow';
  const notificationOptions = {
    body: payload?.notification?.body || 'Bạn có thông báo mới.',
    icon: '/tauri.svg',
  };

  self.registration.showNotification(notificationTitle, notificationOptions);
});

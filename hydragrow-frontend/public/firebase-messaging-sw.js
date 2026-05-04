/* global importScripts, firebase */

importScripts('https://www.gstatic.com/firebasejs/10.12.2/firebase-app-compat.js');
importScripts('https://www.gstatic.com/firebasejs/10.12.2/firebase-messaging-compat.js');

const firebaseConfig = {
  apiKey: 'AIzaSyAjxXN5YIUztbY_pSpor1xsleEvHNuZqnc',
  authDomain: 'hydragrow-iot.firebaseapp.com',
  projectId: 'hydragrow-iot',
  storageBucket: 'hydragrow-iot.firebasestorage.app',
  messagingSenderId: '810716913891',
  appId: '1:810716913891:web:a2fea867c0d63df1bfa5d6',
  measurementId: 'G-14M8B93S7V',
};

firebase.initializeApp(firebaseConfig);

const messaging = firebase.messaging();

messaging.onBackgroundMessage((payload) => {
  console.log('Đã nhận thông báo ngầm:', payload);

  const notificationTitle = payload?.notification?.title || 'HydraGrow';
  const notificationOptions = {
    body: payload?.notification?.body || 'Bạn có thông báo mới.',
    icon: '/tauri.svg',
  };

  self.registration.showNotification(notificationTitle, notificationOptions);
});

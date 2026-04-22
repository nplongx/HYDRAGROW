const firebaseConfig = {
  apiKey: "AIzaSyAjxXN5YIUztbY_pSpor1xsleEvHNuZqnc",
  authDomain: "hydragrow-iot.firebaseapp.com",
  projectId: "hydragrow-iot",
  storageBucket: "hydragrow-iot.firebasestorage.app",
  messagingSenderId: "810716913891",
  appId: "1:810716913891:web:a2fea867c0d63df1bfa5d6",
  measurementId: "G-14M8B93S7V"
};

export const messaging = firebaseConfig.messaging();

messaging.onBackgroundMessage((payload) => {
  console.log('Đã nhận thông báo ngầm:', payload);
  const notificationTitle = payload.notification.title;
  const notificationOptions = {
    body: payload.notification.body,
    icon: '/tauri.svg' // Đường dẫn icon app của bạn
  };

  self.registration.showNotification(notificationTitle, notificationOptions);
});

import { initializeApp } from 'firebase/app';
import { getMessaging, getToken, onMessage, type MessagePayload, type Messaging } from 'firebase/messaging';
import { debugLog, redactSecret } from './redact';

type FirebaseConfig = {
  apiKey?: string;
  authDomain?: string;
  projectId?: string;
  storageBucket?: string;
  messagingSenderId?: string;
  appId?: string;
  measurementId?: string;
};

const firebaseConfig: FirebaseConfig = {
  apiKey: import.meta.env.VITE_FIREBASE_API_KEY,
  authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN,
  projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID,
  storageBucket: import.meta.env.VITE_FIREBASE_STORAGE_BUCKET,
  messagingSenderId: import.meta.env.VITE_FIREBASE_MESSAGING_SENDER_ID,
  appId: import.meta.env.VITE_FIREBASE_APP_ID,
  measurementId: import.meta.env.VITE_FIREBASE_MEASUREMENT_ID,
};

const firebaseVapidKey = import.meta.env.VITE_FIREBASE_VAPID_KEY;

export function hasRequiredFirebaseConfig(config: FirebaseConfig): boolean {
  return Boolean(
    config.apiKey?.trim() &&
      config.authDomain?.trim() &&
      config.projectId?.trim() &&
      config.messagingSenderId?.trim() &&
      config.appId?.trim(),
  );
}

export const app = initializeApp(firebaseConfig);

let messaging: Messaging | null = null;

function getMessagingInstance(): Messaging | null {
  if (messaging) return messaging;
  if (typeof window === 'undefined' || !hasRequiredFirebaseConfig(firebaseConfig)) return null;

  try {
    messaging = getMessaging(app);
    return messaging;
  } catch (error) {
    console.error('[v0] Firebase Messaging unavailable:', error);
    return null;
  }
}

export const requestForWebToken = async (): Promise<string | null> => {
  const messagingInstance = getMessagingInstance();
  if (!messagingInstance || !firebaseVapidKey || !('serviceWorker' in navigator)) return null;

  try {
    const registration = await navigator.serviceWorker.register('/firebase-messaging-sw.js');
    const currentToken = await getToken(messagingInstance, {
      vapidKey: firebaseVapidKey,
      serviceWorkerRegistration: registration,
    });

    if (currentToken) {
      debugLog('Web FCM Token:', redactSecret(currentToken));
      return currentToken;
    }
    debugLog('Không thể lấy FCM token.');
  } catch (error) {
    console.error('[v0] Lỗi khi lấy token:', error);
  }

  return null;
};

export const onWebMessageListener = (): Promise<MessagePayload> =>
  new Promise((resolve) => {
    const messagingInstance = getMessagingInstance();
    if (!messagingInstance) return;
    onMessage(messagingInstance, resolve);
  });

export const subscribeWebMessages = (handler: (payload: MessagePayload) => void): (() => void) => {
  const messagingInstance = getMessagingInstance();
  return messagingInstance ? onMessage(messagingInstance, handler) : () => undefined;
};

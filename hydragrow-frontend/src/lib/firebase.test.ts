import { describe, expect, it } from 'vitest';
import { hasRequiredFirebaseConfig, hasValidFirebaseApiKey } from './firebase';

describe('Firebase configuration', () => {
  it('requires the Firebase project id before enabling messaging', () => {
    expect(
      hasRequiredFirebaseConfig({
        apiKey: 'api-key',
        authDomain: 'example.firebaseapp.com',
        projectId: '',
        messagingSenderId: 'sender-id',
        appId: 'app-id',
      }),
    ).toBe(false);
  });

  it('accepts the minimum configuration required by Firebase Messaging', () => {
    expect(
      hasRequiredFirebaseConfig({
        apiKey: 'AIzaSyA123456789012345678901234567890',
        authDomain: 'example.firebaseapp.com',
        projectId: 'project-id',
        messagingSenderId: 'sender-id',
        appId: 'app-id',
      }),
    ).toBe(true);
  });

  it('rejects placeholder and malformed Firebase API keys', () => {
    expect(hasValidFirebaseApiKey('YOUR_FIREBASE_API_KEY')).toBe(false);
    expect(hasValidFirebaseApiKey('not-a-firebase-key')).toBe(false);
    expect(hasValidFirebaseApiKey('AIzaSyA123456789012345678901234567890')).toBe(true);
  });
});

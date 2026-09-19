import { describe, expect, it } from 'vitest';
import { hasRequiredFirebaseConfig } from './firebase';

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
        apiKey: 'api-key',
        authDomain: 'example.firebaseapp.com',
        projectId: 'project-id',
        messagingSenderId: 'sender-id',
        appId: 'app-id',
      }),
    ).toBe(true);
  });
});

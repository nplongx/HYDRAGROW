import { vi, describe, it, expect } from 'vitest';

// First, mock the dependencies before they are imported
vi.mock('firebase/app', () => {
  return {
    initializeApp: vi.fn().mockReturnValue({}),
  };
});

vi.mock('firebase/messaging', () => {
  return {
    getMessaging: vi.fn().mockReturnValue({}),
    getToken: vi.fn().mockResolvedValue('test-token'),
    onMessage: vi.fn(),
  };
});

describe('Firebase Configuration Security', () => {
  it('should initialize firebase without embedding a Firebase config value in source', async () => {
    // Import the firebase config indirectly by checking how initializeApp was called.
    // Firebase Web API keys are public configuration values; the security requirement
    // here is that the app reads its config from Vite environment variables rather
    // than embedding a project-specific value in source code.
    const firebaseApp = await import('firebase/app');
    await import('../src/lib/firebase');

    const initializeAppMock = vi.mocked(firebaseApp.initializeApp);
    const callArgs = initializeAppMock.mock.calls[0][0];

    expect(callArgs.apiKey).not.toBe(
      'AIzaSyAjxXN5YIUztbY_pSpor1xsleEvHNuZqnc',
    );

    // The Vite environment is intentionally not populated by this unit test.
    // firebase.ts reads VITE_FIREBASE_* through import.meta.env instead.
    expect(callArgs).toHaveProperty('apiKey');
    expect(callArgs).toHaveProperty('authDomain');
    expect(callArgs).toHaveProperty('projectId');
    expect(callArgs).toHaveProperty('appId');
  });
});

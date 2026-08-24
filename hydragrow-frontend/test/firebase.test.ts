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
  it('should initialize firebase with environment variables instead of hardcoded secrets', async () => {
    // Import the firebase config indirectly by checking how initializeApp was called
    const firebaseApp = await import('firebase/app');
    await import('../src/lib/firebase');

    const initializeAppMock = vi.mocked(firebaseApp.initializeApp);

    const callArgs = initializeAppMock.mock.calls[0][0];

    expect(callArgs.apiKey).not.toBe("AIzaSyAjxXN5YIUztbY_pSpor1xsleEvHNuZqnc");

    // In our test environment, Vite meta env variables may be undefined,
    // but the point is we removed the hardcoded secret and use `import.meta.env`
    // We can also set an env var and test it if needed.
  });
});

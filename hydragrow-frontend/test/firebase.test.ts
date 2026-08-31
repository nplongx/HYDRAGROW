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
  it('should initialize firebase with the expected public web configuration', async () => {
  const firebaseApp = await import('firebase/app');
  await import('../src/lib/firebase');

  const initializeAppMock = vi.mocked(firebaseApp.initializeApp);
  expect(initializeAppMock).toHaveBeenCalledTimes(1);

  const callArgs = initializeAppMock.mock.calls[0][0];

  expect(callArgs).toHaveProperty('apiKey');
  expect(callArgs).toHaveProperty('authDomain');
  expect(callArgs).toHaveProperty('projectId');
  expect(callArgs).toHaveProperty('appId');

  expect(callArgs.apiKey).toMatch(/^AIza/);
  expect(callArgs.authDomain).toBeTruthy();
  expect(callArgs.projectId).toBeTruthy();
  expect(callArgs.appId).toBeTruthy();
});
});

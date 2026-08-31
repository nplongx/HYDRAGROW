content = ""
with open('hydragrow-frontend/test/firebase.test.ts', 'r') as f:
    content = f.read()

# We need to set up the mocked import.meta.env values in vitest globals.
# Alternatively we can just mock them before import or use vi.stubEnv.

# Looking at the test file:
#   expect(callArgs.apiKey).toMatch(/^AIza/);
# It's checking that the apiKey is defined and starts with AIza. Since it's undefined, we need to stub the env vars for the test.

new_content = """import { vi, describe, it, expect, beforeEach } from 'vitest';

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
  beforeEach(() => {
    vi.stubEnv('VITE_FIREBASE_API_KEY', 'AIzaSyA_test_key_mocked');
    vi.stubEnv('VITE_FIREBASE_AUTH_DOMAIN', 'test-auth-domain');
    vi.stubEnv('VITE_FIREBASE_PROJECT_ID', 'test-project-id');
    vi.stubEnv('VITE_FIREBASE_APP_ID', 'test-app-id');
  });

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
"""

with open('hydragrow-frontend/test/firebase.test.ts', 'w') as f:
    f.write(new_content)

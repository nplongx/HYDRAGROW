import '@testing-library/jest-dom';
import { vi } from 'vitest';

vi.mock('firebase/app', () => ({
  initializeApp: vi.fn().mockReturnValue({}),
}));

vi.mock('firebase/auth', () => ({
  getAuth: vi.fn().mockReturnValue({}),
  signInWithEmailAndPassword: vi.fn(),
  signOut: vi.fn(),
  onAuthStateChanged: vi.fn((_auth, callback) => {
    callback(null);
    return () => {};
  }),
  onIdTokenChanged: vi.fn((_auth, callback) => {
    callback(null);
    return () => {};
  }),
}));

vi.mock('firebase/messaging', () => ({
  getMessaging: vi.fn().mockReturnValue({}),
  getToken: vi.fn().mockResolvedValue('test-token'),
  onMessage: vi.fn(),
  isSupported: vi.fn().mockResolvedValue(false),
}));

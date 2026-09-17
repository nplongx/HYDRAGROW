import { describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';

const authStateCallback = vi.fn();
const idTokenCallback = vi.fn();

vi.mock('../lib/firebaseAuth', () => ({
  subscribeAuthState: (callback: (user: unknown) => void) => {
    authStateCallback.mockImplementation(callback);
    callback({ uid: 'firebase-user' });
    return () => {};
  },
  subscribeIdToken: (callback: (token: string | null) => void) => {
    idTokenCallback.mockImplementation(callback);
    return () => {};
  },
  loginWithEmailPassword: vi.fn(),
  registerWithEmailPassword: vi.fn(),
  signInWithGoogle: vi.fn(),
  sendPasswordReset: vi.fn(),
  logout: vi.fn(),
  describeAuthError: vi.fn(() => 'error'),
  authErrorField: vi.fn(() => 0),
}));

vi.mock('../lib/authToken', () => ({
  setIdToken: vi.fn(),
}));

import { AuthProvider, useAuth } from './AuthContext';

function Probe() {
  const { status } = useAuth();
  return <div data-testid="status">{status}</div>;
}

describe('AuthProvider', () => {
  it('does not publish authenticated before the Firebase ID token is ready', async () => {
    render(
      <AuthProvider>
        <Probe />
      </AuthProvider>,
    );

    expect(screen.getByTestId('status')).toHaveTextContent('loading');

    act(() => {
      idTokenCallback('firebase-id-token');
    });

    await waitFor(() => {
      expect(screen.getByTestId('status')).toHaveTextContent('authenticated');
    });
  });

});

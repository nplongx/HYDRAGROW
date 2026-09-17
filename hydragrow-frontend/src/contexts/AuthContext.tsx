// src/contexts/AuthContext.tsx
import React, { createContext, useContext, useEffect, useMemo, useState, useCallback } from 'react';
import type { User } from 'firebase/auth';
import {
  loginWithEmailPassword,
  registerWithEmailPassword,
  signInWithGoogle,
  sendPasswordReset,
  logout as firebaseLogout,
  subscribeAuthState,
  subscribeIdToken,
  describeAuthError,
  authErrorField,
} from '../lib/firebaseAuth';
import { setIdToken } from '../lib/authToken';

type AuthStatus = 'loading' | 'authenticated' | 'unauthenticated';

interface AuthContextValue {
  status: AuthStatus;
  user: User | null;
  error: string | null;
  errorCode: string | null;
  errorField: 0 | 1 | 2;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string) => Promise<void>;
  googleLogin: () => Promise<void>;
  resetPassword: (email: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>('loading');
  const [user, setUser] = useState<User | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [errorCode, setErrorCode] = useState<string | null>(null);

  useEffect(() => {
    const unsubscribeAuthState = subscribeAuthState((nextUser) => {
      setUser(nextUser);
      // Do not expose an authenticated state to API consumers until the
      // Firebase ID token callback has populated the in-memory token. This
      // prevents the first API request after a page reload from falling back
      // to the API-key path with an empty key.
      if (!nextUser) {
        setIdToken(null);
        setStatus('unauthenticated');
      } else {
        setStatus('loading');
      }
    });

    const unsubscribeIdToken = subscribeIdToken((token) => {
      setIdToken(token);
      setStatus(token ? 'authenticated' : 'unauthenticated');
    });

    return () => {
      unsubscribeAuthState();
      unsubscribeIdToken();
    };
  }, []);

  const runWithError = useCallback(async (action: () => Promise<unknown>) => {
    setError(null);
    setErrorCode(null);
    try {
      await action();
    } catch (err: any) {
      const code = typeof err?.code === 'string' ? err.code : 'unknown';
      setError(describeAuthError(code));
      setErrorCode(code);
      throw err;
    }
  }, []);

  const login = useCallback((email: string, password: string) => {
    return runWithError(() => loginWithEmailPassword(email, password));
  }, [runWithError]);

  const register = useCallback((email: string, password: string) => {
    return runWithError(() => registerWithEmailPassword(email, password));
  }, [runWithError]);

  const googleLogin = useCallback(() => {
    return runWithError(() => signInWithGoogle());
  }, [runWithError]);

  const resetPassword = useCallback((email: string) => {
    return runWithError(() => sendPasswordReset(email));
  }, [runWithError]);

  const logout = useCallback(async () => {
    await firebaseLogout();
  }, []);

  const errorField = errorCode ? authErrorField(errorCode) : 0;

  const value = useMemo<AuthContextValue>(
    () => ({ status, user, error, errorCode, errorField, login, register, googleLogin, resetPassword, logout }),
    [status, user, error, errorCode, errorField, login, register, googleLogin, resetPassword, logout]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth phải được gọi bên trong <AuthProvider>');
  }
  return context;
}

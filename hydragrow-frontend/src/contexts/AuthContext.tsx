// src/contexts/AuthContext.tsx
import React, { createContext, useContext, useEffect, useMemo, useState, useCallback } from 'react';
import type { User } from 'firebase/auth';
import {
  loginWithEmailPassword,
  logout as firebaseLogout,
  subscribeAuthState,
  subscribeIdToken,
  describeAuthError,
} from '../lib/firebaseAuth';
import { setIdToken } from '../lib/authToken';

type AuthStatus = 'loading' | 'authenticated' | 'unauthenticated';

interface AuthContextValue {
  status: AuthStatus;
  user: User | null;
  error: string | null;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>('loading');
  const [user, setUser] = useState<User | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unsubscribeAuthState = subscribeAuthState((nextUser) => {
      setUser(nextUser);
      setStatus(nextUser ? 'authenticated' : 'unauthenticated');
    });

    const unsubscribeIdToken = subscribeIdToken((token) => {
      setIdToken(token);
    });

    return () => {
      unsubscribeAuthState();
      unsubscribeIdToken();
    };
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    setError(null);
    try {
      await loginWithEmailPassword(email, password);
    } catch (err: any) {
      const code = typeof err?.code === 'string' ? err.code : 'unknown';
      setError(describeAuthError(code));
      throw err;
    }
  }, []);

  const logout = useCallback(async () => {
    await firebaseLogout();
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({ status, user, error, login, logout }),
    [status, user, error, login, logout]
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

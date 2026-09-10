// src/components/auth/LoginScreen.tsx
import React, { useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';
import { AuthCard, AuthTextField, AuthSubmitButton, GoogleIcon } from './AuthCard';

interface LoginScreenProps {
  onShowRegister?: () => void;
  onShowForgot?: () => void;
}

export function LoginScreen({ onShowRegister, onShowForgot }: LoginScreenProps) {
  const { login, googleLogin, error, errorField } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      await login(email, password);
    } catch {
      // error đã được đưa vào AuthContext, hiển thị bên dưới form
    } finally {
      setSubmitting(false);
    }
  };

  const handleGoogle = async () => {
    setSubmitting(true);
    try {
      await googleLogin();
    } catch {
      // error đã được đưa vào AuthContext
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <AuthCard subtitle="Đăng nhập để điều khiển hệ thống của bạn">
      <form className="login-screen-card space-y-4" onSubmit={handleSubmit} noValidate>
        <AuthTextField
          id="login-email"
          label="Email"
          type="email"
          value={email}
          onChange={setEmail}
          autoComplete="username"
          invalid={errorField === 1}
        />

        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <label className="text-sm font-semibold text-primary-deep" htmlFor="login-password">
              Mật khẩu
            </label>
            {onShowForgot && (
              <button
                type="button"
                onClick={onShowForgot}
                className="text-xs font-semibold text-primary hover:text-primary-deep"
              >
                Quên mật khẩu?
              </button>
            )}
          </div>
          <input
            id="login-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            required
            aria-invalid={errorField === 2}
            className={`w-full rounded-xl border px-4 py-2.5 text-sm text-primary-deep outline-none transition-colors ${
              errorField === 2
                ? 'border-red-500 ring-1 ring-red-200 focus:border-red-500'
                : 'border-line bg-white focus:border-primary'
            }`}
          />
        </div>

        {error && <p className="login-screen-error text-sm text-red-600 font-medium" role="alert">{error}</p>}

        <AuthSubmitButton submitting={submitting}>
          {submitting ? 'Đang đăng nhập...' : 'Đăng nhập'}
        </AuthSubmitButton>

        <div className="flex items-center gap-3">
          <div className="h-px flex-1 bg-line" />
          <span className="text-xs text-faint">hoặc</span>
          <div className="h-px flex-1 bg-line" />
        </div>

        <button
          type="button"
          onClick={handleGoogle}
          disabled={submitting}
          className="w-full flex items-center justify-center gap-2 rounded-xl border border-line bg-white px-4 py-3 text-sm font-semibold text-primary-deep hover:bg-soft disabled:opacity-60 transition-colors"
        >
          <GoogleIcon />
          Đăng nhập bằng Google
        </button>
      </form>

      {onShowRegister && (
        <div className="mt-4 text-center text-sm text-text-muted">
          Chưa có tài khoản?{' '}
          <button onClick={onShowRegister} className="font-bold text-primary hover:text-primary-deep">
            Đăng ký ngay
          </button>
        </div>
      )}
    </AuthCard>
  );
}
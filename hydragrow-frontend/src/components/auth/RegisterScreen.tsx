// src/components/auth/RegisterScreen.tsx
// Tự đăng ký tài khoản mới. Sandbox backend tự cấp scope đọc mặc định
// (read:telemetry) ở lần truy cập đầu tiên sau khi token được xác minh.

import React, { useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';
import { AuthCard, AuthTextField, AuthSubmitButton, GoogleIcon } from './AuthCard';

interface RegisterScreenProps {
  onShowLogin: () => void;
}

export function RegisterScreen({ onShowLogin }: RegisterScreenProps) {
  const { register, googleLogin, error, errorField } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLocalError(null);
    if (password.length < 6) {
      setLocalError('Mật khẩu phải có ít nhất 6 ký tự.');
      return;
    }
    if (password !== confirm) {
      setLocalError('Mật khẩu xác nhận không khớp.');
      return;
    }
    setSubmitting(true);
    try {
      await register(email, password);
    } catch {
      // error đã được đưa vào AuthContext
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
    <AuthCard
      subtitle="Tạo tài khoản để ghép nối và theo dõi thiết bị"
    >
      <form className="login-screen-card space-y-4" onSubmit={handleSubmit} noValidate>
        <AuthTextField
          id="register-email"
          label="Email"
          type="email"
          value={email}
          onChange={setEmail}
          autoComplete="username"
          invalid={errorField === 1}
        />

        <AuthTextField
          id="register-password"
          label="Mật khẩu"
          type="password"
          value={password}
          onChange={setPassword}
          autoComplete="new-password"
          invalid={errorField === 2}
        />

        <AuthTextField
          id="register-confirm"
          label="Xác nhận mật khẩu"
          type="password"
          value={confirm}
          onChange={setConfirm}
          autoComplete="new-password"
          invalid={Boolean(localError?.includes('xác nhận'))}
        />

        {(error || localError) && (
          <p className="login-screen-error text-sm text-red-600 font-medium" role="alert">
            {localError ?? error}
          </p>
        )}

        <AuthSubmitButton submitting={submitting}>
          {submitting ? 'Đang tạo tài khoản...' : 'Đăng ký'}
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
          Đăng ký bằng Google
        </button>
      </form>

      <div className="mt-4 text-center text-sm text-text-muted">
        Đã có tài khoản?{' '}
        <button onClick={onShowLogin} className="font-bold text-primary hover:text-primary-deep">
          Đăng nhập
        </button>
      </div>
    </AuthCard>
  );
}
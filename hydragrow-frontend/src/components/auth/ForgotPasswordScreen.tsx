// src/components/auth/ForgotPasswordScreen.tsx
// Gửi email đặt lại mật khẩu qua Firebase Auth.

import React, { useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';
import { AuthCard, AuthTextField, AuthSubmitButton } from './AuthCard';
import { ArrowLeft } from 'lucide-react';

interface ForgotPasswordScreenProps {
  onShowLogin: () => void;
}

export function ForgotPasswordScreen({ onShowLogin }: ForgotPasswordScreenProps) {
  const { resetPassword, error, errorField } = useAuth();
  const [email, setEmail] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [sent, setSent] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      await resetPassword(email);
      setSent(true);
    } catch {
      // error đã được đưa vào AuthContext
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <AuthCard subtitle="Chúng tôi sẽ gửi liên kết đặt lại mật khẩu qua email">
      {sent ? (
        <div className="text-center space-y-4">
          <div className="p-4 bg-pill text-status rounded-xl text-sm font-medium">
            Đã gửi email đặt lại mật khẩu. Kiểm tra hộp thư của bạn và làm theo hướng dẫn.
          </div>
          <button
            onClick={onShowLogin}
            className="w-full flex items-center justify-center gap-2 rounded-xl bg-primary-deep px-4 py-3 text-sm font-bold text-white hover:bg-primary transition-colors"
          >
            <ArrowLeft size={16} />
            Quay lại đăng nhập
          </button>
        </div>
      ) : (
        <form className="login-screen-card space-y-4" onSubmit={handleSubmit} noValidate>
          <AuthTextField
            id="forgot-email"
            label="Email"
            type="email"
            value={email}
            onChange={setEmail}
            autoComplete="username"
            invalid={errorField === 1}
          />

          {error && <p className="login-screen-error text-sm text-red-600 font-medium" role="alert">{error}</p>}

          <AuthSubmitButton submitting={submitting}>
            {submitting ? 'Đang gửi...' : 'Gửi email đặt lại mật khẩu'}
          </AuthSubmitButton>

          <button
            type="button"
            onClick={onShowLogin}
            className="w-full flex items-center justify-center gap-2 text-sm font-semibold text-primary hover:text-primary-deep"
          >
            <ArrowLeft size={14} />
            Quay lại đăng nhập
          </button>
        </form>
      )}
    </AuthCard>
  );
}
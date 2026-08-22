// src/components/auth/LoginScreen.tsx
import React, { useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';

export function LoginScreen() {
  const { login, error } = useAuth();
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

  return (
    <div className="login-screen">
      <form className="login-screen-card" onSubmit={handleSubmit}>
        <h1 className="login-screen-title">HydraGrow</h1>
        <p className="login-screen-subtitle">Đăng nhập bằng tài khoản đã được cấp</p>

        <label className="login-screen-label" htmlFor="login-email">Email</label>
        <input
          id="login-email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoComplete="username"
          required
        />

        <label className="login-screen-label" htmlFor="login-password">Mật khẩu</label>
        <input
          id="login-password"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
          required
        />

        {error && <p className="login-screen-error" role="alert">{error}</p>}

        <button type="submit" disabled={submitting}>
          {submitting ? 'Đang đăng nhập...' : 'Đăng nhập'}
        </button>

        <p className="login-screen-hint">
          Chưa có tài khoản? Liên hệ quản trị viên hệ thống để được cấp.
        </p>
      </form>
    </div>
  );
}

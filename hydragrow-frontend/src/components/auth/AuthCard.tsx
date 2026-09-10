// src/components/auth/AuthCard.tsx
// Vỏ khung chung cho các màn hình xác thực (đăng nhập / đăng ký / quên mật khẩu).

import React from 'react';

interface AuthCardProps {
  subtitle: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
}

export function AuthCard({ subtitle, children, footer }: AuthCardProps) {
  return (
    <div className="login-screen min-h-screen flex items-center justify-center p-4 bg-surface-muted">
      <div className="w-full max-w-sm ui-card bg-white border border-line rounded-2xl p-8 shadow-sm">
        <div className="text-center mb-6">
          <h1 className="login-screen-title text-3xl font-bold text-primary-deep tracking-tight">
            HydraGrow
          </h1>
          <p className="text-sm text-text-muted mt-2">{subtitle}</p>
        </div>
        {children}
        {footer && (
          <div className="mt-6 pt-4 border-t border-line text-sm text-text-muted text-center">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

interface AuthTextFieldProps {
  id: string;
  label: string;
  type?: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete?: string;
  invalid?: boolean;
  placeholder?: string;
  required?: boolean;
}

export function AuthTextField({
  id,
  label,
  type = 'text',
  value,
  onChange,
  autoComplete,
  invalid = false,
  placeholder,
  required = true,
}: AuthTextFieldProps) {
  return (
    <div className="space-y-1.5">
      <label className="block text-sm font-semibold text-primary-deep" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        autoComplete={autoComplete}
        placeholder={placeholder}
        required={required}
        aria-invalid={invalid}
        className={`w-full rounded-xl border px-4 py-2.5 text-sm text-primary-deep outline-none transition-colors ${
          invalid
            ? 'border-red-500 ring-1 ring-red-200 focus:border-red-500'
            : 'border-line bg-white focus:border-primary'
        }`}
      />
    </div>
  );
}

export function AuthSubmitButton({
  submitting,
  children,
}: {
  submitting: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="submit"
      disabled={submitting}
      className="w-full flex items-center justify-center gap-2 rounded-xl bg-primary-deep px-4 py-3 text-sm font-bold text-white disabled:opacity-60 hover:bg-primary transition-colors"
    >
      {children}
    </button>
  );
}

export function GoogleIcon({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 48 48" aria-hidden="true">
      <path
        fill="#EA4335"
        d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"
      />
      <path
        fill="#4285F4"
        d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z"
      />
      <path
        fill="#FBBC05"
        d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z"
      />
      <path
        fill="#34A853"
        d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"
      />
    </svg>
  );
}
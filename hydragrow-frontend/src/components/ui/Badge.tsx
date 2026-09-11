import React from 'react';

export type BadgeTone = 'success' | 'warning' | 'danger' | 'info' | 'neutral';

interface BadgeProps {
  tone?: BadgeTone;
  dot?: boolean;
  title?: string;
  className?: string;
  children: React.ReactNode;
}

const TONES: Record<BadgeTone, string> = {
  success: 'bg-success-bg text-status border-transparent',
  warning: 'bg-warning-bg text-warn-deep border-transparent',
  danger: 'bg-danger-bg text-error border-transparent',
  info: 'bg-info-bg text-info-fg border-transparent',
  neutral: 'bg-surface-muted text-text-muted border-transparent',
};

const DOT_COLORS: Record<BadgeTone, string> = {
  success: 'bg-status',
  warning: 'bg-warn-deep',
  danger: 'bg-error',
  info: 'bg-info-fg',
  neutral: 'bg-text-muted',
};

export const Badge: React.FC<BadgeProps> = ({ tone = 'neutral', dot = false, title, className, children }) => (
  <span
    title={title}
    className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[10px] font-bold ${TONES[tone]} ${className ?? ''}`}
  >
    {dot && <span aria-hidden="true" className={`h-1.5 w-1.5 rounded-full ${DOT_COLORS[tone]}`} />}
    {children}
  </span>
);
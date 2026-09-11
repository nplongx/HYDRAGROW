import React from 'react';
import { AlertCircle, AlertTriangle, Info } from 'lucide-react';

export type BannerTone = 'info' | 'warning' | 'danger';

interface BannerProps {
  tone?: BannerTone;
  title: React.ReactNode;
  children?: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
}

const META: Record<BannerTone, { panel: string; text: string; description: string; Icon: typeof Info }> = {
  info: {
    panel: 'bg-info-bg border-info-fg/25',
    text: 'text-info-fg',
    description: 'text-info-fg/80',
    Icon: Info,
  },
  warning: {
    panel: 'bg-warning-bg border-warn-deep/25',
    text: 'text-warn-deep',
    description: 'text-warn-deep/80',
    Icon: AlertTriangle,
  },
  danger: {
    panel: 'bg-danger-bg border-error/25',
    text: 'text-error',
    description: 'text-error/80',
    Icon: AlertCircle,
  },
};

export const Banner: React.FC<BannerProps> = ({ tone = 'info', title, children, action, className }) => {
  const meta = META[tone];
  const { Icon } = meta;
  return (
    <div role="alert" className={`flex items-start gap-3 rounded-xl border px-4 py-3 ${meta.panel} ${className ?? ''}`}>
      <Icon size={18} aria-hidden="true" className={`mt-0.5 shrink-0 ${meta.text}`} />
      <div className="min-w-0 flex-1">
        <p className={`text-sm font-bold ${meta.text}`}>{title}</p>
        {children && <p className={`mt-0.5 text-xs leading-relaxed ${meta.description}`}>{children}</p>}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
};
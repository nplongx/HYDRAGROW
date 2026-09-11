import React from 'react';
import { LucideIcon } from 'lucide-react';

interface SensorBentoCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  icon: LucideIcon | React.ElementType;
  theme: 'blue' | 'fuchsia' | 'orange' | 'cyan' | 'rose' | 'emerald';
  statusLabel?: string;
  statusTone?: 'good' | 'warn' | 'danger' | 'info';
  rangeLabel?: string;
  description?: string;
  compact?: boolean;
  sparkline?: number;
}

const themeClasses: Record<string, string> = {
  blue: 'text-sky-700 bg-sky-50 border-sky-100',
  fuchsia: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-100',
  orange: 'text-orange-700 bg-orange-50 border-orange-100',
  cyan: 'text-cyan-700 bg-cyan-50 border-cyan-100',
  rose: 'text-rose-700 bg-rose-50 border-rose-100',
  emerald: 'text-emerald-700 bg-emerald-50 border-emerald-100',
};

const statusClasses: Record<string, string> = {
  good: 'bg-pill text-status border-transparent',
  warn: 'bg-[#FFFBEB] text-warn-deep border-transparent',
  danger: 'bg-[#FEE2E2] text-error border-transparent',
  info: 'bg-sky-50 text-sky-700 border-transparent',
};

const sparkColor: Record<string, string> = {
  good: 'bg-status',
  warn: 'bg-warn-deep',
  danger: 'bg-error',
  info: 'bg-primary/50',
};

const clampPercent = (value: number) => Math.min(100, Math.max(0, value));

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({
  title, value, unit, icon: Icon, theme, statusLabel, statusTone = 'info',
  rangeLabel, description, compact = false, sparkline,
}) => (
  <div className={`bg-white border rounded-[18px] flex flex-col justify-between transition-all hover:border-primary/40 hover:shadow-md shadow-sm ${compact ? 'p-3.5 min-h-[140px]' : 'p-4 md:p-5 min-h-[176px]'} ${statusTone === 'danger' ? 'border-red-200 bg-red-50/30' : statusTone === 'warn' ? 'border-amber-200 bg-amber-50/20' : 'border-line'}`}>
    <div className="flex items-start justify-between gap-2">
      <div className="flex items-center gap-2.5">
        <div className={`p-2 rounded-xl border ${themeClasses[theme]} shrink-0`}>
          <Icon size={compact ? 15 : 17} strokeWidth={2.5} />
        </div>
        <span className={`font-semibold text-primary-deep ${compact ? 'text-xs' : 'text-sm'}`}>{title}</span>
      </div>
      {statusLabel && (
        <span className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-bold ${statusClasses[statusTone]}`}>
          {statusLabel}
        </span>
      )}
    </div>
    <div className={`space-y-1 ${compact ? 'mt-3' : 'mt-4'}`}>
      <div className="flex items-baseline gap-1">
        <span className={`font-black text-primary-deep ${compact ? 'text-2xl' : 'text-3xl'}`}>{value ?? '--'}</span>
        {unit && <span className={`font-semibold text-primary/70 ${compact ? 'text-xs' : 'text-sm'}`}>{unit}</span>}
      </div>
      {rangeLabel && <p className="text-[11px] font-medium text-faint">{rangeLabel}</p>}
      {description && <p className="text-xs text-faint/80 leading-relaxed">{description}</p>}
    </div>
    {sparkline != null && (
      <div className="mt-3">
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-line">
          <div
            className={`h-full rounded-full transition-all duration-500 ${sparkColor[statusTone] ?? sparkColor.info}`}
            style={{ width: `${clampPercent(sparkline)}%` }}
          />
        </div>
      </div>
    )}
  </div>
);

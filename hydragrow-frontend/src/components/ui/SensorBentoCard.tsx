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
}

const themeClasses: Record<string, string> = {
  blue: 'text-blue-700 bg-blue-50 border-blue-100',
  fuchsia: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-100',
  orange: 'text-orange-700 bg-orange-50 border-orange-100',
  cyan: 'text-cyan-700 bg-cyan-50 border-cyan-100',
  rose: 'text-rose-700 bg-rose-50 border-rose-100',
  emerald: 'text-emerald-700 bg-emerald-50 border-emerald-100',
};

const statusClasses: Record<string, string> = {
  good: 'bg-emerald-50 text-emerald-700 border-emerald-200',
  warn: 'bg-amber-50 text-amber-700 border-amber-200',
  danger: 'bg-red-50 text-red-600 border-red-200',
  info: 'bg-sky-50 text-sky-700 border-sky-200',
};

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({
  title, value, unit, icon: Icon, theme, statusLabel, statusTone = 'info',
  rangeLabel, description, compact = false,
}) => (
  <div className={`bg-white border rounded-2xl flex flex-col justify-between transition-all hover:border-emerald-200 hover:shadow-md shadow-sm ${compact ? 'p-3.5 min-h-[140px]' : 'p-4 md:p-5 min-h-[176px]'} ${statusTone === 'danger' ? 'border-red-200 bg-red-50/30' : statusTone === 'warn' ? 'border-amber-200 bg-amber-50/20' : 'border-emerald-100'}`}>
    <div className="flex items-start justify-between gap-2">
      <div className="flex items-center gap-2.5">
        <div className={`p-2 rounded-xl border ${themeClasses[theme]} shrink-0`}>
          <Icon size={compact ? 15 : 17} strokeWidth={2.5} />
        </div>
        <span className={`font-semibold text-emerald-900 ${compact ? 'text-xs' : 'text-sm'}`}>{title}</span>
      </div>
      {statusLabel && (
        <span className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-bold ${statusClasses[statusTone]}`}>
          {statusLabel}
        </span>
      )}
    </div>
    <div className={`space-y-1 ${compact ? 'mt-3' : 'mt-4'}`}>
      <div className="flex items-baseline gap-1">
        <span className={`font-black text-emerald-950 ${compact ? 'text-2xl' : 'text-3xl'}`}>{value ?? '--'}</span>
        {unit && <span className={`font-semibold text-emerald-600/80 ${compact ? 'text-xs' : 'text-sm'}`}>{unit}</span>}
      </div>
      {rangeLabel && <p className="text-[11px] font-medium text-emerald-700/60">{rangeLabel}</p>}
      {description && <p className="text-xs text-emerald-700/50 leading-relaxed">{description}</p>}
    </div>
  </div>
);

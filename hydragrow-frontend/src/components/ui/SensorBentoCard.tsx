import React from 'react';

interface SensorBentoCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  icon: React.ElementType;
  theme: 'blue' | 'fuchsia' | 'orange' | 'cyan' | 'rose';
  statusLabel?: string;
  statusTone?: 'good' | 'warn' | 'danger' | 'info';
  rangeLabel?: string;
  description?: string;
}

const themeClasses = {
  blue: 'text-blue-700 bg-blue-50 border-blue-100',
  fuchsia: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-100',
  orange: 'text-orange-700 bg-orange-50 border-orange-100',
  cyan: 'text-cyan-700 bg-cyan-50 border-cyan-100',
  rose: 'text-rose-700 bg-rose-50 border-rose-100',
};

const statusClasses = {
  good: 'bg-emerald-50 text-emerald-700 border-emerald-200',
  warn: 'bg-amber-50 text-amber-800 border-amber-200',
  danger: 'bg-red-50 text-red-700 border-red-200',
  info: 'bg-sky-50 text-sky-700 border-sky-200',
};

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({
  title,
  value,
  unit,
  icon: Icon,
  theme,
  statusLabel,
  statusTone = 'info',
  rangeLabel,
  description,
}) => {
  const iconTheme = themeClasses[theme];

  return (
    <div className="bg-white border border-emerald-100 rounded-xl p-5 flex flex-col justify-between min-h-[180px] transition-colors hover:border-emerald-300 shadow-sm shadow-emerald-950/5">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-3 text-emerald-900">
          <div className={`p-2 rounded-lg border ${iconTheme}`}>
            <Icon size={18} strokeWidth={2.5} />
          </div>
          <span className="font-semibold text-sm">{title}</span>
        </div>
        {statusLabel && (
          <span className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-bold ${statusClasses[statusTone]}`}>
            {statusLabel}
          </span>
        )}
      </div>
      <div className="mt-4 space-y-2">
        <div className="flex items-baseline gap-1">
          <span className="text-3xl font-bold text-emerald-950">{value ?? '--'}</span>
          {unit && <span className="text-sm font-semibold text-emerald-700/80">{unit}</span>}
        </div>
        {rangeLabel && <p className="text-xs font-medium text-emerald-800/75">{rangeLabel}</p>}
        {description && <p className="text-xs text-emerald-700/70 leading-relaxed">{description}</p>}
      </div>
    </div>
  );
};

import React from 'react';

interface SensorBentoCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  icon: React.ElementType;
  theme: 'blue' | 'fuchsia' | 'orange' | 'cyan' | 'rose';
}

const themeClasses = {
  blue: 'text-blue-500 bg-blue-500/10',
  fuchsia: 'text-fuchsia-500 bg-fuchsia-500/10',
  orange: 'text-orange-500 bg-orange-500/10',
  cyan: 'text-cyan-500 bg-cyan-500/10',
  rose: 'text-rose-500 bg-rose-500/10',
};

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({ title, value, unit, icon: Icon, theme }) => {
  const iconTheme = themeClasses[theme];

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl p-5 flex flex-col justify-between aspect-[4/3] transition-colors hover:border-slate-700">
      <div className="flex items-center gap-3 text-slate-400">
        <div className={`p-2 rounded-lg ${iconTheme}`}>
          <Icon size={18} strokeWidth={2.5} />
        </div>
        <span className="font-medium text-sm">{title}</span>
      </div>
      <div className="mt-4 flex items-baseline gap-1">
        <span className="text-3xl font-semibold text-slate-100">{value ?? '--'}</span>
        {unit && <span className="text-sm font-medium text-slate-500">{unit}</span>}
      </div>
    </div>
  );
};

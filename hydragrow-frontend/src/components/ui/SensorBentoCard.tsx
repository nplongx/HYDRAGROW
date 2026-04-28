interface SensorBentoCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  icon: React.ElementType;
  theme: 'blue' | 'fuchsia' | 'orange' | 'cyan' | 'rose';
}

const themeClasses = {
  blue: { text: 'text-slate-300', accent: 'border-slate-700' },
  fuchsia: { text: 'text-slate-300', accent: 'border-slate-700' },
  orange: { text: 'text-slate-300', accent: 'border-slate-700' },
  cyan: { text: 'text-slate-300', accent: 'border-slate-700' },
  rose: { text: 'text-rose-300', accent: 'border-rose-500/40' },
};

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({ title, value, unit, icon: Icon, theme }) => {
  const styles = themeClasses[theme];

  return (
    <div className={`bg-slate-900 border ${styles.accent} rounded-2xl p-5 flex flex-col justify-between aspect-square`}>
      <div className={`flex items-center space-x-2 ${styles.text}`}>
        <Icon size={18} />
        <span className="font-semibold text-sm">{title}</span>
      </div>
      <div className="mt-4">
        <span className="text-3xl font-semibold text-white">{value ?? '--'}</span>
        {unit && <span className="text-slate-400 ml-1 text-sm">{unit}</span>}
      </div>
    </div>
  );
};

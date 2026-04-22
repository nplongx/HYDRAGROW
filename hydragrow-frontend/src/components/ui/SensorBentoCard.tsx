interface SensorBentoCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  icon: React.ElementType;
  theme: 'blue' | 'fuchsia' | 'orange' | 'cyan' | 'rose';
}

const themeClasses = {
  blue: { text: 'text-blue-400', glow: 'bg-blue-500/10' },
  fuchsia: { text: 'text-fuchsia-400', glow: 'bg-fuchsia-500/10' },
  orange: { text: 'text-orange-400', glow: 'bg-orange-500/10' },
  cyan: { text: 'text-cyan-400', glow: 'bg-cyan-500/10' },
  rose: { text: 'text-rose-400', glow: 'bg-rose-500/10' }, // Đã thêm style cho màu rose
};

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({ title, value, unit, icon: Icon, theme }) => {
  const styles = themeClasses[theme];

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-3xl p-5 flex flex-col justify-between aspect-square relative overflow-hidden">
      <div className={`flex items-center space-x-2 relative z-10 ${styles.text}`}>
        <Icon size={20} />
        <span className="font-semibold text-sm">{title}</span>
      </div>
      <div className="relative z-10 mt-4">
        <span className="text-4xl font-bold text-white">{value ?? '--'}</span>
        {unit && <span className="text-slate-400 ml-1">{unit}</span>}
      </div>
      {/* Hiệu ứng phát sáng góc dưới */}
      <div className={`absolute -bottom-10 -right-10 w-32 h-32 rounded-full blur-2xl ${styles.glow}`}></div>
    </div>
  );
};

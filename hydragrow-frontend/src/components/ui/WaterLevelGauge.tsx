import React from 'react';

interface WaterLevelGaugeProps {
  min: number;
  target: number;
  max: number;
  unit?: string;
}

const clampPercent = (value: number, min: number, max: number): number => {
  if (max <= min) return 0;
  const pct = ((value - min) / (max - min)) * 100;
  return Math.min(100, Math.max(0, pct));
};

export const WaterLevelGauge: React.FC<WaterLevelGaugeProps> = ({ min, target, max, unit = '%' }) => {
  const fillPercent = clampPercent(target, min, max);

  return (
    <div className="space-y-2">
      <div className="h-2.5 w-full rounded-full bg-emerald-100 overflow-hidden">
        <div
          data-testid="water-level-gauge-fill"
          className="h-full rounded-full bg-emerald-600 transition-all"
          style={{ width: `${fillPercent}%` }}
        />
      </div>
      <div className="flex items-center justify-between text-[11px] font-semibold text-emerald-800/75">
        <span>Min {min}{unit}</span>
        <span className="text-emerald-900">Mục tiêu {target}{unit}</span>
        <span>Max {max}{unit}</span>
      </div>
    </div>
  );
};

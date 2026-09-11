import React from 'react';

export type HealthTone = 'great' | 'attention' | 'critical' | 'offline';

interface HealthScoreProps {
  score: number | null;
  label?: string;
  max?: number;
  className?: string;
}

const TONE_META: Record<HealthTone, { segment: string; text: string }> = {
  great: { segment: 'bg-success', text: 'text-status' },
  attention: { segment: 'bg-warn-deep', text: 'text-warn-deep' },
  critical: { segment: 'bg-error', text: 'text-error' },
  offline: { segment: 'bg-line', text: 'text-faint' },
};

const toneOf = (score: number | null): HealthTone => {
  if (score === null) return 'offline';
  if (score >= 80) return 'great';
  if (score >= 50) return 'attention';
  return 'critical';
};

const LABELS: Record<HealthTone, string> = {
  great: 'Tốt',
  attention: 'Cần chú ý',
  critical: 'Nghiêm trọng',
  offline: 'Ngoại tuyến',
};

const SEGMENTS = 5;

export const HealthScore: React.FC<HealthScoreProps> = ({ score, label, max = 100, className }) => {
  const tone = toneOf(score);
  const meta = TONE_META[tone];
  const filled = score === null ? 0 : Math.round((Math.min(Math.max(score, 0), max) / max) * SEGMENTS);

  return (
    <div className={`flex items-center gap-3 ${className ?? ''}`}>
      <div className="flex items-baseline gap-0.5">
        <span className={`text-2xl font-black ${meta.text}`}>{score ?? '--'}</span>
        <span className="text-xs font-semibold text-faint">/{max}</span>
      </div>
      <div className="flex items-center gap-1" aria-label={label}>
        {Array.from({ length: SEGMENTS }).map((_, index) => (
          <span
            key={index}
            aria-hidden="true"
            className={`h-2.5 w-1.5 rounded-full ${index < filled ? meta.segment : 'bg-line'}`}
          />
        ))}
      </div>
      <span className="text-[10px] font-bold text-text-muted">{label ?? LABELS[tone]}</span>
    </div>
  );
};
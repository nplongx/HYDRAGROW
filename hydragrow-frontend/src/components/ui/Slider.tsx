import React from 'react';

interface SliderProps {
  value: number;
  onChange: (value: number) => void;
  onCommit?: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  id?: string;
  ariaLabel?: string;
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

export const Slider: React.FC<SliderProps> = ({
  value,
  onChange,
  onCommit,
  min = 0,
  max = 100,
  step = 1,
  disabled = false,
  id,
  ariaLabel,
}) => {
  const safeMin = Math.min(min, max);
  const safeMax = Math.max(min, max);
  const current = clamp(value, safeMin, safeMax);
  const percent = safeMax === safeMin ? 0 : ((current - safeMin) / (safeMax - safeMin)) * 100;

  const commit = () => {
    onCommit?.(current);
  };

  return (
    <input
      id={id}
      type="range"
      aria-label={ariaLabel}
      min={safeMin}
      max={safeMax}
      step={step}
      value={current}
      disabled={disabled}
      onChange={(e) => onChange(Number(e.target.value))}
      onMouseUp={commit}
      onTouchEnd={commit}
      onKeyUp={commit}
      className="app-slider"
      style={{ background: `linear-gradient(to right, var(--color-primary) ${percent}%, var(--color-line) ${percent}%)` }}
    />
  );
};
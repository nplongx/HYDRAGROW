import React from 'react';

interface InputGroupProps {
  label: string;
  type?: string;
  value: string | number;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  step?: string;
  desc?: string;
  min?: number;
  max?: number;
  errorText?: string;
}

export const InputGroup: React.FC<InputGroupProps> = ({
  label, type = "number", value, onChange, step, desc, min, max, errorText
}) => (
  <div className="flex flex-col gap-1">
    <label className="text-sm font-semibold text-emerald-950">
      {label}
    </label>
    <input
      type={type}
      step={step}
      min={min}
      max={max}
      value={value}
      onChange={onChange}
      className={`w-full bg-white text-emerald-950 text-sm rounded-lg p-2.5 outline-none transition-colors border disabled:opacity-50 disabled:cursor-not-allowed
        ${errorText
          ? 'border-red-300 focus:border-red-600 focus:ring-2 focus:ring-red-500/20'
          : 'border-emerald-200 focus:border-emerald-600 focus:ring-2 focus:ring-emerald-500/20 hover:border-emerald-400'
        }
      `}
    />
    {desc && <span className="text-xs text-emerald-700/75 mt-0.5 leading-relaxed">{desc}</span>}
    {errorText && <span className="text-xs font-medium text-red-700 mt-0.5">{errorText}</span>}
  </div>
);

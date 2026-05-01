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
    <label className="text-sm font-medium text-slate-300">
      {label}
    </label>
    <input
      type={type}
      step={step}
      min={min}
      max={max}
      value={value}
      onChange={onChange}
      className={`w-full bg-slate-950 text-slate-100 text-sm rounded-lg p-2.5 outline-none transition-colors border
        ${errorText
          ? 'border-red-500/50 focus:border-red-500 focus:ring-1 focus:ring-red-500'
          : 'border-slate-800 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 hover:border-slate-700'
        }
      `}
    />
    {desc && <span className="text-xs text-slate-500 mt-0.5 leading-relaxed">{desc}</span>}
    {errorText && <span className="text-xs text-red-400 mt-0.5">{errorText}</span>}
  </div>
);

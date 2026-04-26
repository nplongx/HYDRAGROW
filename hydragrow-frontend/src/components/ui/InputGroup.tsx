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
  label,
  type = "number",
  value,
  onChange,
  step,
  desc,
  min,
  max,
  errorText
}) => (
  <div className="ui-form-row group">
    <label className="ui-form-label group-focus-within:text-emerald-400 transition-colors">
      {label}
    </label>
    <input
      type={type}
      step={step}
      min={min}
      max={max}
      value={value}
      onChange={onChange}
      className={`ui-input ${errorText
          ? 'border-rose-500/70 focus:ring-rose-500/30 focus:border-rose-500'
          : 'border-slate-800 focus:ring-emerald-500/30 focus:border-emerald-500/50 hover:border-slate-700'
        }`}
    />
    {desc && <span className="ui-helper-text">{desc}</span>}
    {errorText && <span className="text-[11px] text-rose-400 pl-1 leading-tight font-medium">{errorText}</span>}
  </div>
);

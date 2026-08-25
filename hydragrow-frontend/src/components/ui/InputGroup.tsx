import React from 'react';

interface InputGroupProps {
  label: string;
  unit?: string;
  helperText?: string;
  desc?: string;
  error?: string;
  errorText?: string;
  children?: React.ReactNode;
  // Input props for direct use when children is omitted
  type?: string;
  value?: string | number;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  step?: string;
  min?: number;
  max?: number;
  disabled?: boolean;
}

export const InputGroup: React.FC<InputGroupProps> = ({
  label, unit, helperText, desc, error, errorText, children,
  type = 'number', value, onChange, step, min, max, disabled
}) => {
  const displayHelper = helperText || desc;
  const displayError = error || errorText;

  return (
    <div className="ui-form-row flex flex-col gap-1">
      <label className="ui-form-label text-sm font-semibold text-emerald-950">
        {label}
        {unit && <span className="ml-1 font-normal text-emerald-700/50">({unit})</span>}
      </label>
      {children ? children : (
        <input
          type={type}
          step={step}
          min={min}
          max={max}
          value={value ?? ''}
          onChange={onChange}
          disabled={disabled}
          className={`w-full bg-white text-emerald-950 text-sm rounded-lg p-2.5 outline-none transition-colors border disabled:opacity-50 disabled:cursor-not-allowed ${
            displayError
              ? 'border-red-300 focus:border-red-600 focus:ring-2 focus:ring-red-500/20'
              : 'border-emerald-200 focus:border-emerald-600 focus:ring-2 focus:ring-emerald-500/20 hover:border-emerald-400'
          }`}
        />
      )}
      {displayHelper && !displayError && <p className="ui-helper-text text-xs text-emerald-700/75 mt-0.5 leading-relaxed">{displayHelper}</p>}
      {displayError && <p className="text-[11px] font-medium text-red-600 mt-1">{displayError}</p>}
    </div>
  );
};

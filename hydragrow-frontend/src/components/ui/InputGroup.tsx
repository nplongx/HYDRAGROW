import React from 'react';

interface InputGroupProps {
  label: string;
  unit?: string;
  helperText?: string;
  error?: string;
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
  label,
  unit,
  helperText,
  error,
  children,
  type = 'number',
  value,
  onChange,
  step,
  min,
  max,
  disabled,
}) => {
  return (
    <div className="ui-form-row flex flex-col gap-1">
      <label className="ui-form-label text-sm font-semibold text-emerald-950">
        {label}
        {unit && (
          <span className="ml-1 font-normal text-emerald-700/50">
            ({unit})
          </span>
        )}
      </label>

      {children ? (
        children
      ) : (
        <input
          type={type}
          step={step}
          min={min}
          max={max}
          value={value ?? ''}
          onChange={onChange}
          disabled={disabled}
          className={`w-full rounded-lg border bg-white p-2.5 text-sm text-emerald-950 outline-none transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${
            error
              ? 'border-red-300 focus:border-red-600 focus:ring-2 focus:ring-red-500/20'
              : 'border-emerald-200 hover:border-emerald-400 focus:border-emerald-600 focus:ring-2 focus:ring-emerald-500/20'
          }`}
        />
      )}

      {helperText && !error && (
        <p className="ui-helper-text mt-0.5 text-xs leading-relaxed text-emerald-700/75">
          {helperText}
        </p>
      )}

      {error && (
        <p className="mt-1 text-[11px] font-medium text-red-600">
          {error}
        </p>
      )}
    </div>
  );
};
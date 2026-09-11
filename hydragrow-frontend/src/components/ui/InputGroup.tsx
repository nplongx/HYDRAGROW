import React from 'react';

interface InputGroupProps {
  label: string;
  id?: string;
  name?: string;
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
  placeholder?: string;
}

export const InputGroup: React.FC<InputGroupProps> = ({
  label, id, name, unit, helperText, desc, error, errorText, children,
  type = 'number', value, onChange, step, min, max, disabled, placeholder
}) => {
  const generatedId = React.useId();
  const inputId = id || generatedId;
  const errorId = `${inputId}-error`;
  const helperId = `${inputId}-helper`;

  const displayHelper = helperText || desc;
  const displayError = error || errorText;

  let normalizedValue = value ?? '';
  if (type === 'number' && typeof value === 'number' && Number.isFinite(value)) {
    if (step && step.includes('.')) {
      const decimals = step.split('.')[1].length;
      normalizedValue = Number(value.toFixed(Math.min(Math.max(decimals, 2), 4)));
    } else {
      normalizedValue = Number(value.toFixed(4));
    }
  }

  return (
    <div className="ui-form-row flex flex-col gap-1">
      <label htmlFor={inputId} className="ui-form-label text-sm font-semibold text-primary-deep cursor-pointer">
        {label}
        {unit && <span className="ml-1 font-normal text-text-muted">({unit})</span>}
      </label>
      {children ? children : (
        <input
          id={inputId}
          name={name || inputId}
          type={type}
          step={step}
          min={min}
          max={max}
          value={normalizedValue}
          onChange={onChange}
          disabled={disabled}
          placeholder={placeholder}
          aria-invalid={Boolean(displayError)}
          aria-describedby={displayError ? errorId : displayHelper ? helperId : undefined}
          className={`w-full bg-white text-primary-deep text-sm rounded-lg p-2.5 outline-none transition-colors border disabled:opacity-50 disabled:cursor-not-allowed ${
            displayError
              ? 'border-red-300 focus:border-red-600 focus:ring-2 focus:ring-red-500/20'
              : 'border-line focus:border-primary focus:ring-2 focus:ring-primary/20 hover:border-primary/40'
          }`}
        />
      )}
      {displayHelper && !displayError && <p id={helperId} className="ui-helper-text text-xs text-text-muted mt-0.5 leading-relaxed">{displayHelper}</p>}
      {displayError && <p id={errorId} className="text-[11px] font-medium text-red-600 mt-1" role="alert">{displayError}</p>}
    </div>
  );
};

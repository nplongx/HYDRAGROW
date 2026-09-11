import React from 'react';
import { ChevronDown } from 'lucide-react';

export interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  hint?: string;
  error?: string;
  options: SelectOption[];
  placeholder?: string;
}

export const Select: React.FC<SelectProps> = ({
  label,
  hint,
  error,
  id,
  options,
  placeholder,
  className,
  ...rest
}) => {
  const selectId = id ?? (label ? `${label}-select`.replace(/\s+/g, '-').toLowerCase() : undefined);
  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label htmlFor={selectId} className="ui-form-label">
          {label}
        </label>
      )}
      <div className="relative">
        <select
          id={selectId}
          aria-invalid={error ? true : undefined}
          className={`ui-input appearance-none pr-10 ${error ? 'border-error focus:ring-error/30 focus:border-error' : ''} ${className ?? ''}`}
          {...rest}
        >
          {placeholder && (
            <option value="" disabled>
              {placeholder}
            </option>
          )}
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <ChevronDown
          size={16}
          aria-hidden="true"
          className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 text-faint"
        />
      </div>
      {error ? (
        <p className="text-[11px] text-error pl-1 leading-tight" role="alert">
          {error}
        </p>
      ) : hint ? (
        <p className="ui-helper-text">{hint}</p>
      ) : null}
    </div>
  );
};